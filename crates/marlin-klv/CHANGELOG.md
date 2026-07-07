# Changelog

All notable changes to `marlin-klv` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
