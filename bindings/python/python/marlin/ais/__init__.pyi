"""Type stubs for marlin.ais — typed AIS decoders + reassembly."""

from __future__ import annotations

from types import TracebackType
from typing import Literal, Union, final

from typing_extensions import TypeAlias

from .. import MarlinError

class AisError(MarlinError): ...
class ReassemblyError(AisError): ...

@final
class NavStatus:
    UNDERWAY_USING_ENGINE: NavStatus
    AT_ANCHOR: NavStatus
    NOT_UNDER_COMMAND: NavStatus
    RESTRICTED_MANEUVERABILITY: NavStatus
    CONSTRAINED_BY_DRAFT: NavStatus
    MOORED: NavStatus
    AGROUND: NavStatus
    ENGAGED_IN_FISHING: NavStatus
    UNDERWAY_SAILING: NavStatus
    AIS_SART_ACTIVE: NavStatus
    NOT_DEFINED: NavStatus
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class ManeuverIndicator:
    NOT_AVAILABLE: ManeuverIndicator
    NO_SPECIAL: ManeuverIndicator
    SPECIAL: ManeuverIndicator
    RESERVED: ManeuverIndicator
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class TurnDirection:
    """Direction of a ±127 "no turn indicator" rate-of-turn status.

    The int values are enum discriminants (RIGHT = 0, LEFT = 1), not wire
    codes; on the wire the statuses are raw ROT +127 and −127.
    """

    RIGHT: TurnDirection
    LEFT: TurnDirection
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AltitudeSensor:
    """Source of a SAR aircraft's altitude (Type 9). Int values are the
    wire codes: GNSS = 0, BAROMETRIC = 1."""

    GNSS: AltitudeSensor
    BAROMETRIC: AltitudeSensor
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AtonType:
    """Type of aid to navigation (Type 21, ITU-R M.1371-5 Table 74).
    Int values are the 5-bit wire codes 0..=31; codes 5-19 are fixed AtoN,
    20-31 floating AtoN, 0-4 neither."""

    NOT_SPECIFIED: AtonType
    REFERENCE_POINT: AtonType
    RACON: AtonType
    FIXED_STRUCTURE_OFFSHORE: AtonType
    EMERGENCY_WRECK_MARKING_BUOY: AtonType
    LIGHT_WITHOUT_SECTORS: AtonType
    LIGHT_WITH_SECTORS: AtonType
    LEADING_LIGHT_FRONT: AtonType
    LEADING_LIGHT_REAR: AtonType
    BEACON_CARDINAL_NORTH: AtonType
    BEACON_CARDINAL_EAST: AtonType
    BEACON_CARDINAL_SOUTH: AtonType
    BEACON_CARDINAL_WEST: AtonType
    BEACON_PORT_HAND: AtonType
    BEACON_STARBOARD_HAND: AtonType
    BEACON_PREFERRED_CHANNEL_PORT_HAND: AtonType
    BEACON_PREFERRED_CHANNEL_STARBOARD_HAND: AtonType
    BEACON_ISOLATED_DANGER: AtonType
    BEACON_SAFE_WATER: AtonType
    BEACON_SPECIAL_MARK: AtonType
    CARDINAL_MARK_NORTH: AtonType
    CARDINAL_MARK_EAST: AtonType
    CARDINAL_MARK_SOUTH: AtonType
    CARDINAL_MARK_WEST: AtonType
    PORT_HAND_MARK: AtonType
    STARBOARD_HAND_MARK: AtonType
    PREFERRED_CHANNEL_PORT_HAND: AtonType
    PREFERRED_CHANNEL_STARBOARD_HAND: AtonType
    ISOLATED_DANGER: AtonType
    SAFE_WATER: AtonType
    SPECIAL_MARK: AtonType
    LIGHT_VESSEL: AtonType
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class EpfdType:
    UNDEFINED: EpfdType
    GPS: EpfdType
    GLONASS: EpfdType
    COMBINED_GPS_GLONASS: EpfdType
    LORAN_C: EpfdType
    CHAYKA: EpfdType
    INTEGRATED_NAVIGATION: EpfdType
    SURVEYED: EpfdType
    GALILEO: EpfdType
    INTERNAL_GNSS: EpfdType
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AisVersion:
    ITU1371V1: AisVersion
    ITU1371V3: AisVersion
    ITU1371V5: AisVersion
    FUTURE: AisVersion
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class Dimensions:
    def __new__(
        cls,
        to_bow_m: int | None = ...,
        to_stern_m: int | None = ...,
        to_port_m: int | None = ...,
        to_starboard_m: int | None = ...,
    ) -> Dimensions: ...
    @property
    def to_bow_m(self) -> int | None: ...
    @property
    def to_stern_m(self) -> int | None: ...
    @property
    def to_port_m(self) -> int | None: ...
    @property
    def to_starboard_m(self) -> int | None: ...

