"""Frozen dataclass mirrors of marlin's typed messages.

Useful for pattern matching and any tooling that consumes plain Python
dataclasses (msgspec, pydantic-via-validators, attrs adapters). For
JSON, pair `dataclasses.asdict()` with a `default=` handler — several
mirrors carry `bytes` fields (`RawSentence.fields`, `Psxn.token`,
`PrdidRaw.fields`, `Other.raw_payload`) which `json.dumps` cannot
encode natively:

    json.dumps(
        dataclasses.asdict(to_dataclass(msg)),
        default=lambda v: v.hex() if isinstance(v, bytes) else str(v),
    )

The runtime PyO3 message types are immutable too, but they are foreign
classes — `dataclasses.asdict()` cannot introspect them. Convert with
`to_dataclass(msg)` to get a frozen-dataclass equivalent.

Enum-typed fields (GgaFixQuality, NavStatus, EpfdType, etc.) are stored
as their integer values for JSON-friendly output, inside the field-state
mirror where the binding class carries a `FieldState`.

A dataclass mirror has the same qualified name and the same field names
as its binding class; `to_dataclass` relies on both to convert without a
per-class function. A variant class of a sum type (`RateOfTurn.DegPerMin`)
is mirrored by a dataclass nested under a namespace class of the sum
type's name. To add a message, write its dataclass here and export it.
`tests/unit/test_dataclass_agreement.py` checks the pairing.
"""

from __future__ import annotations

from dataclasses import dataclass, field, fields, is_dataclass
from typing import Generic, Literal, Optional, SupportsInt, Tuple, TypeVar, Union, get_args

_T = TypeVar("_T")


# ---------- field state ----------

# Mirrors of the five `marlin.field.FieldState` variant classes. `kind` is
# the state tag the binding class reports, kept keyword-only so the
# generated `__match_args__` is the payload alone, as on the binding class;
# `asdict` yields `{"kind": "value", "value": 10.2}` for a field.


@dataclass(frozen=True, slots=True)
class Value(Generic[_T]):
    """Dataclass mirror of marlin.field.FieldState.Value."""

    value: _T
    kind: Literal["value"] = field(default="value", kw_only=True)


@dataclass(frozen=True, slots=True)
class AtLeast(Generic[_T]):
    """Dataclass mirror of marlin.field.FieldState.AtLeast."""

    bound: _T
    kind: Literal["at_least"] = field(default="at_least", kw_only=True)


@dataclass(frozen=True, slots=True)
class NotAvailable(Generic[_T]):
    """Dataclass mirror of marlin.field.FieldState.NotAvailable."""

    kind: Literal["not_available"] = field(default="not_available", kw_only=True)


@dataclass(frozen=True, slots=True)
class SenderError(Generic[_T]):
    """Dataclass mirror of marlin.field.FieldState.SenderError."""

    code: int
    kind: Literal["sender_error"] = field(default="sender_error", kw_only=True)


@dataclass(frozen=True, slots=True)
class Invalid(Generic[_T]):
    """Dataclass mirror of marlin.field.FieldState.Invalid.

    `code` is None for a field with no wire integer.
    """

    code: Optional[int]
    kind: Literal["invalid"] = field(default="invalid", kw_only=True)


FieldState = Union[Value[_T], AtLeast[_T], NotAvailable[_T], SenderError[_T], Invalid[_T]]


# ---------- shared value types ----------


@dataclass(frozen=True)
class UtcTime:
    """Dataclass mirror of marlin.nmea.UtcTime."""

    hour: int
    minute: int
    second: int
    millisecond: int


@dataclass(frozen=True)
class UtcDate:
    """Dataclass mirror of marlin.nmea.UtcDate (RMC `ddmmyy` field).

    `year_yy` is the raw two-digit year — caller applies century resolution.
    """

    day: int
    month: int
    year_yy: int


@dataclass(frozen=True)
class Eta:
    """Dataclass mirror of marlin.ais.Eta. Each member is a `FieldState[int]`."""

    month: FieldState[int]
    day: FieldState[int]
    hour: FieldState[int]
    minute: FieldState[int]


