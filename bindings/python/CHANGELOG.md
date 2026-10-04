# Changelog

All notable changes to `marlin-py` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `marlin.ais.TurnDirection` enum (`RIGHT`, `LEFT`) and
  `PositionReportA.turn_direction`: set when a Type 1/2/3 report carries
  raw rate of turn ±127, "turning right/left at more than 5° per 30 s,
  no turn indicator" (ITU-R M.1371-5 Table 48). The enum's int values
  are discriminants, not wire codes. `marlin.dataclasses.PositionReportA`
  mirrors it as `turn_direction: Optional[int]`.
- `StaticDataB24B.mothership_mmsi` and `StaticDataB24B.epfd`: a Type 24
  Part B from an auxiliary craft (MMSI `98MIDxxxx`) carries the mother
  ship's MMSI in the 30 bits that otherwise hold dimensions (ADR-0002);
  the EPFD type at bits 162–165 was previously dropped.
  `marlin.dataclasses.StaticDataB24B` mirrors both.
- `marlin.ais.SarAircraftPositionReport` (`type_tag == "type9"`) and the
  `AltitudeSensor` enum (`GNSS = 0`, `BAROMETRIC = 1`, wire codes): Type 9
  SAR aircraft position reports now decode instead of surfacing as
  `Other(msg_type=9)`. `altitude_m` and `speed_over_ground` are whole
  metres / whole knots (`None` for the not-available codes 4095 / 1023;
  the over-range codes 4094 m and 1022 kn pass through, ADR-0001); DTE,
  assigned and RAIM flags and the 20-bit `radio_status` are exposed.
  `marlin.dataclasses.SarAircraftPositionReport` mirrors it with
  `altitude_sensor` as `int`. New golden fixture directory
  `tests/fixtures/ais/` holds the gpsd T9-1 / T9-2 vectors
  (BSD-2-Clause, attributed in `tests/fixtures/README.md`).
- `marlin.ais.AidToNavigationReport` (`type_tag == "type21"`) and the
  `AtonType` enum (32 members, `NOT_SPECIFIED = 0` … `LIGHT_VESSEL = 31`,
  wire codes of ITU-R M.1371-5 Table 74): Type 21 aid-to-navigation
  reports now decode instead of surfacing as `Other(msg_type=21)`. `name`
  joins the 20-character name with the optional extension of up to 14
  more characters and trims trailing `@` / spaces (an embedded `@` is
  kept); `dimensions` reuses `Dimensions` and is all-`None` for virtual
  AtoN; the off-position, virtual, assigned and RAIM flags and the raw
  8-bit `aton_status` are exposed. Payloads over 360 bits are tolerated,
  under 272 raise `AisError`. `marlin.dataclasses.AidToNavigationReport`
  mirrors it with `aton_type` and `epfd` as `int`. Golden fixtures
  `02_type21_gpsd_extension`, `03_type21_gpsd_overlong` (gpsd T21-1 and
  T21-2) and `04_type21_short` (246 bits, decodes to nothing).
- `marlin.ais.AisMessageBody`, `marlin.ais.ClockMode`, and
  `marlin.nmea.Nmea0183Message` exist at runtime. The stubs always
  exported these three type aliases, but importing one outside
  `if TYPE_CHECKING:` raised `ImportError`. `AisMessageBody` is the
  `Union` of the nine AIS body classes, `ClockMode` is
  `Literal["auto", "manual"]`, and `Nmea0183Message` is the `Union` of
  the eleven NMEA message classes. `marlin.dataclasses.AisMessageBody`
  is a different alias, the union of the dataclass mirrors.
- `marlin.dataclasses.to_dataclass` also converts a value type passed on
  its own (`UtcTime`, `UtcDate`, `Dimensions`, `Eta`, and the three
  `Prdid` bodies), which used to raise `TypeError`. Messages convert
  exactly as before.

### Fixed