@final
class Eta:
    def __new__(
        cls,
        month: int | None = ...,
        day: int | None = ...,
        hour: int | None = ...,
        minute: int | None = ...,
    ) -> Eta: ...
    @property
    def month(self) -> int | None: ...
    @property
    def day(self) -> int | None: ...
    @property
    def hour(self) -> int | None: ...
    @property
    def minute(self) -> int | None: ...

@final
class PositionReportA:
    def __new__(
        cls,
        mmsi: int = ...,
        navigation_status: NavStatus = ...,
        rate_of_turn: float | None = ...,
        turn_direction: TurnDirection | None = ...,
        speed_over_ground: float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: float | None = ...,
        latitude_deg: float | None = ...,
        course_over_ground: float | None = ...,
        true_heading: int | None = ...,
        timestamp: int = ...,
        special_maneuver: ManeuverIndicator = ...,
        raim: bool = ...,
        radio_status: int = ...,
    ) -> PositionReportA: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def navigation_status(self) -> NavStatus: ...
    @property
    def rate_of_turn(self) -> float | None: ...
    @property
    def turn_direction(self) -> TurnDirection | None: ...
    @property
    def speed_over_ground(self) -> float | None: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def course_over_ground(self) -> float | None: ...
    @property
    def true_heading(self) -> int | None: ...
    @property
    def timestamp(self) -> int: ...
    @property
    def special_maneuver(self) -> ManeuverIndicator: ...
    @property
    def raim(self) -> bool: ...
    @property
    def radio_status(self) -> int: ...

@final
class StaticAndVoyageA:
    def __new__(
        cls,
        mmsi: int = ...,
        ais_version: AisVersion = ...,
        imo_number: int | None = ...,
        call_sign: str | None = ...,
        vessel_name: str | None = ...,
        ship_type: int = ...,
        dimensions: Dimensions | None = ...,
        epfd: EpfdType = ...,
        eta: Eta | None = ...,
        draught_m: float | None = ...,
        destination: str | None = ...,
        dte: bool = ...,
    ) -> StaticAndVoyageA: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def ais_version(self) -> AisVersion: ...
    @property
    def imo_number(self) -> int | None: ...
    @property
    def call_sign(self) -> str | None: ...
    @property
    def vessel_name(self) -> str | None: ...
    @property
    def ship_type(self) -> int: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> EpfdType: ...
    @property
    def eta(self) -> Eta: ...
    @property
    def draught_m(self) -> float | None: ...
    @property
    def destination(self) -> str | None: ...
    @property
    def dte(self) -> bool: ...