@dataclass(frozen=True)
class Dimensions:
    """Dataclass mirror of marlin.ais.Dimensions.

    Each member is a `FieldState[int]`; the field maximum is the over-range
    bound `AtLeast(511)` / `AtLeast(63)`.
    """

    to_bow_m: FieldState[int]
    to_stern_m: FieldState[int]
    to_port_m: FieldState[int]
    to_starboard_m: FieldState[int]


# ---------- AIS sum types in field position ----------

# Mirrors of the variant classes of `marlin.ais.RateOfTurn`, `Timestamp`
# and `Type24BExtent`, nested under a namespace class named like the sum
# type so that `Timestamp.PositioningStatus` and `Type24BExtent.Dimensions`
# do not collide with the `PositioningStatus` enum and the `Dimensions`
# value type. An enum payload is the member's integer value. The converter
# finds a mirror by the binding class's qualified name.

# `Type24BExtent.Dimensions` shadows the value type inside its namespace.
_Dimensions = Dimensions


class RateOfTurn:
    """Namespace of the mirrors of marlin.ais.RateOfTurn's variant classes."""

    @dataclass(frozen=True, slots=True)
    class DegPerMin:
        """Dataclass mirror of marlin.ais.RateOfTurn.DegPerMin."""

        deg_per_min: float

    @dataclass(frozen=True, slots=True)
    class NoIndicator:
        """Dataclass mirror of marlin.ais.RateOfTurn.NoIndicator.

        `direction` is the `TurnDirection` member's integer value.
        """

        direction: int


class Timestamp:
    """Namespace of the mirrors of marlin.ais.Timestamp's variant classes."""

    @dataclass(frozen=True, slots=True)
    class Second:
        """Dataclass mirror of marlin.ais.Timestamp.Second."""

        second: int

    @dataclass(frozen=True, slots=True)
    class PositioningStatus:
        """Dataclass mirror of marlin.ais.Timestamp.PositioningStatus.

        `status` is the `PositioningStatus` member's integer value, the
        timestamp wire code 61, 62 or 63.
        """

        status: int


class Type24BExtent:
    """Namespace of the mirrors of marlin.ais.Type24BExtent's variant classes."""

    @dataclass(frozen=True, slots=True)
    class Dimensions:
        """Dataclass mirror of marlin.ais.Type24BExtent.Dimensions."""

        dimensions: _Dimensions

    @dataclass(frozen=True, slots=True)
    class MothershipMmsi:
        """Dataclass mirror of marlin.ais.Type24BExtent.MothershipMmsi."""

        mmsi: int


# ---------- envelope dataclass mirror ----------


@dataclass(frozen=True)
class RawSentence:
    """Dataclass mirror of marlin.envelope.RawSentence.

    `fields` uses Tuple to preserve frozen-ness (lists are mutable).
    """

    start_delimiter: bytes
    talker: Optional[bytes]
    sentence_type: str
    fields: Tuple[bytes, ...]
    tag_block: Optional[bytes]
    checksum_ok: bool
    raw: bytes


# ---------- NMEA dataclass mirrors ----------

# Every message field but `talker` is a `FieldState` mirror. An enum payload
# is stored as the member's integer value (`int(member)`) for JSON-friendly
# output, so `fix_quality` is `FieldState[int]`, not
# `FieldState[GgaFixQuality]`.


@dataclass(frozen=True)
class Gga:
    """Dataclass mirror of marlin.nmea.Gga.

    `fix_quality` carries the enum member's int value for JSON
    compatibility.
    """

    talker: Optional[bytes]
    utc: FieldState[UtcTime]
    latitude_deg: FieldState[float]
    longitude_deg: FieldState[float]
    fix_quality: FieldState[int]
    satellites_used: FieldState[int]
    hdop: FieldState[float]
    altitude_m: FieldState[float]
    geoid_separation_m: FieldState[float]
    dgps_age_s: FieldState[float]
    dgps_station_id: FieldState[int]


