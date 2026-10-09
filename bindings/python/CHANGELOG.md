# Changelog

All notable changes to `marlin-py` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-10-09

### Added

- `marlin.klv.KlvEncodeError(KlvError)`, raised by `marlin.klv.encode` for
  a set the wire cannot carry, with `variant` (`"unencodable"` for a
  field state the tag cannot carry, `"out_of_range"` for a value outside
  the tag's range or NaN), `tag` (the ST 0601 tag number) and `kind` (the
  state's `FieldState.kind` for `"unencodable"`, `None` otherwise).
- `marlin.klv.KlvError.variant`: the decode failure as a snake-case tag
  (`"truncated"`, `"length_overflow"`, `"bad_checksum"`,
  `"missing_checksum"`, `"bad_timestamp"`, `"bad_key"`), the way
  `EnvelopeError.variant` names its reason.
- `marlin.field.FieldState`: the decoded-field state, a frozen class with
  five variant classes `FieldState.Value(value)`,
  `FieldState.AtLeast(bound)`, `FieldState.NotAvailable()`,
  `FieldState.SenderError(code)` and `FieldState.Invalid(code)` (`code`
  is `None` for a field with no wire integer). Read one with
  `isinstance`, a `match` on the variant class, or the shared `kind`
  (`"value"`, `"at_least"`, `"not_available"`, `"sender_error"`,
  `"invalid"`), `value` and `value_or_bound` properties; equal states
  compare and hash alike. The stub declares `FieldState[T]`, so a later
  message stub can say `FieldState[float]`. No message class carries a
  field state yet. Trap: every state is truthy, `NotAvailable()`
  included, because a falsy not-available state would make `Value(0.0)`
  indistinguishable from it in `if msg.speed_over_ground:`; test the
  state, never the object.
- `marlin.dataclasses` mirrors `Value`, `AtLeast`, `NotAvailable`,
  `SenderError` and `Invalid` (generic frozen dataclasses with a `kind`
  literal and slots) and the `FieldState` union alias; `to_dataclass`
  converts a field state, and `asdict` yields
  `{"kind": "value", "value": 10.2}`.
- `marlin.ais.RateOfTurn`, `marlin.ais.Timestamp` and
  `marlin.ais.Type24BExtent`: frozen sum types with one variant class
  each per Rust variant (ADR-0009), matched by `isinstance` or `match`
  and compared and hashed by payload: `RateOfTurn.DegPerMin(deg_per_min)`
  / `RateOfTurn.NoIndicator(direction)`, `Timestamp.Second(second)` /
  `Timestamp.PositioningStatus(status)`, `Type24BExtent.Dimensions(dimensions)`
  / `Type24BExtent.MothershipMmsi(mmsi)`. `marlin.ais.PositioningStatus`
  is the new int-backed enum of the timestamp statuses (`MANUAL_INPUT = 61`,
  `DEAD_RECKONING = 62`, `INOPERATIVE = 63`, the wire codes).
  `marlin.dataclasses` mirrors the variant classes as dataclasses nested
  under a namespace class of the sum type's name
  (`marlin.dataclasses.Timestamp.Second`), with an enum payload as its
  integer value.
- Type 5 payloads of 420 and 422 bits decode to `StaticAndVoyageA`
  instead of raising `AisError`: the destination holds the whole
  characters transmitted and `dte` is `FieldState.NotAvailable()`.

### Changed (BREAKING)

Migration guide: [`docs/migration-0.3.md`](../../docs/migration-0.3.md).

- `marlin.klv.St0601` requires `timestamp_us` (`St0601()` is a
  `TypeError`), and every scaled-tag property (`sensor_latitude_degrees`,
  `slant_range_meters`, ...) and `version` read a `marlin.field.FieldState`
  instead of an `Optional` number: an omitted tag is
  `FieldState.NotAvailable()`, the ST 0601 sentinel on a signed tag is
  `FieldState.SenderError(code)` with the raw code (where the
  property used to read `None`), and a known tag with the wrong wire
  length is `FieldState.Invalid(None)` with its bytes kept in `unknown`
  (where only `unknown` used to carry it). A setter takes a `FieldState`,
  a bare number (`FieldState.Value`) or `None` (`FieldState.NotAvailable()`)
  and no longer clamps. The twenty `raw_*` properties are gone: the wire
  count survives only inside `FieldState.SenderError`, and a caller that
  wants counts calls `encode`. `repr(St0601(...))` prints the version in
  its variant form (`version=FieldState.Value(11)`, where it printed
  `version=11` or `version=None`).
- `marlin.klv.encode` raises `KlvEncodeError` for a value outside its
  tag's range or NaN (`"out_of_range"`), where it used to clamp (NaN to
  the range minimum), and for `FieldState.AtLeast`, `FieldState.Invalid`
  with a code, or a `FieldState.SenderError` whose code is not the tag's
  own sentinel (`"unencodable"`).
