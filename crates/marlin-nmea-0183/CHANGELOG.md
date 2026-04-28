# Changelog

All notable changes to `marlin-nmea-0183` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