@dataclass(frozen=True)
class Vtg:
    """Dataclass mirror of marlin.nmea.Vtg.

    `mode` carries the enum member's int value for JSON compatibility.
    """

    talker: Optional[bytes]
    course_true_deg: FieldState[float]
    course_magnetic_deg: FieldState[float]
    speed_knots: FieldState[float]
    speed_kmh: FieldState[float]
    mode: FieldState[int]


@dataclass(frozen=True)
class Hdt:
    """Dataclass mirror of marlin.nmea.Hdt."""

    talker: Optional[bytes]
    heading_true_deg: FieldState[float]


@dataclass(frozen=True)
class Rmc:
    """Dataclass mirror of marlin.nmea.Rmc.

    `status`, `mode` and `nav_status` carry the enum member's int value
    for JSON compatibility. `mode` and `nav_status` are not
    available on sentences that predate NMEA 2.3 / 4.10 respectively.
    """

    talker: Optional[bytes]
    utc: FieldState[UtcTime]
    status: FieldState[int]
    latitude_deg: FieldState[float]
    longitude_deg: FieldState[float]
    speed_knots: FieldState[float]
    course_true_deg: FieldState[float]
    date: FieldState[UtcDate]
    magnetic_variation_deg: FieldState[float]
    mode: FieldState[int]
    nav_status: FieldState[int]


@dataclass(frozen=True)
class Gll:
    """Dataclass mirror of marlin.nmea.Gll.

    `status` and `mode` carry the enum member's int value for JSON
    compatibility.
    """

    talker: Optional[bytes]
    latitude_deg: FieldState[float]
    longitude_deg: FieldState[float]
    utc: FieldState[UtcTime]
    status: FieldState[int]
    mode: FieldState[int]


@dataclass(frozen=True)
class Hdg:
    """Dataclass mirror of marlin.nmea.Hdg."""

    talker: Optional[bytes]
    heading_magnetic_deg: FieldState[float]
    deviation_deg: FieldState[float]
    variation_deg: FieldState[float]


@dataclass(frozen=True)
class Ttm:
    """Dataclass mirror of marlin.nmea.Ttm.

    `bearing_reference`, `course_reference`, `units`, `status` and
    `acquisition` carry the enum member's int value for JSON
    compatibility.
    """

    talker: Optional[bytes]
    target_number: FieldState[int]
    distance: FieldState[float]
    bearing_deg: FieldState[float]
    bearing_reference: FieldState[int]
    speed: FieldState[float]
    course_deg: FieldState[float]
    course_reference: FieldState[int]
    cpa: FieldState[float]
    tcpa: FieldState[float]
    units: FieldState[int]
    name: FieldState[str]
    status: FieldState[int]
    reference_target: FieldState[bool]
    utc_time: FieldState[UtcTime]
    acquisition: FieldState[int]


@dataclass(frozen=True)
class Tll:
    """Dataclass mirror of marlin.nmea.Tll.

    `status` carries the enum member's int value for JSON compatibility.
    """

    talker: Optional[bytes]
    target_number: FieldState[int]
    latitude_deg: FieldState[float]
    longitude_deg: FieldState[float]
    name: FieldState[str]
    utc_time: FieldState[UtcTime]
    status: FieldState[int]
    reference_target: FieldState[bool]


@dataclass(frozen=True)
class Unknown:
    """Dataclass mirror of marlin.nmea.Unknown."""

    talker: Optional[bytes]
    sentence_type: str


@dataclass(frozen=True)
class Psxn:
    """Dataclass mirror of marlin.nmea.Psxn."""

    id: FieldState[int]
    token: FieldState[bytes]
    roll_deg: FieldState[float]
    pitch_deg: FieldState[float]
    heave_m: FieldState[float]


@dataclass(frozen=True)
class PrdidPitchRollHeading:
    """Dataclass mirror of marlin.nmea.PrdidPitchRollHeading."""

    pitch_deg: FieldState[float]
    roll_deg: FieldState[float]
    heading_deg: FieldState[float]


