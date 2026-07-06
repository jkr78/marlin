# Architecture

## marlin-klv

Standalone `#![no_std]` + `alloc` leaf crate: a MISB ST 0601 (UAS
Datalink Local Set) KLV encoder/decoder. Unlike `marlin-nmea-0183` and
`marlin-ais`, it does not depend on `marlin-nmea-envelope`: KLV is not
NMEA-framed, so there's no shared envelope layer to sit on.

### Modules

- `ber` — BER length codec (short and long form) and big-endian
  fixed-width readers.
- `scale` — MISB ST 0601 legacy linear int↔f64 scaling (not ST 1201
  IMAPB; those are newer tags, out of scope today).
- `tags` — macro-driven registry of the 20 scaled tags. Each
  `scaled_tags!` entry expands to that tag's encode arm, decode-dispatch
  arm, and getter/setter accessor pair.
- `checksum` — BCC-16 (Tag 1) running-sum checksum.
- `error` — `Error` (non_exhaustive, `Clone`): truncated input, BER
  length overflow, checksum mismatch, wrong local-set key.
- `st0601` — the encode/decode orchestrator. Owns `St0601`,
  `UAS_LS_KEY`, `encode`, `decode`, `precision_timestamp`, and (behind
  the `bytes` feature) `encode_to_bytes`. Composes `ber` + `scale` +
  `tags` + `checksum`.

### Data model

`St0601` stores raw wire integers per tag (`Option<u16>`,
`Option<i32>`, and so on), not engineering units. Each scaled tag gets
an accessor pair (e.g. `sensor_latitude_degrees()` /
`set_sensor_latitude_degrees()`) that converts to and from degrees or
meters. Getters return `None` when the tag is absent, or when the wire
value is the ST 0601 sentinel (`i16::MIN` / `i32::MIN`). Setters clamp
the input to the tag's valid range before conversion, so encoding can
never emit a sentinel value.

Unrecognized tags round-trip verbatim through
`St0601::unknown: Vec<(u8, Vec<u8>)>`, preserved in wire order.

### Invariants

- **Byte-exact round-trip.** `decode(encode(set)) == set` for this
  crate's own output. `encode` reproduces the source bytes for any
  packet it can decode, including a known tag with an unexpected wire
  length (falls back to `unknown` rather than erroring).
- **Tolerant decode.** A malformed known tag doesn't fail the whole
  decode; it lands in `unknown` instead. Tag 2 (precision timestamp) is
  the one exception — mandatory, so a malformed Tag 2 fails the whole
  decode.
- **Framing order.** Tag 2 first, then Tag 65 (version) if present,
  then the scaled tags in ascending tag order, then preserved unknown
  tags in original order, then Tag 1 (checksum) last.

### no_std rounding

Encoding rounds engineering values to the nearest wire count with
`libm::round` (round-half-away-from-zero), since `f64::round` isn't
available in `core` under `no_std`. `libm` is a plain dependency, not
feature-gated.

### Optional `bytes` feature

`encode_to_bytes` (behind the `bytes` feature) encodes directly into a
`bytes::Bytes`, for callers that hand ownership downstream instead of
taking a `Vec<u8>`.
