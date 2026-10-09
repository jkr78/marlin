# Changelog

All notable changes to `marlin-klv` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `St0601::new(timestamp_us)`: a set carrying only the mandatory Tag 2
  timestamp, every optional field `NotAvailable` and `unknown` empty.
  `decode` builds through it.
- `KlvEncodeError` (`#[non_exhaustive]`): `Unencodable { tag, kind }` for a
  field state the tag cannot carry on the wire and `OutOfRange { tag }` for
  a value outside the tag's engineering range or NaN.
- Re-exports of `marlin_field::{FieldState, Invalid, Kind, RawCode}` at
  the crate root; the crate depends on `marlin-field` from 0.3.0.
- `KlvDecodeError::MissingChecksum` and `KlvDecodeError::BadTimestamp`,
  the two structural failures a set can have besides framing and a
  checksum mismatch.

### Changed (BREAKING)

Migration guide: [`docs/migration-0.3.md`](../../docs/migration-0.3.md).

- Every scaled tag is a `FieldState<f64>` field in engineering units,
  named as the former accessor (`sensor_latitude_degrees`,
  `slant_range_meters`, `platform_true_airspeed_mps`), and `version` is a
  `FieldState<u8>`. The raw-count fields, the twenty getter/setter pairs
  and `Default` are gone: build with `St0601::new` and assign a field
  directly (`set.sensor_latitude_degrees = FieldState::Value(60.1768)` or
  `60.1768.into()`); `value()` is the one-line migration where a getter
  was read. The wire count survives only inside `SenderError`; a client
  that wants counts calls `encode`. `TagInfo.name` keeps the base name
  (`sensor_latitude`).
- An omitted tag decodes to `NotAvailable` on the field itself, no
  `Option`.
- The ST 0601 sentinel on a signed tag (`i16::MIN` on Tags 6 and 7,
  `i32::MIN` on Tags 13, 14, 19, 23, 24, 40 and 41) decodes to
  `SenderError(RawCode)` with the raw code, where the accessor used to
  return `None` and the raw field kept the count.
- A known tag with the wrong wire length (Tag 65 included) decodes to
  `Invalid(Unparsable)` on its field **and** keeps its bytes in `unknown`,
  where it used to land in `unknown` alone; re-encode stays byte-exact.
  When the same tag also occurs with the right length, the readable
  occurrence keeps the field (last readable wins) and the wrong-length
  bytes still ride in `unknown`, so re-decoding the re-encoded set gives
  the same set.
- `decode` fails with `BadTimestamp` when Tag 2 is absent (the set used to
  decode with `timestamp_us` 0) or not 8 bytes (used to be `Truncated`
  with a fabricated offset), and with `MissingChecksum` when Tag 1 is
  absent (used to be `BadChecksum { computed: 0, embedded: 0 }`) or not 2
  bytes (used to be `Truncated`). `precision_timestamp` reports a
  wrong-length Tag 2 as `BadTimestamp` too. `Truncated` now means only
  that the input ends early. The `KlvDecodeError` docs state the
  rule: a decode fails for a structural reason only; a field's value never
  fails the set.
- `encode` and `encode_to_bytes` return `Result<_, KlvEncodeError>`. Per
  field state: a value emits its range-checked count, `NotAvailable`
  omits the tag, `SenderError` emits the tag's own sentinel,
  `Invalid(Unparsable)` emits nothing for the typed tag (its bytes ride in
  `unknown`); `AtLeast`, `Invalid(Undefined)`, a `SenderError` whose code
  is not the tag's own indicator, and any `SenderError` on an unsigned
  tag or Tag 65 are `Unencodable`. On `Err` the output buffer is untouched.
- Clamp-on-encode and NaN-to-minimum are gone: a value outside the tag's
  range (inclusive at both ends, checked before rounding) or NaN is
  `OutOfRange`, where it used to be clamped (NaN to the range minimum).
- `Error` is renamed `KlvDecodeError`. Every public error type in the
  workspace now carries its crate's prefix, with an operation word where a
  crate has more than one (ADR-0010); the variants are unchanged.

## [0.2.0] - 2026-10-08

No behavioral changes. Lockstep version bump with the workspace release that
removes the per-crate `Parser` enums and adds the AIS Type 9 and Type 21
decoders.

## [0.1.4] - 2026-07-07

No behavioral changes. Lockstep version bump with the workspace release that
adds HDG/TTM/TLL sentence decoders to `marlin-nmea-0183`.

## [0.1.3] - 2026-07-07

### Added

- Tag-registry inspection API, sourced from the codec's own table so it cannot
  drift from `decode`: `tags() -> &[TagInfo]` (all 22 decodable tags — the 20
  scaled tags plus Tag 2 timestamp and Tag 65 version, in ascending order),
  `tag_number(name) -> Option<u8>`, and `tag_name(number) -> Option<&str>`.
  `TagInfo` carries the wire number, the `St0601` field base name (e.g.
  `sensor_latitude`), and the engineering unit (`degrees` / `meters` / `mps` /
  `microseconds`, or `None`). Tag 1 (checksum) is framing and never listed.

### Changed

- Dropped the stale hardcoded `html_root_url` doc attribute (docs.rs sets the
  root automatically). No API impact.

## [0.1.2] - 2026-07-06

### Added

- Initial release of `marlin-klv` — a sans-I/O, `no_std` MISB ST 0601
  (UAS Datalink Local Set) KLV encoder and decoder. Standalone leaf crate
  with no dependency on `marlin-nmea-envelope` (KLV is not NMEA-framed).
  First published at 0.1.2 to stay in lockstep with the workspace release.
- `encode(&St0601, &mut Vec<u8>)`, `decode(&[u8]) -> St0601`, and a cheap
  checksum-free `precision_timestamp(&[u8])` Tag 2 peek. Optional `bytes`
  feature adds `encode_to_bytes`.
- `St0601` stores raw wire integers with engineering-unit accessor pairs
  (degrees / meters / m·s⁻¹) that clamp to range on encode and honour the
  ST 0601 `i16::MIN` / `i32::MIN` sentinels (returning `None`) on decode.
- 20 scaled ST 0601 tags — platform heading/pitch/roll/true-airspeed,
  sensor lat/lon/altitude/FOV/relative pointing, slant range, target
  width, frame-center lat/lon/elevation, target-location lat/lon/elevation
  — plus framing Tag 2 (precision timestamp), Tag 65 (LS version), and
  Tag 1 (16-bit BCC checksum). Unknown tags round-trip verbatim; a known
  tag with an unexpected wire length degrades to the unknown-tag path
  rather than erroring. Tag 2 with a malformed length is the strict
  exception (a defaulted timestamp would fabricate data).
- `Error` (non-exhaustive, `thiserror`-derived, `Clone`): truncated
  input, BER length overflow, checksum mismatch, wrong local-set key.
- Legacy ST 0601 linear scaling only (ST 1201 IMAPB is out of scope for
  this release). `libm::round` provides round-half-away-from-zero under
  `no_std`.
- `cargo-fuzz` target `klv_decode` asserting the decoder never panics.
