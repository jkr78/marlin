# Changelog

All notable changes to `marlin-nmea-0183` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `UtcTime` and `UtcDate` implement `FromStr`, with the unit error types
  `ParseUtcTimeError` and `ParseUtcDateError`, so a caller can parse
  `hhmmss[.sss]` and `ddmmyy` text with the standard trait.
- Re-exports of `marlin_field::{FieldState, Invalid, Kind, RawCode}` at
  the crate root; the crate depends on `marlin-field` from 0.3.0.

### Changed (BREAKING)

- Every field the wire can leave empty, or fill with text the decoder
  cannot read, is a `FieldState<T>` instead of `Option<T>`: an empty
  field is `NotAvailable` and unreadable text is `Invalid`, so one bad
  field no longer fails the sentence. `value()` is the one-line
  migration where `Option<T>` was read before. The two status fields
  that were bare enums, `GllData::status` and `RmcData::status`, and
  the bare `GgaData::fix_quality` follow the same rule: an empty status
  byte is `NotAvailable`, where it used to decode as the fabricated
  `DataStatus::Other(0)`. Per rule:
  - A number (`number` text, `UtcTime`, `UtcDate`): empty →
    `NotAvailable`; non-UTF-8, a parse failure or a range failure
    (coordinate beyond ±90°/±180°, UTC hour above 23, day 0) →
    `Invalid(Unparsable)`. 0183 numeric text has no wire integer, so
    no numeric invalid state carries a raw code.
  - A one-byte letter code (`DataStatus`, `VtgMode`, `RmcNavStatus`,
    `TargetStatus`, `AngleReference`, `DistanceUnits`,
    `AcquisitionType`): empty, or absent from a short sentence →
    `NotAvailable`; an unnamed byte → `Invalid(Undefined(RawCode(byte)))`;
    two or more bytes → `Invalid(Unparsable)`. Reading only the first
    byte of a longer field is gone. Lowercase letters stay accepted.
  - The GGA fix quality is a digit code: an undefined number →
    `Invalid(Undefined(RawCode(n)))` with the digit, not the ASCII
    byte, as the raw code; a field of two or more bytes is now that
    invalid state instead of a sentence failure.
  - A paired field (latitude with `N`/`S`, longitude with `E`/`W`, HDG
    deviation and variation and RMC magnetic variation with `E`/`W`):
    both empty → `NotAvailable`; exactly one empty → `Invalid(Unparsable)`;
    a letter outside the pair → `Invalid(Undefined(RawCode(byte)))`.
  - TTM and TLL `name`: empty → `NotAvailable`; non-UTF-8 →
    `Invalid(Unparsable)`.
  - TTM and TLL `reference_target` is `FieldState<bool>`: empty or
    absent → `Value(false)`, never `NotAvailable`; `R`/`r` →
    `Value(true)`; another byte → `Invalid(Undefined(RawCode(byte)))`.
  - PSXN `id`, `token` and `heave_m` take their data field's state;
    the derived `roll_deg` and `pitch_deg` are `FieldState<f32>` too: a
    role no data field carries, or an empty data field →
    `NotAvailable`; an unreadable data field → `Invalid(Unparsable)`;
    a sine-encoded value no angle can produce, or gimbal lock →
    `Invalid(Unparsable)`; a sine-encoded roll whose pitch source is
    not a value takes the pitch's state. The two PRDID typed dialects'
    angles are `FieldState<f32>`.

  Exceptions and renames that come with the rule:
  - `Other(u8)` is removed from the seven letter-code enums; an
    unnamed byte is the invalid field state with the byte as its raw
    code. The enums stay `#[non_exhaustive]`.
  - `GgaFixQuality::Invalid` is renamed `NoFix`: the sender's own "no
    fix" is a value, and "invalid" now always means the decoder could
    not give the field a meaning.
  - `UtcTime` and `UtcDate` are parsed through their new `FromStr`
    impls (see Added).
- `DecodeError` keeps `NotEnoughFields` only. `InvalidNumber`,
  `OutOfRange`, `InvalidHemisphere`, `InvalidUtf8` and `InvalidUtcTime`
  are removed; each is now the invalid field state. `decode` on a known
  sentence type with enough fields never returns `Err`.

## [0.2.0] - 2026-10-08

### Added

- Re-exports of `marlin_nmea_envelope::{OneShot, Parser, Streaming}`,
  beside the existing `RawSentence` re-export, so a caller needs no
  direct dependency on the envelope crate to build a source.

### Changed (BREAKING)

- The `Parser` source-mode enum is removed. `Nmea0183Parser<P>`
  now covers that case by wrapping `marlin_nmea_envelope::Parser`,
  which implements `SentenceSource` (ADR-0005). Replacements:
  `Parser::one_shot()` → `Nmea0183Parser::new(OneShot::new())`;
  `Parser::streaming()` → `Nmea0183Parser::new(Streaming::new())`;
  `Parser::streaming_with_capacity(n)` →
  `Nmea0183Parser::new(Streaming::with_capacity(n))`;
  `Parser::one_shot_with_options(o)` →
  `Nmea0183Parser::with_options(OneShot::new(), o)`;
  `Parser::streaming_with_options(o)` →
  `Nmea0183Parser::with_options(Streaming::new(), o)`. For a source
  mode chosen at runtime, pass `Parser::one_shot()` or
  `Parser::streaming()` from the envelope crate as the source.

### Fixed

- The Cargo description and the crate-level docs now list every
  decoded sentence. GLL, RMC, HDG, TTM, and TLL were missing.
- The README and the crate-level quickstart build `Nmea0183Parser`
  over a source, as the marlin-ais docs do, and show `Parser` for a
  source mode chosen at runtime and `with_options` for the
  proprietary-sentence settings. The README used to show only the
  per-sentence decoders. The crate docs' README link pointed back at
  the crate docs; it now points at the repository README.

## [0.1.4] - 2026-07-07

### Added

- Typed decoders for three radar/heading sentences, previously returned
  as `Unknown`: **HDG** (heading, deviation & variation — signed E/W
  corrections), **TTM** (tracked target message — the `$RATTM` radar
  form is standard TTM with the talker preserved), and **TLL** (target
  latitude/longitude). New coded-field enums `TargetStatus` (L/Q/T),
  `AngleReference` (T/R), `DistanceUnits` (K/N/S), and `AcquisitionType`
  (A/M/R), each with an `Other(u8)` fallback. TTM's NMEA-3.0 trailing
  fields (UTC time, acquisition type) are optional.

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
