# marlin-py — usage guide

Five patterns covered below: sync streaming, asyncio integration,
per-protocol filtering, context managers, and dataclass serialization.
Examples use real bytes (files, sockets, captured logs).

For install steps, see `README.md`. For exact signatures, read the type
stubs in `python/marlin/*.pyi`.

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
            print(f"fix: {msg.latitude_deg}, {msg.longitude_deg}")
        elif isinstance(msg, Vtg):
            print(f"speed: {msg.speed_knots} kn")
        elif isinstance(msg, Hdt):
            print(f"heading: {msg.heading_true_deg}°")
```

`Nmea0183Parser` decodes GGA, VTG, HDT, PSXN, and PRDID into typed
classes. Anything it doesn't recognize, AIVDM included, surfaces as
`Unknown`. Match `Unknown` to forward those sentences elsewhere, or skip
it if you only care about typed messages.

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
            print(f"MMSI {body.mmsi}: {body.latitude_deg}, {body.longitude_deg}")
        elif isinstance(body, StaticAndVoyageA):
            print(f"MMSI {body.mmsi}: {body.vessel_name}")
```

`AisParser` filters non-AIS sentences out, so you only see decoded AIS
messages. It also handles multi-fragment reassembly: a Type 5 split
across two `!AIVDM` lines arrives as a single `AisMessage`.

The `body` attribute holds the typed payload. `msg.type_tag` is a string
like `"type1"`, `"type5"`, or `"type18"` for filtering without
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
- typed NMEA: `Gga`, `Vtg`, `Hdt`, `Psxn`, `Prdid`, `Unknown`
- typed AIS bodies: `PositionReportA`, `PositionReportB`,
  `ExtendedPositionReportB`, `StaticAndVoyageA`, `StaticDataB24A`,
  `StaticDataB24B`, `Other`
- AIS wrapper: `AisMessage`

A few mirrors carry `bytes` fields: `RawSentence.fields`, `Psxn.token`,
`PrdidRaw.fields`, `Other.raw_payload`. `json.dumps` rejects raw bytes,
so pass a `default=` handler:

```python
def _default(value):
    return value.hex() if isinstance(value, bytes) else str(value)
```

Enum-typed fields (`fix_quality`, `navigation_status`, `epfd`, etc.)
are stored as their wire integer, which JSON encodes without help.

The mirrors plug into `msgspec`, `pydantic` adapters, structured
loggers, or anything else that reads plain Python dataclasses.

---

## Where to go next

- `examples/` has six runnable scripts: file replay, single datagrams,
  TCP-style framing, AIS reassembly, a stdin pipeline, and a live ship
  tracker (envelope + AIS combined).
- `README.md` covers install, the error hierarchy, and AIS clock modes.
- `CHANGELOG.md` tracks per-version changes.
- `python/marlin/*.pyi` has exact type signatures for every public
  symbol.