@dataclass(frozen=True)
class PrdidRollPitchHeading:
    """Dataclass mirror of marlin.nmea.PrdidRollPitchHeading."""

    roll_deg: FieldState[float]
    pitch_deg: FieldState[float]
    heading_deg: FieldState[float]


@dataclass(frozen=True)
class PrdidRaw:
    """Dataclass mirror of marlin.nmea.PrdidRaw.

    `fields` uses Tuple to preserve frozen-ness.
    """

    fields: Tuple[bytes, ...]


@dataclass(frozen=True)
class Prdid:
    """Dataclass mirror of marlin.nmea.Prdid (tagged union).

    `variant` is the stable snake-case tag string (e.g. "pitch_roll_heading").
    `body` is one of PrdidPitchRollHeading, PrdidRollPitchHeading, or PrdidRaw.
    """

    variant: str
    body: Union[PrdidPitchRollHeading, PrdidRollPitchHeading, PrdidRaw]


# ---------- AIS dataclass mirrors ----------

# Every attribute the wire can leave without a value is a `FieldState`
# mirror; an enum payload is stored as the member's integer value.


@dataclass(frozen=True)
class PositionReportA:
    """Dataclass mirror of marlin.ais.PositionReportA (Types 1/2/3).

    `navigation_status` and `special_maneuver` carry the enum member's
    int value (the wire code). `rate_of_turn` and `timestamp` carry the
    `RateOfTurn` and `Timestamp` variant mirrors.
    """

    mmsi: int
    navigation_status: FieldState[int]
    rate_of_turn: FieldState[Union[RateOfTurn.DegPerMin, RateOfTurn.NoIndicator]]
    speed_over_ground: FieldState[float]
    position_accuracy: bool
    longitude_deg: FieldState[float]
    latitude_deg: FieldState[float]
    course_over_ground: FieldState[float]
    true_heading: FieldState[int]
    timestamp: FieldState[Union[Timestamp.Second, Timestamp.PositioningStatus]]
    special_maneuver: FieldState[int]
    raim: bool
    radio_status: int


@dataclass(frozen=True)
class StaticAndVoyageA:
    """Dataclass mirror of marlin.ais.StaticAndVoyageA (Type 5).

    `ais_version` and the `epfd` payload are int (wire codes).
    `dimensions` and `eta` are always present (non-Optional) per the Rust
    type, each member a `FieldState`.
    """

    mmsi: int
    ais_version: int
    imo_number: FieldState[int]
    call_sign: FieldState[str]
    vessel_name: FieldState[str]
    ship_type: FieldState[int]
    dimensions: Dimensions
    epfd: FieldState[int]
    eta: Eta
    draught_m: FieldState[float]
    destination: FieldState[str]
    dte: FieldState[bool]


@dataclass(frozen=True)
class SarAircraftPositionReport:
    """Dataclass mirror of marlin.ais.SarAircraftPositionReport (Type 9).

    `altitude_m` and `speed_over_ground` are whole metres / whole knots;
    an over-range code is the `AtLeast` mirror. `altitude_sensor` is int
    (wire code: GNSS = 0, BAROMETRIC = 1).
    """

    mmsi: int
    altitude_m: FieldState[int]
    speed_over_ground: FieldState[int]
    position_accuracy: bool
    longitude_deg: FieldState[float]
    latitude_deg: FieldState[float]
    course_over_ground: FieldState[float]
    timestamp: FieldState[Union[Timestamp.Second, Timestamp.PositioningStatus]]
    altitude_sensor: int
    dte: bool
    assigned_flag: bool
    raim: bool
    radio_status: int