@final
class SarAircraftPositionReport:
    """Type 9 standard SAR aircraft position report.

    `altitude_m` and `speed_over_ground` are whole metres / whole knots;
    None is the not-available code, over-range codes (4094 m, 1022 kn)
    pass through. No heading, rate of turn or navigational status exists.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        altitude_m: int | None = ...,
        speed_over_ground: int | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: float | None = ...,
        latitude_deg: float | None = ...,
        course_over_ground: float | None = ...,
        timestamp: int = ...,
        altitude_sensor: AltitudeSensor = ...,
        dte: bool = ...,
        assigned_flag: bool = ...,
        raim: bool = ...,
        radio_status: int = ...,
    ) -> SarAircraftPositionReport: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def altitude_m(self) -> int | None: ...
    @property
    def speed_over_ground(self) -> int | None: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def course_over_ground(self) -> float | None: ...
    @property
    def timestamp(self) -> int: ...
    @property
    def altitude_sensor(self) -> AltitudeSensor: ...
    @property
    def dte(self) -> bool: ...
    @property
    def assigned_flag(self) -> bool: ...
    @property
    def raim(self) -> bool: ...
    @property
    def radio_status(self) -> int: ...

@final
class PositionReportB:
    def __new__(
        cls,
        mmsi: int = ...,
        speed_over_ground: float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: float | None = ...,
        latitude_deg: float | None = ...,
        course_over_ground: float | None = ...,
        true_heading: int | None = ...,
        timestamp: int = ...,
        class_b_cs_flag: bool = ...,
        class_b_display_flag: bool = ...,
        class_b_dsc_flag: bool = ...,
        class_b_band_flag: bool = ...,
        class_b_message22_flag: bool = ...,
        assigned_flag: bool = ...,
        raim: bool = ...,
        radio_status: int = ...,
    ) -> PositionReportB: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def speed_over_ground(self) -> float | None: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def course_over_ground(self) -> float | None: ...
    @property
    def true_heading(self) -> int | None: ...
    @property
    def timestamp(self) -> int: ...
    @property
    def class_b_cs_flag(self) -> bool: ...
    @property
    def class_b_display_flag(self) -> bool: ...
    @property
    def class_b_dsc_flag(self) -> bool: ...
    @property
    def class_b_band_flag(self) -> bool: ...
    @property
    def class_b_message22_flag(self) -> bool: ...
    @property
    def assigned_flag(self) -> bool: ...
    @property
    def raim(self) -> bool: ...
    @property
    def radio_status(self) -> int: ...

@final
class ExtendedPositionReportB:
    def __new__(
        cls,
        mmsi: int = ...,
        speed_over_ground: float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: float | None = ...,
        latitude_deg: float | None = ...,
        course_over_ground: float | None = ...,
        true_heading: int | None = ...,
        timestamp: int = ...,
        vessel_name: str | None = ...,
        ship_type: int = ...,
        dimensions: Dimensions | None = ...,
        epfd: EpfdType = ...,
        raim: bool = ...,
        dte: bool = ...,
        assigned_flag: bool = ...,
    ) -> ExtendedPositionReportB: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def speed_over_ground(self) -> float | None: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def course_over_ground(self) -> float | None: ...
    @property
    def true_heading(self) -> int | None: ...
    @property
    def timestamp(self) -> int: ...
    @property
    def vessel_name(self) -> str | None: ...
    @property
    def ship_type(self) -> int: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> EpfdType: ...
    @property
    def raim(self) -> bool: ...
    @property
    def dte(self) -> bool: ...
    @property
    def assigned_flag(self) -> bool: ...

@final
class AidToNavigationReport:
    """Type 21 aid-to-navigation report.

    `name` joins the 20-character name with the optional extension (up to
    14 more characters) and trims trailing `@` / spaces; an `@` inside the
    name is kept. `dimensions` is all-None for virtual AtoN and reference
    points. `aton_status` is the raw 8-bit field.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        aton_type: AtonType = ...,
        name: str | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: float | None = ...,
        latitude_deg: float | None = ...,
        dimensions: Dimensions | None = ...,
        epfd: EpfdType = ...,
        timestamp: int = ...,
        off_position: bool = ...,
        aton_status: int = ...,
        raim: bool = ...,
        virtual_aton: bool = ...,
        assigned_flag: bool = ...,
    ) -> AidToNavigationReport: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def aton_type(self) -> AtonType: ...
    @property
    def name(self) -> str | None: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> EpfdType: ...
    @property
    def timestamp(self) -> int: ...
    @property
    def off_position(self) -> bool: ...
    @property
    def aton_status(self) -> int: ...
    @property
    def raim(self) -> bool: ...
    @property
    def virtual_aton(self) -> bool: ...
    @property
    def assigned_flag(self) -> bool: ...