- `marlin.klv.decode` raises `KlvError` with `variant` `"bad_timestamp"`
  when Tag 2 is absent (the set used to decode with `timestamp_us` 0) or
  not 8 bytes, and `"missing_checksum"` when Tag 1 is absent or not 2
  bytes; `precision_timestamp` raises `"bad_timestamp"` for a
  wrong-length Tag 2. `"truncated"` now means only that the input ends
  early.
- Every `marlin.ais` message attribute the wire can leave without a value
  is a `marlin.field.FieldState` instead of an `Optional` value, a bare
  `int` or a bare enum: a not-available code is `FieldState.NotAvailable()`,
  an over-range code is `FieldState.AtLeast(bound)` with the bound in
  engineering units (speed 102.2 kn, altitude 4094 m, draught 25.5 m,
  dimensions 511 m / 63 m, where the bound used to pass through as a
  value), and a code ITU-R M.1371-5 leaves undefined is
  `FieldState.Invalid(code)` with the wire integer (a reserved navigation
  status, EPFD or manoeuvre code, a latitude beyond ±90° by a code other
  than not available, COG 3601..=4095, heading 360..=510, ETA month
  13..=15, hour 25..=31, minute 61..=63). Read a field with `.value`
  (`None` unless the state is `Value`), `isinstance` or `match`. Trap:
  `if body.vessel_name:` is always true, `NotAvailable()` included;
  port `if body.field is not None:` to `if body.field.value is not None:`
  or to a state check. The one-bit flags, `mmsi`, `radio_status`,
  `aton_status`, `AisVersion`, `AtonType` and `AltitudeSensor` stay plain.
  Per shape:
  - `timestamp` on Types 1/2/3, 9, 18, 19 and 21 is
    `FieldState[Timestamp]` instead of a raw `int`: `Timestamp.Second(s)`
    for 0..=59, `Timestamp.PositioningStatus(status)` for 61..=63,
    `NotAvailable()` for 60.
  - `PositionReportA.rate_of_turn` is `FieldState[RateOfTurn]`; the
    `turn_direction` sibling attribute is removed (it lived one release,
    0.2.0). Raw ±127 is `Value(RateOfTurn.NoIndicator(direction))`, -128
    is `NotAvailable()`.
  - `StaticDataB24B.extent: Type24BExtent` replaces the `dimensions` and
    `mothership_mmsi` sibling attributes. The constructor's `extent`
    defaults to `Type24BExtent.Dimensions(Dimensions())`.
  - `Dimensions` and `Eta` carry a `FieldState[int]` per member; their
    constructors accept `FieldState[int] | int | None` and default every
    member to `NotAvailable()`.
  - `ship_type` on Types 5, 19 and 24 Part B is `FieldState[int]`:
    `NotAvailable()` for 0, every other code a value.
  - `StaticAndVoyageA.dte` is `FieldState[bool]`: `NotAvailable()` on a
    420- or 422-bit payload (see Added), `Value(...)` from 423 bits.
    Types 9 and 19 keep `dte: bool`.
  - Message constructors accept `FieldState[T] | T | None` per
    field-state attribute, coerce a bare value to `Value` and `None` to
    `NotAvailable()`, default every such keyword to `NotAvailable()`,
    and raise the payload extraction error as is: `TypeError` for a
    payload of the wrong type, `OverflowError` for an integer outside
    the field's width. `Dimensions`, `Eta` and the extent default to
    every member not available.
  - Message `__repr__`s print the variant form
    (`sog=FieldState.AtLeast(102.2)`).