@dataclass(frozen=True)
class PositionReportB:
    """Dataclass mirror of marlin.ais.PositionReportB (Type 18)."""

    mmsi: int
    speed_over_ground: FieldState[float]
    position_accuracy: bool
    longitude_deg: FieldState[float]
    latitude_deg: FieldState[float]
    course_over_ground: FieldState[float]
    true_heading: FieldState[int]
    timestamp: FieldState[Union[Timestamp.Second, Timestamp.PositioningStatus]]
    class_b_cs_flag: bool
    class_b_display_flag: bool
    class_b_dsc_flag: bool
    class_b_band_flag: bool
    class_b_message22_flag: bool
    assigned_flag: bool
    raim: bool
    radio_status: int


@dataclass(frozen=True)
class ExtendedPositionReportB:
    """Dataclass mirror of marlin.ais.ExtendedPositionReportB (Type 19).

    The `epfd` payload is int (wire code). `dimensions` is always present.
    """

    mmsi: int
    speed_over_ground: FieldState[float]
    position_accuracy: bool
    longitude_deg: FieldState[float]
    latitude_deg: FieldState[float]
    course_over_ground: FieldState[float]
    true_heading: FieldState[int]
    timestamp: FieldState[Union[Timestamp.Second, Timestamp.PositioningStatus]]
    vessel_name: FieldState[str]
    ship_type: FieldState[int]
    dimensions: Dimensions
    epfd: FieldState[int]
    raim: bool
    dte: bool
    assigned_flag: bool


@dataclass(frozen=True)
class AidToNavigationReport:
    """Dataclass mirror of marlin.ais.AidToNavigationReport (Type 21).

    `aton_type` and the `epfd` payload are int (wire codes). `name` is
    the joined and trimmed 20 + up to 14 character name; `dimensions` is
    always present (all not available for virtual AtoN and reference
    points).
    """

    mmsi: int
    aton_type: int
    name: FieldState[str]
    position_accuracy: bool
    longitude_deg: FieldState[float]
    latitude_deg: FieldState[float]
    dimensions: Dimensions
    epfd: FieldState[int]
    timestamp: FieldState[Union[Timestamp.Second, Timestamp.PositioningStatus]]
    off_position: bool
    aton_status: int
    raim: bool
    virtual_aton: bool
    assigned_flag: bool


@dataclass(frozen=True)
class StaticDataB24A:
    """Dataclass mirror of marlin.ais.StaticDataB24A (Type 24 Part A)."""

    mmsi: int
    vessel_name: FieldState[str]


@dataclass(frozen=True)
class StaticDataB24B:
    """Dataclass mirror of marlin.ais.StaticDataB24B (Type 24 Part B).

    `extent` is the `Type24BExtent` variant mirror: dimensions, or the
    mother ship's MMSI for an auxiliary craft (MMSI `98MIDxxxx`). The
    `epfd` payload is int (wire code).
    """

    mmsi: int
    ship_type: FieldState[int]
    vendor_id: FieldState[str]
    call_sign: FieldState[str]
    extent: Union[Type24BExtent.Dimensions, Type24BExtent.MothershipMmsi]
    epfd: FieldState[int]


@dataclass(frozen=True)
class Other:
    """Dataclass mirror of marlin.ais.Other (catch-all for un-decoded msg_type)."""

    msg_type: int
    raw_payload: bytes
    total_bits: int


AisMessageBody = Union[
    PositionReportA,
    StaticAndVoyageA,
    SarAircraftPositionReport,
    PositionReportB,
    ExtendedPositionReportB,
    AidToNavigationReport,
    StaticDataB24A,
    StaticDataB24B,
    Other,
]


@dataclass(frozen=True)
class AisMessage:
    """Dataclass mirror of marlin.ais.AisMessage."""

    is_own_ship: bool
    type_tag: str
    body: AisMessageBody


# ---------- type aliases ----------

NmeaMessage = Union[Gga, Gll, Hdg, Hdt, Rmc, Tll, Ttm, Vtg, Psxn, Prdid, Unknown]


# ---------- conversion ----------

# Modules whose classes are binding classes. A value from one of them is
# either a class with a dataclass mirror of the same name, or an enum.
_BINDING_MODULES = ("marlin.ais", "marlin.envelope", "marlin.field", "marlin.nmea")


