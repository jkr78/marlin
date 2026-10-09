# marlin-py

Python bindings for the Marlin Rust suite — NMEA 0183 envelope parsing,
typed sentence decoding, AIS (AIVDM/AIVDO) message decoding with
multi-sentence reassembly, and MISB ST 0601 (KLV) encode/decode.

## Status

Wraps four Rust crates: `marlin-nmea-envelope` (framing + checksum),
`marlin-nmea-0183` (GGA, GLL, HDG, HDT, RMC, TLL, TTM, VTG, PSXN, PRDID
typed decoders), `marlin-ais` (Types 1/2/3/5/9/18/19/21/24A/24B +
reassembly), and `marlin-klv` (MISB ST 0601 UAS Datalink Local Set
encode/decode). `marlin.field` carries `FieldState`, the decoded-field
state every decoded field reads (`marlin-field`). `py.typed` marker and
`.pyi` stubs ship with the package; mypy `--strict` is clean across the
entire `python/marlin/`, `tests/`, and `examples/` tree.

## Install

The PyPI distribution name is `marlin-py`; the Python import name stays
plain `marlin`.

```bash
# From PyPI — wheels for Linux x86_64 / aarch64, macOS universal2,
# Windows x86_64. abi3 wheel covers Python 3.10 through 3.14+.
pip install marlin-py

# Local development — build the extension in-place
cd bindings/python
maturin develop
```

## Quickstart: envelope

```python
from marlin.envelope import StreamingParser

parser = StreamingParser()
parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n")
for sentence in parser:
    talker = sentence.talker.decode() if sentence.talker else ""
    print(talker, sentence.sentence_type, sentence.checksum_ok)
    # GP GGA True
```

`OneShotParser` handles UDP datagrams (no `\r\n` required). `parse()` is a
one-call convenience for a single complete sentence.

## Quickstart: NMEA typed decode

```python
from marlin.envelope import parse
from marlin.nmea import Nmea0183Parser, decode_gga, DecodeOptions, PrdidDialect

# Single-sentence convenience.
raw = parse(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47")  # raises EnvelopeError on framing/checksum failure
gga = decode_gga(raw)                # raises DecodeError if not GGA or too few fields
print(gga.latitude_deg.value, gga.longitude_deg.value, gga.fix_quality.value)

# Streaming parser — same feed/iterate API as the envelope layer.
parser = Nmea0183Parser.streaming()
parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n")
for msg in parser:
    print(type(msg).__name__, msg)
```

Every decoded field of a message is a `marlin.field.FieldState`: `.value`
is `None` unless the field holds a value, and a `match` on the variant
class tells not available from invalid (see `GUIDE.md` §3). A field state
is always truthy, so test `.value`, never the object.

`DecodeOptions` configures dialect-sensitive sentences:

```python
opts = DecodeOptions().with_prdid_dialect(PrdidDialect.PITCH_ROLL_HEADING)
parser = Nmea0183Parser.streaming(options=opts)
```

Per-sentence extension functions (`decode_gga`, `decode_gll`, `decode_hdg`,
`decode_hdt`, `decode_rmc`, `decode_tll`, `decode_ttm`, `decode_vtg`,
`decode_psxn`, `decode_prdid`) are public so downstream code can build its
own message enum and delegate to Marlin for the standard types.

## Quickstart: AIS

```python
from marlin.ais import AisParser, PositionReportA

parser = AisParser.streaming()
parser.feed(b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n")
for msg in parser:
    if isinstance(msg.body, PositionReportA):
        print(msg.body.mmsi, msg.body.latitude_deg.value, msg.body.longitude_deg.value)
```

Multi-sentence reassembly is automatic — feed fragments in order and the
parser yields a complete `AisMessage` only when the last fragment arrives.

## Quickstart: KLV

```python
from marlin.klv import St0601, decode, encode

s = St0601(timestamp_us=1_700_000_000_000_000)
s.sensor_latitude_degrees = 60.1768      # a bare number is FieldState.Value
s.platform_heading_degrees = 159.97
wire = encode(s)                          # bytes; KlvEncodeError if out of range
got = decode(wire)
print(got.sensor_latitude_degrees.value, got.timestamp_us)
```