- `NavStatus.NOT_DEFINED`, `ManeuverIndicator.NOT_AVAILABLE`,
  `ManeuverIndicator.RESERVED` and `EpfdType.UNDEFINED` are removed: the
  not-available code is `FieldState.NotAvailable()` and a reserved code
  is `FieldState.Invalid(code)` on the message. A reserved code used to
  collapse onto `NOT_DEFINED` / `UNDEFINED`.
- `marlin.dataclasses` AIS mirrors carry the `FieldState` mirrors on the
  same attributes, with an enum payload stored as its integer value and
  the sum types as their nested variant mirrors; `asdict` yields
  `{"rate_of_turn": {"kind": "value", "value": {"deg_per_min": 19.7}}}`.
  `to_dataclass` finds a mirror by the binding class's qualified name, so
  `Type24BExtent.Dimensions` and `Dimensions` are told apart.
- Every `marlin.nmea` message attribute but `talker` is a
  `marlin.field.FieldState` instead of an `Optional` value or a bare
  enum: an empty field is `FieldState.NotAvailable()`, text the decoder
  cannot read is `FieldState.Invalid(None)`, an unnamed letter code is
  `FieldState.Invalid(byte)`, and a message with one bad field now
  decodes instead of raising `DecodeError`. Read a field with `.value`
  (`None` unless the state is `Value`), `isinstance` or `match`. Trap:
  `if msg.speed_knots:` is always true, `NotAvailable()` included; port
  `if msg.field is not None:` to `if msg.field.value is not None:` or
  to a state check. Per shape:
  - `Gga.fix_quality`, `Gll.status` and `Rmc.status` were bare enums
    and are `FieldState[...]` like every other field; an empty status
    byte is `NotAvailable()`, where it used to read as `DataStatus.VOID`.
  - `Ttm.reference_target` and `Tll.reference_target` are
    `FieldState[bool]`: `Value(False)` for an empty or absent field,
    never `NotAvailable()`.
  - `Psxn` and the two `Prdid` typed bodies carry `FieldState` on every
    attribute; a PSXN angle the data fields cannot produce is
    `Invalid(None)`.
  - Message constructors accept `FieldState[T] | T | None` per
    field-state attribute, coerce a bare value to `Value` and `None` to
    `NotAvailable()`, default every such keyword to `NotAvailable()`,
    and raise the payload extraction error as is: `TypeError` for a
    payload of the wrong type, `OverflowError` for an integer outside
    the field's width.
  - Message `__repr__`s print the variant form
    (`lat=FieldState.Value(48.5)`).
- `GgaFixQuality.INVALID` is renamed `NO_FIX`: the sender's own "no
  fix" is a value, and "invalid" now always means the decoder could
  not give the field a meaning.
- The catch-all members `TargetStatus.UNKNOWN`, `AngleReference.UNKNOWN`,
  `DistanceUnits.UNKNOWN` and `AcquisitionType.UNKNOWN` are removed, and
  an unnamed `VtgMode`, `DataStatus` or `RmcNavStatus` letter no longer
  collapses onto `NOT_VALID` / `VOID`: every unnamed letter is
  `FieldState.Invalid(byte)` on the message.
- `marlin.nmea.DecodeError` is raised for one reason only: the sentence
  has fewer fields than its decoder's floor. Its docstring says so.
- `marlin.dataclasses` NMEA mirrors carry the `FieldState` mirrors on
  the same attributes, with an enum payload stored as its integer value
  (`Gga.fix_quality: FieldState[int]`); `asdict` yields
  `{"speed_knots": {"kind": "value", "value": 22.4}}`.
- Python floor is 3.10 (`requires-python >= 3.10`, `abi3-py310` wheels):
  3.9 is past end of life and cannot parse the `match` statement the
  field-state documentation and tests use.

## [0.2.0] - 2026-10-08

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
  `turn_direction` is short-lived: the next breaking minor folds it back
  into `rate_of_turn` as a tagged value, and gives `mothership_mmsi`
  below the same treatment.
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

- `AisParser.tick(now_ms)` evicts reassembly partials past their timeout
  at once instead of on the next `next_message()`; each eviction still
  surfaces as a `ReassemblyError` from a later `next_message()`, and the
  manual clock still starts at 0, so callers see the same sequence of
  results. The binding no longer keeps its own copy of the manual clock
  (`marlin-ais` ADR-0004).
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
