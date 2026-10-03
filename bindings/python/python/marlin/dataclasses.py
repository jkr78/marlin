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
as their integer values for JSON-friendly output.

A dataclass mirror has the same name and the same field names as its
binding class; `to_dataclass` relies on both to convert without a
per-class function. To add a message, write its dataclass here and
export it. `tests/unit/test_dataclass_agreement.py` checks the pairing.
"""

from __future__ import annotations

from dataclasses import dataclass, fields, is_dataclass
from typing import Optional, SupportsInt, Tuple, Union


# ---------- shared value types ----------


@dataclass(frozen=True)
class UtcTime:
    """Mirror of marlin.nmea.UtcTime."""

    hour: int
    minute: int
    second: int
    millisecond: int


@dataclass(frozen=True)
class UtcDate:
    """Mirror of marlin.nmea.UtcDate (RMC `ddmmyy` field).

    `year_yy` is the raw two-digit year — caller applies century resolution.
    """

    day: int
    month: int
    year_yy: int


@dataclass(frozen=True)
class Eta:
    """Mirror of marlin.ais.Eta. All fields are Optional[int]."""

    month: Optional[int]
    day: Optional[int]
    hour: Optional[int]
    minute: Optional[int]


@dataclass(frozen=True)
class Dimensions:
    """Mirror of marlin.ais.Dimensions. All fields are Optional[int]."""

    to_bow_m: Optional[int]
    to_stern_m: Optional[int]
    to_port_m: Optional[int]
    to_starboard_m: Optional[int]


# ---------- envelope mirror ----------


@dataclass(frozen=True)
class RawSentence:
    """Mirror of marlin.envelope.RawSentence.

    `fields` uses Tuple to preserve frozen-ness (lists are mutable).
    """

    start_delimiter: bytes
    talker: Optional[bytes]
    sentence_type: str
    fields: Tuple[bytes, ...]
    tag_block: Optional[bytes]
    checksum_ok: bool
    raw: bytes


# ---------- NMEA message mirrors ----------


@dataclass(frozen=True)
class Gga:
    """Mirror of marlin.nmea.Gga.

    `fix_quality` is stored as an int (wire value) for JSON compatibility.
    """

    talker: Optional[bytes]
    utc: Optional[UtcTime]
    latitude_deg: Optional[float]
    longitude_deg: Optional[float]
    fix_quality: int
    satellites_used: Optional[int]
    hdop: Optional[float]
    altitude_m: Optional[float]
    geoid_separation_m: Optional[float]
    dgps_age_s: Optional[float]
    dgps_station_id: Optional[int]


@dataclass(frozen=True)
class Vtg:
    """Mirror of marlin.nmea.Vtg.

    `mode` is stored as Optional[int] (wire value) for JSON compatibility.
    """

    talker: Optional[bytes]
    course_true_deg: Optional[float]
    course_magnetic_deg: Optional[float]
    speed_knots: Optional[float]
    speed_kmh: Optional[float]
    mode: Optional[int]


@dataclass(frozen=True)
class Hdt:
    """Mirror of marlin.nmea.Hdt."""

    talker: Optional[bytes]
    heading_true_deg: Optional[float]


@dataclass(frozen=True)
class Rmc:
    """Mirror of marlin.nmea.Rmc.

    `status`, `mode`, and `nav_status` are stored as int (wire values) for
    JSON compatibility. `mode` and `nav_status` are Optional because they
    only exist on NMEA 2.3+ / 4.10+ sentences respectively.
    """

    talker: Optional[bytes]
    utc: Optional[UtcTime]
    status: int
    latitude_deg: Optional[float]
    longitude_deg: Optional[float]
    speed_knots: Optional[float]
    course_true_deg: Optional[float]
    date: Optional[UtcDate]
    magnetic_variation_deg: Optional[float]
    mode: Optional[int]
    nav_status: Optional[int]


@dataclass(frozen=True)
class Gll:
    """Mirror of marlin.nmea.Gll.

    `status` is stored as int (wire value) for JSON compatibility.
    """

    talker: Optional[bytes]
    latitude_deg: Optional[float]
    longitude_deg: Optional[float]
    utc: Optional[UtcTime]
    status: int
    mode: Optional[int]


@dataclass(frozen=True)
class Hdg:
    """Mirror of marlin.nmea.Hdg."""

    talker: Optional[bytes]
    heading_magnetic_deg: Optional[float]
    deviation_deg: Optional[float]
    variation_deg: Optional[float]


@dataclass(frozen=True)
class Ttm:
    """Mirror of marlin.nmea.Ttm.

    `bearing_reference`, `course_reference`, `units`, `status`, and
    `acquisition` are stored as Optional[int] (wire values) for JSON
    compatibility.
    """

    talker: Optional[bytes]
    target_number: Optional[int]
    distance: Optional[float]
    bearing_deg: Optional[float]
    bearing_reference: Optional[int]
    speed: Optional[float]
    course_deg: Optional[float]
    course_reference: Optional[int]
    cpa: Optional[float]
    tcpa: Optional[float]
    units: Optional[int]
    name: Optional[str]
    status: Optional[int]
    reference_target: bool
    utc_time: Optional[UtcTime]
    acquisition: Optional[int]


@dataclass(frozen=True)
class Tll:
    """Mirror of marlin.nmea.Tll.

    `status` is stored as Optional[int] (wire value) for JSON compatibility.
    """

    talker: Optional[bytes]
    target_number: Optional[int]
    latitude_deg: Optional[float]
    longitude_deg: Optional[float]
    name: Optional[str]
    utc_time: Optional[UtcTime]
    status: Optional[int]
    reference_target: bool


@dataclass(frozen=True)
class Unknown:
    """Mirror of marlin.nmea.Unknown."""

    talker: Optional[bytes]
    sentence_type: str


@dataclass(frozen=True)
class Psxn:
    """Mirror of marlin.nmea.Psxn."""

    id: Optional[int]
    token: Optional[bytes]
    roll_deg: Optional[float]
    pitch_deg: Optional[float]
    heave_m: Optional[float]


@dataclass(frozen=True)
class PrdidPitchRollHeading:
    """Mirror of marlin.nmea.PrdidPitchRollHeading."""

    pitch_deg: Optional[float]
    roll_deg: Optional[float]
    heading_deg: Optional[float]


@dataclass(frozen=True)
class PrdidRollPitchHeading:
    """Mirror of marlin.nmea.PrdidRollPitchHeading."""

    roll_deg: Optional[float]
    pitch_deg: Optional[float]
    heading_deg: Optional[float]


@dataclass(frozen=True)
class PrdidRaw:
    """Mirror of marlin.nmea.PrdidRaw.

    `fields` uses Tuple to preserve frozen-ness.
    """

    fields: Tuple[bytes, ...]


@dataclass(frozen=True)
class Prdid:
    """Mirror of marlin.nmea.Prdid (tagged union).

    `variant` is the stable snake-case tag string (e.g. "pitch_roll_heading").
    `body` is one of PrdidPitchRollHeading, PrdidRollPitchHeading, or PrdidRaw.
    """

    variant: str
    body: Union[PrdidPitchRollHeading, PrdidRollPitchHeading, PrdidRaw]


# ---------- AIS message mirrors ----------


@dataclass(frozen=True)
class PositionReportA:
    """Mirror of marlin.ais.PositionReportA (Types 1/2/3).

    `navigation_status` and `special_maneuver` are stored as int (wire values).
    `turn_direction` is stored as int too, but it is the `TurnDirection`
    enum value (RIGHT = 0, LEFT = 1), not a wire code: on the wire the
    status is raw ROT ±127. At most one of `rate_of_turn` and
    `turn_direction` is set; both are None for the −128 sentinel.
    """

    mmsi: int
    navigation_status: int
    rate_of_turn: Optional[float]
    turn_direction: Optional[int]
    speed_over_ground: Optional[float]
    position_accuracy: bool
    longitude_deg: Optional[float]
    latitude_deg: Optional[float]
    course_over_ground: Optional[float]
    true_heading: Optional[int]
    timestamp: int
    special_maneuver: int
    raim: bool
    radio_status: int


@dataclass(frozen=True)
class StaticAndVoyageA:
    """Mirror of marlin.ais.StaticAndVoyageA (Type 5).

    `ais_version` and `epfd` are stored as int (wire values).
    `dimensions` and `eta` are always present (non-Optional) per the Rust type.
    """

    mmsi: int
    ais_version: int
    imo_number: Optional[int]
    call_sign: Optional[str]
    vessel_name: Optional[str]
    ship_type: int
    dimensions: Dimensions
    epfd: int
    eta: Eta
    draught_m: Optional[float]
    destination: Optional[str]
    dte: bool


@dataclass(frozen=True)
class SarAircraftPositionReport:
    """Mirror of marlin.ais.SarAircraftPositionReport (Type 9).

    `altitude_m` and `speed_over_ground` are whole metres / whole knots;
    None is the not-available code and over-range codes pass through.
    `altitude_sensor` is stored as int (wire value: GNSS = 0, BAROMETRIC = 1).
    """

    mmsi: int
    altitude_m: Optional[int]
    speed_over_ground: Optional[int]
    position_accuracy: bool
    longitude_deg: Optional[float]
    latitude_deg: Optional[float]
    course_over_ground: Optional[float]
    timestamp: int
    altitude_sensor: int
    dte: bool
    assigned_flag: bool
    raim: bool
    radio_status: int


@dataclass(frozen=True)
class PositionReportB:
    """Mirror of marlin.ais.PositionReportB (Type 18)."""

    mmsi: int
    speed_over_ground: Optional[float]
    position_accuracy: bool
    longitude_deg: Optional[float]
    latitude_deg: Optional[float]
    course_over_ground: Optional[float]
    true_heading: Optional[int]
    timestamp: int
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
    """Mirror of marlin.ais.ExtendedPositionReportB (Type 19).

    `epfd` is stored as int (wire value). `dimensions` is always present.
    """

    mmsi: int
    speed_over_ground: Optional[float]
    position_accuracy: bool
    longitude_deg: Optional[float]
    latitude_deg: Optional[float]
    course_over_ground: Optional[float]
    true_heading: Optional[int]
    timestamp: int
    vessel_name: Optional[str]
    ship_type: int
    dimensions: Dimensions
    epfd: int
    raim: bool
    dte: bool
    assigned_flag: bool


@dataclass(frozen=True)
class AidToNavigationReport:
    """Mirror of marlin.ais.AidToNavigationReport (Type 21).

    `aton_type` and `epfd` are stored as int (wire values). `name` is the
    joined and trimmed 20 + up to 14 character name; `dimensions` is
    always present (all-None for virtual AtoN and reference points).
    """

    mmsi: int
    aton_type: int
    name: Optional[str]
    position_accuracy: bool
    longitude_deg: Optional[float]
    latitude_deg: Optional[float]
    dimensions: Dimensions
    epfd: int
    timestamp: int
    off_position: bool
    aton_status: int
    raim: bool
    virtual_aton: bool
    assigned_flag: bool


@dataclass(frozen=True)
class StaticDataB24A:
    """Mirror of marlin.ais.StaticDataB24A (Type 24 Part A)."""

    mmsi: int
    vessel_name: Optional[str]


@dataclass(frozen=True)
class StaticDataB24B:
    """Mirror of marlin.ais.StaticDataB24B (Type 24 Part B).

    Exactly one of `dimensions` and `mothership_mmsi` is set on parser
    output: the latter for an auxiliary craft (MMSI `98MIDxxxx`), whose
    30 extent bits carry the mother ship's MMSI instead of dimensions.
    `epfd` is stored as int (wire value).
    """

    mmsi: int
    ship_type: int
    vendor_id: Optional[str]
    call_sign: Optional[str]
    dimensions: Optional[Dimensions]
    mothership_mmsi: Optional[int]
    epfd: int


@dataclass(frozen=True)
class Other:
    """Mirror of marlin.ais.Other (catch-all for un-decoded msg_type)."""

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
    """Mirror of marlin.ais.AisMessage."""

    is_own_ship: bool
    type_tag: str
    body: AisMessageBody


# ---------- type aliases ----------

NmeaMessage = Union[Gga, Gll, Hdg, Hdt, Rmc, Tll, Ttm, Vtg, Psxn, Prdid, Unknown]


# ---------- conversion ----------

# Modules whose classes are binding classes. A value from one of them is
# either a class with a dataclass mirror of the same name, or an enum.
_BINDING_MODULES = ("marlin.ais", "marlin.envelope", "marlin.nmea")


def to_dataclass(msg: object) -> object:
    """Convert a marlin runtime message into its frozen dataclass mirror.

    Accepts any of:

    - Envelope: ``marlin.envelope.RawSentence``
    - NMEA: ``Gga``, ``Gll``, ``Hdg``, ``Hdt``, ``Rmc``, ``Tll``, ``Ttm``,
      ``Vtg``, ``Psxn``, ``Prdid``, ``Unknown``
    - AIS: ``AisMessage`` (the wrapper) or any body variant directly
    - A value type nested in one of those, such as ``UtcTime``,
      ``Dimensions`` or ``Eta``

    Raises ``TypeError`` for anything else, enum members included.
    """
    if _mirror_of(msg) is None:
        raise TypeError(
            f"to_dataclass: unrecognised marlin message type {type(msg).__qualname__!r}"
        )
    return _convert(msg)


def _mirror_of(value: object) -> Optional[type]:
    """The dataclass mirror named like `value`'s binding class, if any."""
    binding_class = type(value)
    if binding_class.__module__ not in _BINDING_MODULES:
        return None
    mirror = globals().get(binding_class.__name__)
    if isinstance(mirror, type) and is_dataclass(mirror):
        return mirror
    return None


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
    "Dimensions",
    "Eta",
    "ExtendedPositionReportB",
    "Gga",
    "Gll",
    "Hdg",
    "Hdt",
    "NmeaMessage",
    "Other",
    "PositionReportA",
    "PositionReportB",
    "Prdid",
    "PrdidPitchRollHeading",
    "PrdidRaw",
    "PrdidRollPitchHeading",
    "Psxn",
    "RawSentence",
    "Rmc",
    "SarAircraftPositionReport",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "Tll",
    "Ttm",
    "Unknown",
    "UtcDate",
    "UtcTime",
    "Vtg",
    "to_dataclass",
]