def to_dataclass(msg: object) -> object:
    """Convert a marlin runtime message into its frozen dataclass mirror.

    Accepts any of:

    - Envelope: ``marlin.envelope.RawSentence``
    - NMEA: ``Gga``, ``Gll``, ``Hdg``, ``Hdt``, ``Rmc``, ``Tll``, ``Ttm``,
      ``Vtg``, ``Psxn``, ``Prdid``, ``Unknown``
    - AIS: ``AisMessage`` (the wrapper) or any body variant directly
    - A value type nested in one of those, such as ``UtcTime``,
      ``Dimensions`` or ``Eta``
    - A field state: any ``marlin.field.FieldState`` variant

    Raises ``TypeError`` for anything else, enum members included.
    """
    mirror = _mirror_of(msg)
    if mirror is None:
        raise TypeError(
            f"to_dataclass: unrecognised marlin message type {type(msg).__qualname__!r}"
        )
    if mirror is AisMessage:
        # `AisMessage.body` is the one field the binding class does not type
        # check, so a hand-built wrapper can hold anything.
        body = getattr(msg, "body")
        if _mirror_of(body) not in get_args(AisMessageBody):
            raise TypeError(
                f"to_dataclass: unrecognised AIS body type {type(body).__qualname__!r}"
            )
    return _convert(msg)


def _mirror_of(value: object) -> Optional[type]:
    """The dataclass mirror of `value`'s binding class, if any.

    A mirror is found by the binding class's qualified name: a nested
    variant class (`Type24BExtent.Dimensions`) is the dataclass of the same
    name under the namespace class of the same name here. The `FieldState`
    mirrors are flat because `FieldState` is the union alias, so a name
    the path does not resolve is looked up bare (`Value`).
    """
    binding_class = type(value)
    if binding_class.__module__ not in _BINDING_MODULES:
        return None
    mirror = _resolve(binding_class.__qualname__.split("."))
    if mirror is None:
        mirror = globals().get(binding_class.__name__)
    if isinstance(mirror, type) and is_dataclass(mirror):
        return mirror
    return None


def _resolve(path: list[str]) -> object:
    """Walk `path` from this module's globals through class attributes."""
    head, *rest = path
    node: object = globals().get(head)
    for name in rest:
        if not isinstance(node, type):
            return None
        node = vars(node).get(name)
    return node


def _convert(value: object) -> object:
    """Convert one value, descending into nested binding classes.

    A binding class becomes its dataclass mirror, field by field. Any other
    value from a binding module is an enum and becomes its integer. Everything
    else (``None``, numbers, ``str``, ``bytes``, tuples of those) passes through.
    """
    mirror = _mirror_of(value)
    if mirror is not None:
        return mirror(
            **{f.name: _convert(getattr(value, f.name)) for f in fields(mirror)}
        )
    if type(value).__module__ in _BINDING_MODULES:
        if not isinstance(value, SupportsInt):
            raise TypeError(
                f"to_dataclass: {type(value).__qualname__!r} has no dataclass mirror"
                " and is not an enum"
            )
        return int(value)
    return value


__all__ = [
    "AidToNavigationReport",
    "AisMessage",
    "AisMessageBody",
    "AtLeast",
    "Dimensions",
    "Eta",
    "ExtendedPositionReportB",
    "FieldState",
    "Gga",
    "Gll",
    "Hdg",
    "Hdt",
    "Invalid",
    "NmeaMessage",
    "NotAvailable",
    "Other",
    "PositionReportA",
    "PositionReportB",
    "Prdid",
    "PrdidPitchRollHeading",
    "PrdidRaw",
    "PrdidRollPitchHeading",
    "Psxn",
    "RateOfTurn",
    "RawSentence",
    "Rmc",
    "SarAircraftPositionReport",
    "SenderError",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "Timestamp",
    "Tll",
    "Ttm",
    "Type24BExtent",
    "Unknown",
    "UtcDate",
    "UtcTime",
    "Value",
    "Vtg",
    "to_dataclass",
]