@final
class StaticDataB24A:
    def __new__(
        cls,
        mmsi: int = ...,
        vessel_name: str | None = ...,
    ) -> StaticDataB24A: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def vessel_name(self) -> str | None: ...

@final
class StaticDataB24B:
    def __new__(
        cls,
        mmsi: int = ...,
        ship_type: int = ...,
        vendor_id: str | None = ...,
        call_sign: str | None = ...,
        dimensions: Dimensions | None = ...,
        mothership_mmsi: int | None = ...,
        epfd: EpfdType = ...,
    ) -> StaticDataB24B: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def ship_type(self) -> int: ...
    @property
    def vendor_id(self) -> str | None: ...
    @property
    def call_sign(self) -> str | None: ...
    @property
    def dimensions(self) -> Dimensions | None: ...
    @property
    def mothership_mmsi(self) -> int | None: ...
    @property
    def epfd(self) -> EpfdType: ...

@final
class Other:
    def __new__(
        cls,
        msg_type: int = ...,
        raw_payload: bytes | None = ...,
        total_bits: int = ...,
    ) -> Other: ...
    @property
    def msg_type(self) -> int: ...
    @property
    def raw_payload(self) -> bytes: ...
    @property
    def total_bits(self) -> int: ...

AisMessageBody: TypeAlias = Union[
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

ClockMode: TypeAlias = Literal["auto", "manual"]

@final
class AisMessage:
    def __new__(
        cls,
        is_own_ship: bool,
        type_tag: str,
        body: AisMessageBody,
    ) -> AisMessage: ...
    @property
    def is_own_ship(self) -> bool: ...
    @property
    def type_tag(self) -> str: ...
    @property
    def body(self) -> AisMessageBody: ...

@final
class _AisIterator:
    def __iter__(self) -> _AisIterator: ...
    def __next__(self) -> AisMessage: ...

@final
class AisParser:
    @staticmethod
    def one_shot(
        timeout_ms: int | None = ...,
        clock: ClockMode | None = ...,
    ) -> AisParser: ...
    @staticmethod
    def streaming(
        timeout_ms: int | None = ...,
        clock: ClockMode | None = ...,
        max_size: int = ...,
    ) -> AisParser: ...
    def feed(self, data: bytes) -> None: ...
    def tick(self, now_ms: int) -> None: ...
    def next_message(self) -> AisMessage | None: ...
    def __iter__(self) -> _AisIterator: ...
    def iter(self, strict: bool = ...) -> _AisIterator: ...
    def __enter__(self) -> AisParser: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...

@final
class BitReader:
    def __new__(cls, data: bytes, total_bits: int) -> BitReader: ...
    def u(self, n: int) -> int: ...
    def i(self, n: int) -> int: ...
    def b(self) -> bool: ...
    def string(self, chars: int) -> str: ...
    def remaining(self) -> int: ...

__all__ = [
    "AidToNavigationReport",
    "AisError",
    "AisMessage",
    "AisMessageBody",
    "AisParser",
    "AisVersion",
    "AltitudeSensor",
    "AtonType",
    "BitReader",
    "ClockMode",
    "Dimensions",
    "EpfdType",
    "Eta",
    "ExtendedPositionReportB",
    "ManeuverIndicator",
    "NavStatus",
    "Other",
    "PositionReportA",
    "PositionReportB",
    "ReassemblyError",
    "SarAircraftPositionReport",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "TurnDirection",
]
