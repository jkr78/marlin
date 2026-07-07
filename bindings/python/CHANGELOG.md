# Changelog

All notable changes to `marlin-py` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.3] - 2026-07-07

### Added

- `marlin.klv` inspection surface for tooling that reverse-engineers a KLV
  stream: `UAS_LS_KEY` (the 16-byte local-set label every packet is framed
  with), `tags()` (all 22 decodable tags as `TagInfo` with `number` / `name` /
  `unit`), `tag_number(name)`, and `tag_name(number)`. The registry comes
  straight from the Rust codec, so it stays in step with `decode` across
  releases instead of being re-derived in Python. `TagInfo` is frozen and
  hashable.

## [0.1.2] - 2026-07-06

### Added

- `marlin.klv` — Python bindings for the new `marlin-klv` crate (MISB
  ST 0601 KLV encoder/decoder). `St0601` exposes engineering-unit
  accessors (e.g. `sensor_latitude_degrees`) plus `raw_*` wire-integer
  escape hatches, both readable and writable, alongside `timestamp_us`,
  `version`, and a read-only `unknown` list. Module functions
  `decode(bytes) -> St0601`, `encode(St0601) -> bytes`, and
  `precision_timestamp(bytes) -> int | None`. `KlvError` subclasses
  `MarlinError`. This is the suite's first encoder exposed to Python —
  KLV round-trips in both directions, unlike the decode-only NMEA/AIS
  bindings.

## [0.1.1] - 2026-05-08

### Added

- Typed `Rmc` and `Gll` Python classes with full attribute getters,
  matching the new `marlin-nmea-0183` decoders. The default
  `Nmea0183Parser` iterator now surfaces `RMC` and `GLL` sentences as
  typed objects instead of `Unknown`.
- New shared types: `DataStatus` (`ACTIVE` / `VOID`), `RmcNavStatus`
  (`SAFE` / `CAUTION` / `UNSAFE` / `NOT_VALID`), and `UtcDate`
  (day / month / year_yy).
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
