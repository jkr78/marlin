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
as their integer values for JSON-friendly output. The converters below
handle the `int(...)` extraction.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Tuple, Union


# ---------- shared value types ----------


@dataclass(frozen=True)
class UtcTime:
    """Mirror of marlin.nmea.UtcTime."""

    hour: int
    minute: int
    second: int
    millisecond: int


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
    """

    mmsi: int
    navigation_status: int
    rate_of_turn: Optional[float]
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
class StaticDataB24A:
    """Mirror of marlin.ais.StaticDataB24A (Type 24 Part A)."""

    mmsi: int
    vessel_name: Optional[str]


@dataclass(frozen=True)
class StaticDataB24B:
    """Mirror of marlin.ais.StaticDataB24B (Type 24 Part B).

    `dimensions` is always present per the Rust type.
    """

    mmsi: int
    ship_type: int
    vendor_id: Optional[str]
    call_sign: Optional[str]
    dimensions: Dimensions


@dataclass(frozen=True)
class Other:
    """Mirror of marlin.ais.Other (catch-all for un-decoded msg_type)."""

    msg_type: int
    raw_payload: bytes
    total_bits: int


AisMessageBody = Union[
    PositionReportA,
    StaticAndVoyageA,
    PositionReportB,
    ExtendedPositionReportB,
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

NmeaMessage = Union[Gga, Vtg, Hdt, Psxn, Prdid, Unknown]


# ---------- dispatcher ----------


def to_dataclass(msg: object) -> object:
    """Convert a marlin runtime message into its frozen dataclass mirror.

    Accepts any of:
    - Envelope: ``marlin.envelope.RawSentence``
    - NMEA: ``Gga``, ``Vtg``, ``Hdt``, ``Psxn``, ``Prdid``, ``Unknown``
    - AIS: ``AisMessage`` (the wrapper) or any body variant directly

    Raises ``TypeError`` if the object is not a recognized marlin message.
    """
    import marlin.ais as _ais
    import marlin.envelope as _env
    import marlin.nmea as _nmea

    # --- envelope ---
    if isinstance(msg, _env.RawSentence):
        return RawSentence(
            start_delimiter=msg.start_delimiter,
            talker=msg.talker,
            sentence_type=msg.sentence_type,
            fields=msg.fields,
            tag_block=msg.tag_block,
            checksum_ok=msg.checksum_ok,
            raw=msg.raw,
        )

    # --- NMEA ---
    if isinstance(msg, _nmea.Gga):
        return Gga(
            talker=msg.talker,
            utc=_convert_utc(msg.utc),
            latitude_deg=msg.latitude_deg,
            longitude_deg=msg.longitude_deg,
            fix_quality=int(msg.fix_quality),
            satellites_used=msg.satellites_used,
            hdop=msg.hdop,
            altitude_m=msg.altitude_m,
            geoid_separation_m=msg.geoid_separation_m,
            dgps_age_s=msg.dgps_age_s,
            dgps_station_id=msg.dgps_station_id,
        )
    if isinstance(msg, _nmea.Vtg):
        return Vtg(
            talker=msg.talker,
            course_true_deg=msg.course_true_deg,
            course_magnetic_deg=msg.course_magnetic_deg,
            speed_knots=msg.speed_knots,
            speed_kmh=msg.speed_kmh,
            mode=int(msg.mode) if msg.mode is not None else None,
        )
    if isinstance(msg, _nmea.Hdt):
        return Hdt(
            talker=msg.talker,
            heading_true_deg=msg.heading_true_deg,
        )
    if isinstance(msg, _nmea.Unknown):
        return Unknown(
            talker=msg.talker,
            sentence_type=msg.sentence_type,
        )
    if isinstance(msg, _nmea.Psxn):
        return Psxn(
            id=msg.id,
            token=msg.token,
            roll_deg=msg.roll_deg,
            pitch_deg=msg.pitch_deg,
            heave_m=msg.heave_m,
        )
    if isinstance(msg, _nmea.Prdid):
        return _convert_prdid(msg)

    # --- AIS ---
    if isinstance(msg, _ais.AisMessage):
        return AisMessage(
            is_own_ship=msg.is_own_ship,
            type_tag=msg.type_tag,
            body=_convert_ais_body(msg.body),
        )
    if isinstance(msg, _ais.PositionReportA):
        return _convert_position_report_a(msg)
    if isinstance(msg, _ais.StaticAndVoyageA):
        return _convert_static_and_voyage_a(msg)
    if isinstance(msg, _ais.PositionReportB):
        return _convert_position_report_b(msg)
    if isinstance(msg, _ais.ExtendedPositionReportB):
        return _convert_extended_position_report_b(msg)
    if isinstance(msg, _ais.StaticDataB24A):
        return StaticDataB24A(mmsi=msg.mmsi, vessel_name=msg.vessel_name)
    if isinstance(msg, _ais.StaticDataB24B):
        return _convert_static_data_b24b(msg)
    if isinstance(msg, _ais.Other):
        return Other(
            msg_type=msg.msg_type,
            raw_payload=msg.raw_payload,
            total_bits=msg.total_bits,
        )

    raise TypeError(
        f"to_dataclass: unrecognised marlin message type {type(msg).__qualname__!r}"
    )


# ---------- private converters ----------


def _convert_utc(utc: object) -> Optional[UtcTime]:
    import marlin.nmea as _nmea

    if utc is None:
        return None
    if isinstance(utc, _nmea.UtcTime):
        return UtcTime(
            hour=utc.hour,
            minute=utc.minute,
            second=utc.second,
            millisecond=utc.millisecond,
        )
    raise TypeError(
        f"_convert_utc: expected UtcTime or None, got {type(utc).__qualname__!r}"
    )


def _convert_dimensions(d: object) -> Dimensions:
    import marlin.ais as _ais

    if isinstance(d, _ais.Dimensions):
        return Dimensions(
            to_bow_m=d.to_bow_m,
            to_stern_m=d.to_stern_m,
            to_port_m=d.to_port_m,
            to_starboard_m=d.to_starboard_m,
        )
    return Dimensions(to_bow_m=None, to_stern_m=None, to_port_m=None, to_starboard_m=None)


def _convert_eta(e: object) -> Eta:
    import marlin.ais as _ais

    if isinstance(e, _ais.Eta):
        return Eta(month=e.month, day=e.day, hour=e.hour, minute=e.minute)
    return Eta(month=None, day=None, hour=None, minute=None)


def _convert_prdid(msg: object) -> Prdid:
    import marlin.nmea as _nmea

    if not isinstance(msg, _nmea.Prdid):
        raise TypeError(f"expected Prdid, got {type(msg)!r}")
    variant = msg.variant
    body = msg.body
    if isinstance(body, _nmea.PrdidPitchRollHeading):
        dc_body: Union[PrdidPitchRollHeading, PrdidRollPitchHeading, PrdidRaw] = (
            PrdidPitchRollHeading(
                pitch_deg=body.pitch_deg,
                roll_deg=body.roll_deg,
                heading_deg=body.heading_deg,
            )
        )
    elif isinstance(body, _nmea.PrdidRollPitchHeading):
        dc_body = PrdidRollPitchHeading(
            roll_deg=body.roll_deg,
            pitch_deg=body.pitch_deg,
            heading_deg=body.heading_deg,
        )
    else:
        # PrdidRaw (isinstance check for type narrowing)
        if isinstance(body, _nmea.PrdidRaw):
            dc_body = PrdidRaw(fields=body.fields)
        else:
            dc_body = PrdidRaw(fields=())
    return Prdid(variant=variant, body=dc_body)


def _convert_position_report_a(msg: object) -> PositionReportA:
    import marlin.ais as _ais

    if not isinstance(msg, _ais.PositionReportA):
        raise TypeError(f"expected PositionReportA, got {type(msg)!r}")
    return PositionReportA(
        mmsi=msg.mmsi,
        navigation_status=int(msg.navigation_status),
        rate_of_turn=msg.rate_of_turn,
        speed_over_ground=msg.speed_over_ground,
        position_accuracy=msg.position_accuracy,
        longitude_deg=msg.longitude_deg,
        latitude_deg=msg.latitude_deg,
        course_over_ground=msg.course_over_ground,
        true_heading=msg.true_heading,
        timestamp=msg.timestamp,
        special_maneuver=int(msg.special_maneuver),
        raim=msg.raim,
        radio_status=msg.radio_status,
    )


def _convert_static_and_voyage_a(msg: object) -> StaticAndVoyageA:
    import marlin.ais as _ais

    if not isinstance(msg, _ais.StaticAndVoyageA):
        raise TypeError(f"expected StaticAndVoyageA, got {type(msg)!r}")
    return StaticAndVoyageA(
        mmsi=msg.mmsi,
        ais_version=int(msg.ais_version),
        imo_number=msg.imo_number,
        call_sign=msg.call_sign,
        vessel_name=msg.vessel_name,
        ship_type=msg.ship_type,
        dimensions=_convert_dimensions(msg.dimensions),
        epfd=int(msg.epfd),
        eta=_convert_eta(msg.eta),
        draught_m=msg.draught_m,
        destination=msg.destination,
        dte=msg.dte,
    )


def _convert_position_report_b(msg: object) -> PositionReportB:
    import marlin.ais as _ais

    if not isinstance(msg, _ais.PositionReportB):
        raise TypeError(f"expected PositionReportB, got {type(msg)!r}")
    return PositionReportB(
        mmsi=msg.mmsi,
        speed_over_ground=msg.speed_over_ground,
        position_accuracy=msg.position_accuracy,
        longitude_deg=msg.longitude_deg,
        latitude_deg=msg.latitude_deg,
        course_over_ground=msg.course_over_ground,
        true_heading=msg.true_heading,
        timestamp=msg.timestamp,
        class_b_cs_flag=msg.class_b_cs_flag,
        class_b_display_flag=msg.class_b_display_flag,
        class_b_dsc_flag=msg.class_b_dsc_flag,
        class_b_band_flag=msg.class_b_band_flag,
        class_b_message22_flag=msg.class_b_message22_flag,
        assigned_flag=msg.assigned_flag,
        raim=msg.raim,
        radio_status=msg.radio_status,
    )


def _convert_extended_position_report_b(msg: object) -> ExtendedPositionReportB:
    import marlin.ais as _ais

    if not isinstance(msg, _ais.ExtendedPositionReportB):
        raise TypeError(f"expected ExtendedPositionReportB, got {type(msg)!r}")
    return ExtendedPositionReportB(
        mmsi=msg.mmsi,
        speed_over_ground=msg.speed_over_ground,
        position_accuracy=msg.position_accuracy,
        longitude_deg=msg.longitude_deg,
        latitude_deg=msg.latitude_deg,
        course_over_ground=msg.course_over_ground,
        true_heading=msg.true_heading,
        timestamp=msg.timestamp,
        vessel_name=msg.vessel_name,
        ship_type=msg.ship_type,
        dimensions=_convert_dimensions(msg.dimensions),
        epfd=int(msg.epfd),
        raim=msg.raim,
        dte=msg.dte,
        assigned_flag=msg.assigned_flag,
    )


def _convert_static_data_b24b(msg: object) -> StaticDataB24B:
    import marlin.ais as _ais

    if not isinstance(msg, _ais.StaticDataB24B):
        raise TypeError(f"expected StaticDataB24B, got {type(msg)!r}")
    return StaticDataB24B(
        mmsi=msg.mmsi,
        ship_type=msg.ship_type,
        vendor_id=msg.vendor_id,
        call_sign=msg.call_sign,
        dimensions=_convert_dimensions(msg.dimensions),
    )


def _convert_ais_body(body: object) -> AisMessageBody:
    import marlin.ais as _ais

    if isinstance(body, _ais.PositionReportA):
        return _convert_position_report_a(body)
    if isinstance(body, _ais.StaticAndVoyageA):
        return _convert_static_and_voyage_a(body)
    if isinstance(body, _ais.PositionReportB):
        return _convert_position_report_b(body)
    if isinstance(body, _ais.ExtendedPositionReportB):
        return _convert_extended_position_report_b(body)
    if isinstance(body, _ais.StaticDataB24A):
        return StaticDataB24A(mmsi=body.mmsi, vessel_name=body.vessel_name)
    if isinstance(body, _ais.StaticDataB24B):
        return _convert_static_data_b24b(body)
    if isinstance(body, _ais.Other):
        return Other(
            msg_type=body.msg_type,
            raw_payload=body.raw_payload,
            total_bits=body.total_bits,
        )
    raise TypeError(
        f"to_dataclass: unrecognised AIS body type {type(body).__qualname__!r}"
    )


__all__ = [
    "AisMessage",
    "AisMessageBody",
    "Dimensions",
    "Eta",
    "ExtendedPositionReportB",
    "Gga",
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
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "Unknown",
    "UtcTime",
    "Vtg",
    "to_dataclass",
]