- Every int-backed enum in `marlin.ais` and `marlin.nmea` (`NavStatus`,
  `ManeuverIndicator`, `TurnDirection`, `EpfdType`, `AisVersion`,
  `GgaFixQuality`, `VtgMode`, `DataStatus`, `RmcNavStatus`, `TargetStatus`,
  `AngleReference`, `DistanceUnits`, `AcquisitionType`, `PsxnSlot`,
  `PrdidDialect`) is now hashable, so members work as set members and
  dict keys. The stubs always declared `__hash__`; at runtime `hash()`
  raised `TypeError`.
- Two multi-sentence AIS messages sharing a sequential id on channels A
  and B both decode. Previously the second channel's continuation
  fragment raised `ReassemblyError` and both messages were lost
  (`marlin-ais` reassembly fix).
- Parser `__exit__` type stubs now return `Literal[False]` (they never suppress
  exceptions), so type checkers no longer report variables assigned inside a
  `with parser as p:` block as possibly-unbound after the block.
- The package docstring, `marlin.dataclasses.to_dataclass` docstring,
  README, and GUIDE now list every typed NMEA sentence (GLL, RMC, HDG,
  TTM, and TLL were missing) and the AIS Type 9 and Type 21 classes.
- The stubs for `marlin.nmea.Gga`, `Vtg`, `Hdt`, and `Unknown` no longer
  mark the constructor parameters keyword-only. The runtime always
  accepted them positionally, like every other message class.

### Changed (BREAKING)

- `PositionReportA.rate_of_turn` is `None` for raw rate of turn ±127, which
  used to decode to a fabricated ±720.0 °/min; the status now lives in
  `turn_direction`. Measured rates (raw 0..=±126) and the −128
  not-available sentinel are unchanged. Consumers that treated
  `abs(rate_of_turn) > 708.7` as the no-turn-indicator case should test
  `turn_direction is not None` instead.
- `StaticDataB24B.dimensions` is `Optional[Dimensions]`: `None` for an
  auxiliary craft, whose extent surfaces as `mothership_mmsi` instead
  (ADR-0003 flattening; parser output sets exactly one of the two). The
  constructor no longer substitutes an all-`None` `Dimensions` when the
  argument is omitted. `marlin.dataclasses.StaticDataB24B.dimensions` is
  `Optional[Dimensions]` likewise.
- `ReassemblyError` no longer carries a "channel mismatch" message; the
  underlying `marlin-ais` variant is removed. The out-of-order and
  timeout messages are unchanged.

### Changed

- The stub for the private `marlin._core` extension module no longer
  repeats every class. It keeps `__version__` and the exception classes
  and types the `envelope`, `nmea`, `ais`, and `klv` submodules as `Any`.
  The public stubs beside each package are unchanged and are now the
  only place a class is declared. Code that imported from `marlin._core`
  directly loses its type information and should import from
  `marlin.envelope`, `marlin.nmea`, `marlin.ais`, or `marlin.klv`.
- Every binding class is marked `@final` in the stubs, matching the
  runtime: subclassing one always raised `TypeError`, and a type checker
  now reports it.
- The stubs declare constructors as `__new__`, which is what the
  extension classes define, and `__eq__` on the enums and `RawSentence`
  takes its argument positional-only. Ordinary calls type-check as
  before.

## [0.1.4] - 2026-07-07

### Added

- `marlin.nmea` typed classes `Hdg`, `Ttm`, `Tll` for the radar/heading
  sentences now decoded by `marlin-nmea-0183`, plus the enums
  `TargetStatus`, `AngleReference`, `DistanceUnits`, `AcquisitionType`
  (each with an `UNKNOWN` member for uncoded wire bytes). Standalone
  `decode_hdg` / `decode_ttm` / `decode_tll` helpers alongside the
  existing per-sentence decoders.
- `marlin.dataclasses.to_dataclass` now maps `Hdg`, `Ttm`, and `Tll`,
  backed by new frozen mirrors `dataclasses.Hdg`, `dataclasses.Ttm`,
  `dataclasses.Tll`.

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
  M.1371-5 Annex 8 §3.22, Table 78) defines Part A as 160 bits
  exactly. All spec-canonical Part A frames (27-character payloads with `fill_bits=2`)
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
