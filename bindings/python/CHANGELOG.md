# Changelog

All notable changes to `marlin-py` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Typed `Rmc` and `Gll` Python classes with full attribute getters,
  matching the new `marlin-nmea-0183` decoders. The default
  `Nmea0183Parser` iterator now surfaces `RMC` and `GLL` sentences as
  typed objects instead of `Unknown`.
- New shared types: `DataStatus` (Active / Void), `RmcNavStatus`
  (Safe / Caution / Unsafe / NotValid), and `UtcDate` (day / month /
  year_yy).
- Per-sentence extension-point functions `decode_rmc` and `decode_gll`
  alongside the existing `decode_gga` / `decode_vtg` / etc.
- Frozen dataclass mirrors `Rmc`, `Gll`, and `UtcDate` in
  `marlin.dataclasses`. `to_dataclass(msg)` dispatches both new
  variants for JSON / msgspec serialization.

### Fixed

- AIS Type 24 Part A messages now decode correctly. v0.1.0 enforced a
  168-bit minimum on both parts of Type 24, but the spec (ITU-R
  M.1371-5 §5.3.24.1) defines Part A as 160 bits exactly. All
  spec-canonical Part A frames (27-character payloads with `fill_bits=2`)
  were silently rejected with a `PayloadTooShort`-equivalent error.
  Fix lives in the underlying `marlin-ais` crate.

## [0.1.0] - 2026-04-28

### Added

- Initial release of `marlin-py` — Python bindings for the Marlin Rust suite
  (envelope framing, NMEA 0183 typed decoders, AIS typed decoders + reassembly)
- Synchronous parsers: `OneShotParser`, `StreamingParser` (envelope),
  `Nmea0183Parser` (NMEA), `AisParser` (AIS) — all with iterator protocol
- Context manager support (`with parser as p: ...`) on every parser
- Async iterator helpers in `marlin.aio`: `aiter_sentences`,
  `aiter_nmea_messages`, `aiter_ais_messages` for `asyncio.StreamReader`
  integration
- Frozen dataclass mirrors in `marlin.dataclasses` with `to_dataclass(msg)`
  dispatcher — JSON / msgspec / dataclasses-asdict friendly
- Three AIS clock modes (no-timeout / auto / manual) with deterministic-replay
  guarantee on `clock="manual"`
- Six runnable examples under `bindings/python/examples/`
- Type stubs (`py.typed` + `.pyi` files) for full mypy --strict coverage
- CI workflow: wheel builds for Linux x86_64/aarch64, macOS universal2,
  Windows x86_64; pytest + mypy strict; sdist
- Hypothesis property tests verifying panic-freedom on arbitrary byte input
