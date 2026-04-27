"""Type stubs for marlin.ais — typed AIS decoders + reassembly."""

from __future__ import annotations

from types import TracebackType
from typing import Literal, Union

from typing_extensions import TypeAlias

from .. import MarlinError

class AisError(MarlinError): ...
class ReassemblyError(AisError): ...

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
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class ManeuverIndicator:
    NOT_AVAILABLE: ManeuverIndicator
    NO_SPECIAL: ManeuverIndicator
    SPECIAL: ManeuverIndicator
    RESERVED: ManeuverIndicator
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

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
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class AisVersion:
    ITU1371V1: AisVersion
    ITU1371V3: AisVersion
    ITU1371V5: AisVersion
    FUTURE: AisVersion
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class Dimensions:
    def __init__(
        self,
        to_bow_m: int | None = ...,
        to_stern_m: int | None = ...,
        to_port_m: int | None = ...,
        to_starboard_m: int | None = ...,
    ) -> None: ...
    @property
    def to_bow_m(self) -> int | None: ...
    @property
    def to_stern_m(self) -> int | None: ...
    @property
    def to_port_m(self) -> int | None: ...
    @property
    def to_starboard_m(self) -> int | None: ...

class Eta:
    def __init__(
        self,
        month: int | None = ...,
        day: int | None = ...,
        hour: int | None = ...,
        minute: int | None = ...,
    ) -> None: ...
    @property
    def month(self) -> int | None: ...
    @property
    def day(self) -> int | None: ...
    @property
    def hour(self) -> int | None: ...
    @property
    def minute(self) -> int | None: ...

class PositionReportA:
    def __init__(
        self,
        mmsi: int = ...,
        navigation_status: NavStatus = ...,
        rate_of_turn: float | None = ...,
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
    ) -> None: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def navigation_status(self) -> NavStatus: ...
    @property
    def rate_of_turn(self) -> float | None: ...
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

class StaticAndVoyageA:
    def __init__(
        self,
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
    ) -> None: ...
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

class PositionReportB:
    def __init__(
        self,
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
    ) -> None: ...
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

class ExtendedPositionReportB:
    def __init__(
        self,
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
    ) -> None: ...
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

class StaticDataB24A:
    def __init__(
        self,
        mmsi: int = ...,
        vessel_name: str | None = ...,
    ) -> None: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def vessel_name(self) -> str | None: ...

class StaticDataB24B:
    def __init__(
        self,
        mmsi: int = ...,
        ship_type: int = ...,
        vendor_id: str | None = ...,
        call_sign: str | None = ...,
        dimensions: Dimensions | None = ...,
    ) -> None: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def ship_type(self) -> int: ...
    @property
    def vendor_id(self) -> str | None: ...
    @property
    def call_sign(self) -> str | None: ...
    @property
    def dimensions(self) -> Dimensions: ...

class Other:
    def __init__(
        self,
        msg_type: int = ...,
        raw_payload: bytes | None = ...,
        total_bits: int = ...,
    ) -> None: ...
    @property
    def msg_type(self) -> int: ...
    @property
    def raw_payload(self) -> bytes: ...
    @property
    def total_bits(self) -> int: ...

AisMessageBody: TypeAlias = Union[
    PositionReportA,
    StaticAndVoyageA,
    PositionReportB,
    ExtendedPositionReportB,
    StaticDataB24A,
    StaticDataB24B,
    Other,
]

ClockMode: TypeAlias = Literal["auto", "manual"]

class AisMessage:
    def __init__(
        self,
        is_own_ship: bool,
        type_tag: str,
        body: AisMessageBody,
    ) -> None: ...
    @property
    def is_own_ship(self) -> bool: ...
    @property
    def type_tag(self) -> str: ...
    @property
    def body(self) -> AisMessageBody: ...

class _AisIterator:
    def __iter__(self) -> _AisIterator: ...
    def __next__(self) -> AisMessage: ...

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
    ) -> bool: ...

class BitReader:
    def __init__(self, data: bytes, total_bits: int) -> None: ...
    def u(self, n: int) -> int: ...
    def i(self, n: int) -> int: ...
    def b(self) -> bool: ...
    def string(self, chars: int) -> str: ...
    def remaining(self) -> int: ...

__all__ = [
    "AisError",
    "AisMessage",
    "AisMessageBody",
    "AisParser",
    "AisVersion",
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
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
]
