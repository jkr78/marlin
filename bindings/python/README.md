# marlin-py

Python bindings for the Marlin Rust suite — NMEA 0183 envelope parsing,
typed sentence decoding, and AIS (AIVDM/AIVDO) message decoding with
multi-sentence reassembly.

## Status

Wraps three Rust crates: `marlin-nmea-envelope` (framing + checksum),
`marlin-nmea-0183` (GGA, VTG, HDT, PSXN, PRDID typed decoders), and
`marlin-ais` (Types 1/2/3/5/18/19/24A/24B + reassembly). API surface is
feature-complete for v0.1. `py.typed` marker and `.pyi` stubs ship with the
package; mypy `--strict` is clean across the entire `python/marlin/`, `tests/`, and `examples/` tree.

## Install

```bash
# Local development — build the extension in-place.
cd bindings/python
maturin develop
```

PyPI publish is deferred to post-v0.1. No wheel is currently on PyPI.

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
gga = decode_gga(raw)                # raises DecodeError if not GGA or fields are malformed
print(gga.latitude_deg, gga.longitude_deg, gga.fix_quality)

# Streaming parser — same feed/iterate API as the envelope layer.
parser = Nmea0183Parser.streaming()
parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n")
for msg in parser:
    print(type(msg).__name__, msg)
```

`DecodeOptions` configures dialect-sensitive sentences:

```python
opts = DecodeOptions().with_prdid_dialect(PrdidDialect.PITCH_ROLL_HEADING)
parser = Nmea0183Parser.streaming(options=opts)
```

Per-sentence extension functions (`decode_gga`, `decode_vtg`, `decode_hdt`,
`decode_psxn`, `decode_prdid`) are public so downstream code can build its
own message enum and delegate to Marlin for the standard types.

## Quickstart: AIS

```python
from marlin.ais import AisParser, PositionReportA

parser = AisParser.streaming()
parser.feed(b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n")
for msg in parser:
    if isinstance(msg.body, PositionReportA):
        print(msg.body.mmsi, msg.body.latitude_deg, msg.body.longitude_deg)
```

Multi-sentence reassembly is automatic — feed fragments in order and the
parser yields a complete `AisMessage` only when the last fragment arrives.

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
- `DecodeError` — field-level decode failure in a typed NMEA sentence
- `AisError` — AIS armor or bit-level decode failure
- `ReassemblyError` — fragment reassembly violation (out-of-order, channel
  mismatch, or timeout eviction)

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

The Python bindings wrap three Rust crates:
[`marlin-nmea-envelope`](../../crates/marlin-nmea-envelope),
[`marlin-nmea-0183`](../../crates/marlin-nmea-0183),
[`marlin-ais`](../../crates/marlin-ais).

## MSRV / Python version

Rust 1.82. Python 3.9+.

## License

Dual-licensed under MIT OR Apache-2.0.
