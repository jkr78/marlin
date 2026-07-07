# Changelog

All notable changes to `marlin-nmea-0183` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.3] - 2026-07-07

No behavioral changes. Lockstep version bump with the workspace release that
extends `marlin-klv` with a tag-registry inspection API. Dropped the stale
hardcoded `html_root_url` doc attribute (docs.rs sets the root automatically).

## [0.1.2] - 2026-07-06

No behavioral changes. The version bump tracks the workspace release that
adds the new `marlin-klv` crate (MISB ST 0601 KLV encoder/decoder).

## [0.1.1] - 2026-05-08

### Added

- `RMC` decoder (`decode_rmc`, `RmcData`, re-exported types
  `RmcNavStatus`, `UtcDate`). Single-sentence carrier of UTC time,
  date, position, speed, course, and magnetic variation. Accepts
  pre-NMEA-2.3 (11 fields), NMEA-2.3+ with mode (12 fields), and
  NMEA-4.10+ with nav status (13 fields).
- `GLL` decoder (`decode_gll`, `GllData`). Position-only sentence
  with UTC time, validity status, and optional mode indicator
  (NMEA 2.3+).
- `DataStatus` shared validity-flag enum (`Active` / `Void` / `Other`)
  used by both RMC and GLL.
- `Nmea0183Message::Rmc` and `Nmea0183Message::Gll` variants on the
  top-level message enum. The dispatcher in `decode` / `decode_with`
  now routes `RMC` and `GLL` sentence types to typed variants instead
  of `Unknown`.

## [0.1.0] - 2026-04-28

### Added

- Initial release of `marlin-nmea-0183` — typed sans-I/O decoders for
  NMEA 0183 sentences, built on `marlin-nmea-envelope`
- `Nmea0183Message` non-exhaustive enum with an `Unknown(RawSentence)`
  variant for sentence types this crate does not yet decode
- `DecodeError` (non-exhaustive, `thiserror`-derived) reporting the
  failing field index on parse error
- Decoders for the v0.1 sentence set:
  - `GGA`: fix quality, satellites, HDOP, altitude, geoid, DGPS fields
  - `VTG`: pre-2.3 and 2.3+ forms; `VtgMode` covers every recognized
    mode indicator
  - `HDT`: true heading
  - `PSXN`: install-configured 6-slot layout (`PsxnLayout`, `PsxnSlot`)
    including the TSS sine-encoded roll/pitch variant; `FromStr` for
    legacy `"rphx1"` config strings
  - `PRDID`: two dialect structs (`PitchRollHeading`,
    `RollPitchHeading`) plus a strict-default `PrdidDialect::Unknown`
    that emits `PrdidData::Raw` rather than guessing
- `UtcTime` with millisecond resolution
- Latitude/longitude decoder (`ddmm.mmmm` + hemisphere → signed decimal
  degrees)
- `DecodeOptions` with `with_psxn_layout` and `with_prdid_dialect`
  builders; `decode` uses defaults, `decode_with` takes explicit options
- `Nmea0183Parser<P>` generic wrapper plus a runtime-dispatch `Parser`
  enum, mirroring the envelope crate's pattern
- `Nmea0183Error` unifies envelope and decode errors at the parser surface
- Per-sentence `decode_gga` / `decode_vtg` / `decode_hdt` / `decode_psxn`
  / `decode_prdid` functions exported as extension points
- Full rustdoc and README
