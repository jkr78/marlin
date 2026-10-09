# marlin-py — usage guide

Five patterns covered below: sync streaming, asyncio integration,
per-protocol filtering, context managers, and dataclass serialization.
Examples use real bytes (files, sockets, captured logs).

For install steps, see `README.md`. For exact signatures, read the type
stubs in `python/marlin/*/__init__.pyi`.

---

## 1. Streaming with the iterator protocol

Feed bytes in chunks. Iterate for completed sentences. The parser
reassembles sentences split across `feed()` calls, so chunk boundaries
don't matter.

```python
from marlin.envelope import StreamingParser

parser = StreamingParser()

with open("capture.nmea", "rb") as f:
    while chunk := f.read(4096):
        parser.feed(chunk)
        for sentence in parser:
            talker = sentence.talker.decode() if sentence.talker else ""
            print(talker, sentence.sentence_type, sentence.checksum_ok)
```

`for ... in parser` drains every sentence framed since the last drain.
Call `feed()` to push more bytes, then iterate again. One parser handles
the whole input source.

---

## 2. Asyncio integration

`marlin.aio` provides three helpers that turn each parser type into an
async iterator over an `asyncio.StreamReader`:

```python
import asyncio
from marlin.aio import aiter_sentences

async def consume(host: str, port: int) -> None:
    reader, _ = await asyncio.open_connection(host, port)
    async for sentence in aiter_sentences(reader):
        handle(sentence)

asyncio.run(consume("ais.example.com", 4001))
```

Use `aiter_nmea_messages` for typed NMEA output and `aiter_ais_messages`
for typed AIS. All three accept an optional `parser=` keyword if you
need to inject a pre-configured parser (custom `DecodeOptions`, manual
AIS clock, etc.).

Each `feed()` call inside the helper costs microseconds, so the event
loop yields on `await reader.read()` between chunks. Other coroutines
run on schedule.

For full control over the read loop, drive the sync parser directly:

```python
parser = StreamingParser()
while chunk := await reader.read(4096):
    parser.feed(chunk)
    for sentence in parser:
        handle(sentence)
```

If you ever feed multi-megabyte chunks where parsing shows up on a
profiler, wrap the call with `await asyncio.to_thread(parser.feed,
chunk)`. Typical TCP feeds of NMEA or AIS run at kbit/s and never reach
that point.

---

## 3. Choose the right parser

marlin ships three parsers at different abstraction levels. Pick by the
kind of message you want to consume.

### NMEA only, typed messages

```python
from marlin.nmea import Nmea0183Parser, Gga, Vtg, Hdt

parser = Nmea0183Parser.streaming()

while chunk := source.read(4096):
    parser.feed(chunk)
    for msg in parser:
        if isinstance(msg, Gga):
            print(f"fix: {msg.latitude_deg.value}, {msg.longitude_deg.value}")
        elif isinstance(msg, Vtg):
            print(f"speed: {msg.speed_knots.value} kn")
        elif isinstance(msg, Hdt):
            print(f"heading: {msg.heading_true_deg.value}°")
```

`Nmea0183Parser` decodes GGA, GLL, HDG, HDT, RMC, TLL, TTM, VTG, PSXN,
and PRDID into typed classes. Anything it doesn't recognize, AIVDM included, surfaces
as `Unknown`. Match `Unknown` to forward those sentences elsewhere, or
skip it if you only care about typed messages.

Every decoded field of a message is a `marlin.field.FieldState`.
`.value` is `None` unless the field holds a value. To tell not
available (the sender left the field empty) from invalid (the decoder
could not read it), `match` on the variant class:

```python
from marlin.field import FieldState

match msg.latitude_deg:
    case FieldState.Value(lat):
        print(f"lat {lat}")
    case FieldState.NotAvailable():
        print("no position")
    case FieldState.Invalid(code):
        print(f"unreadable latitude ({code})")
    case _:
        pass
```

A field state is always truthy, `FieldState.NotAvailable()` included:
`if msg.latitude_deg:` never branches. Test `.value`, or the state.
A bad field never raises; `DecodeError` means only that the sentence
has fewer fields than its decoder's floor.

For a specific PSXN or PRDID hardware dialect, pass `DecodeOptions`:

```python
from marlin.nmea import Nmea0183Parser, DecodeOptions, PrdidDialect

opts = DecodeOptions().with_prdid_dialect(PrdidDialect.PITCH_ROLL_HEADING)
parser = Nmea0183Parser.streaming(options=opts)
```

### AIS only, typed AisMessage

```python
from marlin.ais import AisParser, PositionReportA, StaticAndVoyageA

parser = AisParser.streaming()

while chunk := source.read(4096):
    parser.feed(chunk)
    for msg in parser:
        body = msg.body
        if isinstance(body, PositionReportA):
            print(f"MMSI {body.mmsi}: {body.latitude_deg.value}, {body.longitude_deg.value}")
        elif isinstance(body, StaticAndVoyageA):
            print(f"MMSI {body.mmsi}: {body.vessel_name.value}")
```

AIS adds the over-range state: a speed of 102.2 kn or more is
`FieldState.AtLeast(102.2)`, and `.value` is `None` for it. `match` to
keep the bound, and read a status-carrying field through its own
variant classes:

```python
from marlin.ais import RateOfTurn

match body.speed_over_ground:
    case FieldState.Value(sog):
        print(f"{sog} kn")
    case FieldState.AtLeast(bound):
        print(f"{bound} kn or more")
    case _:
        pass

match body.rate_of_turn:
    case FieldState.Value(RateOfTurn.DegPerMin(rate)):
        print(f"{rate} deg/min")
    case FieldState.Value(RateOfTurn.NoIndicator(direction)):
        print(f"turning {direction}, no indicator")
    case _:
        pass
```

`AisParser` filters non-AIS sentences out, so you only see decoded AIS
messages. It also handles multi-fragment reassembly: a Type 5 split
across two `!AIVDM` lines arrives as a single `AisMessage`.

The `body` attribute holds the typed payload. `msg.type_tag` is a string
like `"type1"`, `"type5"`, or `"type21"` for filtering without
`isinstance`.

For deterministic replay or tests that patch `time`, set `clock="manual"`
and drive the clock yourself:

```python
parser = AisParser.streaming(timeout_ms=60_000, clock="manual")

for now_ms, chunk in replay_log():
    parser.feed(chunk)
    parser.tick(now_ms=now_ms)
    for msg in parser:
        ...
```

In this mode the binding makes zero `time.monotonic_ns()` calls, which
is why a test that patches `time` still gets reproducible
fragment-timeout behavior.

### Both at once

If your stream mixes NMEA position updates with AIVDM reports, common on
combined GPS + AIS receivers, run two parsers against the same bytes:

```python
from marlin.nmea import Nmea0183Parser
from marlin.ais import AisParser

nmea = Nmea0183Parser.streaming()
ais = AisParser.streaming()

while chunk := source.read(4096):
    nmea.feed(chunk)
    ais.feed(chunk)
    for msg in nmea:
        ...   # typed NMEA, AIVDM filtered out
    for msg in ais:
        ...   # decoded AIS, NMEA filtered out
```

Each parser owns its own buffer. Feeding the same bytes to both is fine.

### KLV: single-shot encode/decode

`marlin.klv` has no streaming parser — MISB ST 0601 local sets aren't
framed inside a byte stream the way NMEA/AIS sentences are, so `decode`
and `encode` work on one complete datagram at a time:

```python
from marlin.klv import St0601, decode, encode

from marlin.field import FieldState

s = St0601(timestamp_us=1_700_000_000_000_000)
s.sensor_latitude_degrees = 60.1768      # a bare number is FieldState.Value
s.platform_heading_degrees = 159.97
s.platform_pitch_degrees = None          # None is FieldState.NotAvailable()
wire = encode(s)                          # bytes
got = decode(wire)
print(got.sensor_latitude_degrees.value, got.timestamp_us)

match got.platform_roll_degrees:
    case FieldState.Value(roll):
        print(f"roll {roll}°")
    case FieldState.NotAvailable():
        print("roll omitted")
    case FieldState.SenderError(code):
        print(f"roll sender error, raw code {code}")
    case FieldState.Invalid(None):
        print("roll tag had the wrong length; its bytes are in got.unknown")
    case _:
        pass
```

