# Changelog

All notable changes to `marlin-field` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Initial release of `marlin-field`: the decoded-field state type shared
  by the marlin decoders. A `#![no_std]` leaf crate with no dependencies
  and no `alloc`. First published at 0.3.0 to stay in lockstep with the
  workspace release that moves `marlin-nmea-0183`, `marlin-ais` and
  `marlin-klv` onto it (ADR-0008).
- `FieldState<T>` with the five states `Value(T)`, `AtLeast(T)`,
  `NotAvailable`, `SenderError(RawCode)` and `Invalid(Invalid)`, and the
  accessors `value`, `value_or_bound`, `kind`, `map`, `as_ref`, `as_mut`.
  `From<T>` lifts a bare value into `Value`.
- `Kind`, the state with the value erased, with `Kind::name()` returning
  the stable cross-language tag (`value`, `at_least`, `not_available`,
  `sender_error`, `invalid`).
- `RawCode(i64)` and `Invalid { Undefined(RawCode), Unparsable }`.