`St0601` fields are properties in engineering units, not setter methods:
assign `s.sensor_latitude_degrees = ...` (a number, `None` for not
available, or a `FieldState`). `decode` raises `KlvError` for a
structural reason only (bad key, truncated bytes, checksum missing or
mismatched, Tag 2 absent or the wrong length); a field's value never
fails the set. `encode` raises `KlvEncodeError` for a value outside its
tag's range or a field state the wire cannot carry.
`precision_timestamp(data)` reads Tag 2 alone, without verifying the
checksum, for callers that only need a cheap timestamp peek.

## AIS clock modes

Fragment reassembly has three timeout behaviours, selected at construction.

**No timeout** — fragments are buffered until the 16-slot eviction cap is
reached. The oldest incomplete group is discarded when a 17th arrives. No
clock reads occur:

```python
parser = AisParser.streaming()          # timeout_ms omitted → no timeout
```

**Auto clock** — the binding calls `time.monotonic_ns()` internally to
drive the timeout. Use this when system time is reliable and you do not
need deterministic replay:

```python
parser = AisParser.streaming(timeout_ms=60_000)          # clock="auto" implied
# or explicitly:
parser = AisParser.streaming(timeout_ms=60_000, clock="auto")
```

**Manual clock** — the caller drives the clock via `tick(now_ms=...)`. No
clock reads occur inside the binding. This is the correct mode for unit
tests, simulators, and any environment where `time` is patched:

```python
parser = AisParser.streaming(timeout_ms=60_000, clock="manual")

parser.feed(fragment_bytes)
parser.tick(now_ms=current_time_ms)     # expire stale groups at this instant
for msg in parser:
    ...
```

Calling `tick()` on a parser that wasn't built with `clock="manual"` raises
`ValueError` — including parsers built with no `timeout_ms` (which default to
`clock="auto"`).

## Errors

Every exception is a `MarlinError` subclass (importable from `marlin`):

- `EnvelopeError` — framing or checksum failure
- `DecodeError` — a typed NMEA sentence with fewer fields than its
  decoder's floor; a bad field is a field state, never an error
- `AisError` — AIS armor or bit-level decode failure
- `ReassemblyError` — fragment reassembly violation (out-of-order or
  timeout eviction)
- `KlvError` — a KLV decode failed for a structural reason (bad
  local-set key, truncated bytes, checksum missing or mismatched, Tag 2
  absent or the wrong length); `variant` names the reason
- `KlvEncodeError` (a `KlvError`) — a KLV encode failed: a value outside
  its tag's range or NaN, or a field state the wire cannot carry;
  carries `variant`, `tag` and `kind`

Property tests (via Hypothesis) verify panic-freedom on arbitrary byte
inputs — no input can cause the binding to crash the interpreter.

## Examples

Six runnable scripts live in `bindings/python/examples/`:

| Script | What it shows |
| --- | --- |
| `one_shot_no_crlf.py` | Single-datagram framing without `\r\n` (`OneShotParser`) |
| `parse_log_file.py` | Disk-backed `.nmea` log replay (`StreamingParser`) |
| `streaming_tcp_style.py` | TCP-style chunked feed; sentences straddle `feed()` boundaries |
| `decode_aivdm_log.py` | Multi-fragment AIS reassembly: Type 1 + Type 5 |
| `parse_stdin.py` | Stdin pipe reader for live NMEA/AIS capture |
| `live_ais_dashboard.py` | Live ship tracker: envelope + AIS in one script, periodic refresh |

## Type stubs

The `py.typed` marker is included in the package. Every public class,
function, and constant has a `.pyi` stub. Downstream projects that run
`mypy --strict` get full type-check coverage without extra configuration.

## Rust crates

The Python bindings wrap four Rust crates:
[`marlin-nmea-envelope`](../../crates/marlin-nmea-envelope),
[`marlin-nmea-0183`](../../crates/marlin-nmea-0183),
[`marlin-ais`](../../crates/marlin-ais),
[`marlin-klv`](../../crates/marlin-klv).

## MSRV / Python version

Rust 1.82. Python 3.10+.

## License

Dual-licensed under MIT OR Apache-2.0.