`St0601` fields are properties in engineering units: set them with plain
assignment (`s.sensor_latitude_degrees = ...`), never a `set_*` method
call — none exist. A setter takes a number, `None` or a `FieldState`.
`decode` raises `KlvError` for a structural reason only (framing, the
checksum, the mandatory Tag 2 timestamp); a field's value never fails
the set. `encode` raises `KlvEncodeError` for a value outside its tag's
range or NaN (nothing is pulled into range) and for a state the wire
cannot carry, such as `FieldState.AtLeast`.

---

## 4. Context managers

Every parser supports the `with` protocol:

```python
from marlin.envelope import StreamingParser

with StreamingParser() as parser:
    parser.feed(chunk)
    for sentence in parser:
        handle(sentence)
```

Today this is stylistic. The parsers hold no OS resources, so explicit
scoping does nothing concrete that a plain assignment doesn't. The
value is forward compatibility: if a future parser variant ever owns a
worker thread or a socket, it cleans up via `__exit__` without breaking
existing call sites.

`OneShotParser`, `StreamingParser`, `Nmea0183Parser`, and `AisParser`
all implement the protocol.

---

## 5. Serialization with dataclass mirrors

`marlin.dataclasses` provides a frozen `@dataclass` mirror for every
typed runtime message. Convert any message with `to_dataclass`:

```python
import dataclasses
import json

from marlin.envelope import StreamingParser
from marlin.dataclasses import to_dataclass

parser = StreamingParser()
parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n")

for sentence in parser:
    dc = to_dataclass(sentence)
    payload = dataclasses.asdict(dc)
    print(json.dumps(payload, default=_default))
```

Coverage:

- envelope: `RawSentence`
- typed NMEA: `Gga`, `Gll`, `Hdg`, `Hdt`, `Rmc`, `Tll`, `Ttm`, `Vtg`,
  `Psxn`, `Prdid`, `Unknown`
- typed AIS bodies: `PositionReportA`, `StaticAndVoyageA`,
  `SarAircraftPositionReport`, `PositionReportB`,
  `ExtendedPositionReportB`, `AidToNavigationReport`, `StaticDataB24A`,
  `StaticDataB24B`, `Other`
- AIS wrapper: `AisMessage`
- field states: `Value`, `AtLeast`, `NotAvailable`, `SenderError` and
  `Invalid`, generic frozen dataclasses with a `kind` literal, nested in
  every message mirror on the same attributes; the AIS sum types
  `RateOfTurn`, `Timestamp` and `Type24BExtent` as variant mirrors under
  a namespace class of the type's name

A few mirrors carry `bytes` fields: `RawSentence.fields`, `Psxn.token`,
`PrdidRaw.fields`, `Other.raw_payload`. `json.dumps` rejects raw bytes,
so pass a `default=` handler:

```python
def _default(value):
    return value.hex() if isinstance(value, bytes) else str(value)
```

A field state serializes as a dict with its `kind` and payload:
`{"speed_knots": {"kind": "value", "value": 22.4}}`,
`{"speed_over_ground": {"kind": "at_least", "bound": 102.2}}`,
`{"hdop": {"kind": "not_available"}}`. An enum payload (`fix_quality`,
`navigation_status`, `epfd`, etc.) is stored as its wire integer, which
JSON encodes without help. The dict form is the one to `match` on
after a round trip through JSON:

```python
for name, state in payload.items():
    match state:
        case {"kind": "value", "value": value}:
            print(f"{name} = {value}")
        case {"kind": "at_least", "bound": bound}:
            print(f"{name} >= {bound}")
        case {"kind": "not_available"}:
            pass
        case {"kind": "sender_error" | "invalid", "code": code}:
            print(f"{name} unusable ({code})")
        case _:
            pass  # a plain attribute such as `talker` or `mmsi`
```

The mirrors plug into `msgspec`, `pydantic` adapters, structured
loggers, or anything else that reads plain Python dataclasses.

---

## Where to go next

- `examples/` has six runnable scripts: file replay, single datagrams,
  TCP-style framing, AIS reassembly, a stdin pipeline, and a live ship
  tracker (envelope + AIS combined).
- `README.md` covers install, the error hierarchy, and AIS clock modes.
- `CHANGELOG.md` tracks per-version changes.
- `python/marlin/*/__init__.pyi` has exact type signatures for every public
  symbol.
