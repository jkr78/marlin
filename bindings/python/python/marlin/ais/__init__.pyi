"""Type stubs for marlin.ais — typed AIS decoders + reassembly.

Every message attribute the wire can leave without a value is a
`marlin.field.FieldState`. The getter returns `FieldState[T]` only; the
constructor accepts `FieldState[T] | T | None` and coerces a bare value to
`FieldState.Value` and `None` to `FieldState.NotAvailable()`, which is also
the keyword default. `RateOfTurn`, `Timestamp` and `Type24BExtent` are sum
types with one nested variant class each per Rust variant (ADR-0009).
"""

from __future__ import annotations

from types import TracebackType
from typing import Literal, Union, final

from typing_extensions import Self, TypeAlias, disjoint_base

from .. import MarlinError
from ..field import FieldState

class AisError(MarlinError): ...
class ReassemblyError(AisError): ...

@final
class NavStatus:
    """Navigation status, int values the 4-bit wire codes 0..=8 and 14.

    Code 15 (not defined) is `FieldState.NotAvailable()` and the reserved
    codes 9..=13 are `FieldState.Invalid(code)` on the message.
    """

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
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class ManeuverIndicator:
    """Special manoeuvre indicator, int values the 2-bit wire codes 1 and 2.

    Code 0 is `FieldState.NotAvailable()` and the reserved code 3 is
    `FieldState.Invalid(3)` on the message.
    """

    NO_SPECIAL: ManeuverIndicator
    SPECIAL: ManeuverIndicator
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class TurnDirection:
    """Direction of a ±127 "no turn indicator" rate-of-turn status, carried
    by `RateOfTurn.NoIndicator(direction)`.

    The int values are enum discriminants (RIGHT = 0, LEFT = 1), not wire
    codes; on the wire the statuses are raw ROT +127 and −127.
    """

    RIGHT: TurnDirection
    LEFT: TurnDirection
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class PositioningStatus:
    """Status of the positioning system when a timestamp field carries a
    status instead of a second, carried by
    `Timestamp.PositioningStatus(status)`. Int values are the 6-bit
    timestamp wire codes 61, 62 and 63.
    """

    MANUAL_INPUT: PositioningStatus
    DEAD_RECKONING: PositioningStatus
    INOPERATIVE: PositioningStatus
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
    """Electronic position-fixing device type, int values the 4-bit wire
    codes 1..=8 and 15.

    Code 0 (undefined) is `FieldState.NotAvailable()` and the reserved
    codes 9..=14 are `FieldState.Invalid(code)` on the message.
    """

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
    """Extent of a station from its position-reference point, in metres.

    Each attribute is a `FieldState[int]`: the wire code 0 is
    `FieldState.NotAvailable()`, the field maximum (511 to bow or stern,
    63 to port or starboard) is `FieldState.AtLeast(511)` /
    `FieldState.AtLeast(63)`.
    """

    def __new__(
        cls,
        to_bow_m: FieldState[int] | int | None = ...,
        to_stern_m: FieldState[int] | int | None = ...,
        to_port_m: FieldState[int] | int | None = ...,
        to_starboard_m: FieldState[int] | int | None = ...,
    ) -> Dimensions: ...
    @property
    def to_bow_m(self) -> FieldState[int]: ...
    @property
    def to_stern_m(self) -> FieldState[int]: ...
    @property
    def to_port_m(self) -> FieldState[int]: ...
    @property
    def to_starboard_m(self) -> FieldState[int]: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class Eta:
    """Estimated time of arrival of a Type 5 report.

    Each attribute is a `FieldState[int]` with its own not-available code
    (month 0, day 0, hour 24, minute 60); a code the standard leaves
    undefined (month 13..=15, hour 25..=31, minute 61..=63) is
    `FieldState.Invalid(code)`.
    """

    def __new__(
        cls,
        month: FieldState[int] | int | None = ...,
        day: FieldState[int] | int | None = ...,
        hour: FieldState[int] | int | None = ...,
        minute: FieldState[int] | int | None = ...,
    ) -> Eta: ...
    @property
    def month(self) -> FieldState[int]: ...
    @property
    def day(self) -> FieldState[int]: ...
    @property
    def hour(self) -> FieldState[int]: ...
    @property
    def minute(self) -> FieldState[int]: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

# The three sum types below are not `@final`: their variant classes derive
# from them. `@disjoint_base` is what stubtest requires of a PyO3 base.
@disjoint_base
class RateOfTurn:
    """Rate of turn of a Class A position report, carried as
    `PositionReportA.rate_of_turn: FieldState[RateOfTurn]`.

    Not instantiable; construct one of the two variant classes and read
    one with `isinstance` or `match`.
    """

    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

    @final
    class DegPerMin(RateOfTurn):
        """A measured rate in degrees per minute, starboard positive."""

        __match_args__ = ("deg_per_min",)
        def __new__(cls, deg_per_min: float) -> Self: ...
        @property
        def deg_per_min(self) -> float: ...

    @final
    class NoIndicator(RateOfTurn):
        """Raw ±127: turning at more than 5° per 30 s, no turn indicator;
        only the direction is known. A value of the field, not a field
        state."""

        __match_args__ = ("direction",)
        def __new__(cls, direction: TurnDirection) -> Self: ...
        @property
        def direction(self) -> TurnDirection: ...

@disjoint_base
class Timestamp:
    """The timestamp field of a position report, carried as
    `timestamp: FieldState[Timestamp]` on Types 1/2/3, 9, 18, 19 and 21.

    Not instantiable; construct one of the two variant classes and read
    one with `isinstance` or `match`. The wire code 60 is
    `FieldState.NotAvailable()` on the message.
    """

    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

    @final
    class Second(Timestamp):
        """The UTC second within the minute of the position fix, 0..=59."""

        __match_args__ = ("second",)
        def __new__(cls, second: int) -> Self: ...
        @property
        def second(self) -> int: ...

    @final
    class PositioningStatus(Timestamp):
        """Wire codes 61..=63: the positioning system reports a status
        instead of a second. A value of the field, not a field state."""

        __match_args__ = ("status",)
        def __new__(cls, status: PositioningStatus) -> Self: ...
        @property
        def status(self) -> PositioningStatus: ...

@disjoint_base
class Type24BExtent:
    """What the 30-bit extent field of a Type 24 Part B holds, carried as
    `StaticDataB24B.extent`.

    Not instantiable; construct one of the two variant classes and read
    one with `isinstance` or `match`. The variant is decided by the MMSI
    prefix (ADR-0002): an auxiliary craft (`98MIDxxxx`) sends its mother
    ship's MMSI where every other station sends dimensions.
    """

    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

    @final
    class Dimensions(Type24BExtent):
        """Dimensions A/B/C/D, for every MMSI that is not an auxiliary
        craft."""

        __match_args__ = ("dimensions",)
        def __new__(cls, dimensions: Dimensions) -> Self: ...
        @property
        def dimensions(self) -> Dimensions: ...

    @final
    class MothershipMmsi(Type24BExtent):
        """MMSI of the mother ship, for an auxiliary-craft MMSI."""

        __match_args__ = ("mmsi",)
        def __new__(cls, mmsi: int) -> Self: ...
        @property
        def mmsi(self) -> int: ...

@final
class PositionReportA:
    """Class A position report (Types 1, 2 and 3).

    Field-state attributes: `navigation_status`, `rate_of_turn`,
    `speed_over_ground` (`FieldState.AtLeast(102.2)` on the over-range
    code), `longitude_deg`, `latitude_deg`, `course_over_ground`,
    `true_heading`, `timestamp`, `special_maneuver`. The flags, `mmsi`
    and `radio_status` are plain.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        navigation_status: FieldState[NavStatus] | NavStatus | None = ...,
        rate_of_turn: FieldState[RateOfTurn] | RateOfTurn | None = ...,
        speed_over_ground: FieldState[float] | float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        course_over_ground: FieldState[float] | float | None = ...,
        true_heading: FieldState[int] | int | None = ...,
        timestamp: FieldState[Timestamp] | Timestamp | None = ...,
        special_maneuver: FieldState[ManeuverIndicator] | ManeuverIndicator | None = ...,
        raim: bool = ...,
        radio_status: int = ...,
    ) -> PositionReportA: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def navigation_status(self) -> FieldState[NavStatus]: ...
    @property
    def rate_of_turn(self) -> FieldState[RateOfTurn]: ...
    @property
    def speed_over_ground(self) -> FieldState[float]: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def course_over_ground(self) -> FieldState[float]: ...
    @property
    def true_heading(self) -> FieldState[int]: ...
    @property
    def timestamp(self) -> FieldState[Timestamp]: ...
    @property
    def special_maneuver(self) -> FieldState[ManeuverIndicator]: ...
    @property
    def raim(self) -> bool: ...
    @property
    def radio_status(self) -> int: ...

@final
class StaticAndVoyageA:
    """Class A static and voyage data (Type 5).

    Field-state attributes: `imo_number`, `call_sign`, `vessel_name`,
    `ship_type`, `epfd`, `draught_m` (`FieldState.AtLeast(25.5)` on the
    over-range code), `destination` and `dte`, which is
    `FieldState.NotAvailable()` on a 420- or 422-bit payload that ends
    before the DTE bit. `dimensions` and `eta` carry a field state per
    member.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        ais_version: AisVersion = ...,
        imo_number: FieldState[int] | int | None = ...,
        call_sign: FieldState[str] | str | None = ...,
        vessel_name: FieldState[str] | str | None = ...,
        ship_type: FieldState[int] | int | None = ...,
        dimensions: Dimensions | None = ...,
        epfd: FieldState[EpfdType] | EpfdType | None = ...,
        eta: Eta | None = ...,
        draught_m: FieldState[float] | float | None = ...,
        destination: FieldState[str] | str | None = ...,
        dte: FieldState[bool] | bool | None = ...,
    ) -> StaticAndVoyageA: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def ais_version(self) -> AisVersion: ...
    @property
    def imo_number(self) -> FieldState[int]: ...
    @property
    def call_sign(self) -> FieldState[str]: ...
    @property
    def vessel_name(self) -> FieldState[str]: ...
    @property
    def ship_type(self) -> FieldState[int]: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> FieldState[EpfdType]: ...
    @property
    def eta(self) -> Eta: ...
    @property
    def draught_m(self) -> FieldState[float]: ...
    @property
    def destination(self) -> FieldState[str]: ...
    @property
    def dte(self) -> FieldState[bool]: ...

@final
class SarAircraftPositionReport:
    """Type 9 standard SAR aircraft position report.

    `altitude_m` and `speed_over_ground` are `FieldState[int]` in whole
    metres / whole knots: 4095 / 1023 are `FieldState.NotAvailable()`, the
    over-range codes 4094 / 1022 are `FieldState.AtLeast(4094)` /
    `FieldState.AtLeast(1022)`. No heading, rate of turn or navigational
    status exists; `dte` is a plain `bool`.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        altitude_m: FieldState[int] | int | None = ...,
        speed_over_ground: FieldState[int] | int | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        course_over_ground: FieldState[float] | float | None = ...,
        timestamp: FieldState[Timestamp] | Timestamp | None = ...,
        altitude_sensor: AltitudeSensor = ...,
        dte: bool = ...,
        assigned_flag: bool = ...,
        raim: bool = ...,
        radio_status: int = ...,
    ) -> SarAircraftPositionReport: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def altitude_m(self) -> FieldState[int]: ...
    @property
    def speed_over_ground(self) -> FieldState[int]: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def course_over_ground(self) -> FieldState[float]: ...
    @property
    def timestamp(self) -> FieldState[Timestamp]: ...
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
    """Class B CS position report (Type 18). The position, speed, course,
    heading and timestamp attributes are field states as on
    `PositionReportA`; the Class B capability flags are plain."""

    def __new__(
        cls,
        mmsi: int = ...,
        speed_over_ground: FieldState[float] | float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        course_over_ground: FieldState[float] | float | None = ...,
        true_heading: FieldState[int] | int | None = ...,
        timestamp: FieldState[Timestamp] | Timestamp | None = ...,
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
    def speed_over_ground(self) -> FieldState[float]: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def course_over_ground(self) -> FieldState[float]: ...
    @property
    def true_heading(self) -> FieldState[int]: ...
    @property
    def timestamp(self) -> FieldState[Timestamp]: ...
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
    """Class B extended position report (Type 19): the Type 18 position
    attributes plus the Type 5 static tail (`vessel_name`, `ship_type`,
    `dimensions`, `epfd`). `dte` is a plain `bool`."""

    def __new__(
        cls,
        mmsi: int = ...,
        speed_over_ground: FieldState[float] | float | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        course_over_ground: FieldState[float] | float | None = ...,
        true_heading: FieldState[int] | int | None = ...,
        timestamp: FieldState[Timestamp] | Timestamp | None = ...,
        vessel_name: FieldState[str] | str | None = ...,
        ship_type: FieldState[int] | int | None = ...,
        dimensions: Dimensions | None = ...,
        epfd: FieldState[EpfdType] | EpfdType | None = ...,
        raim: bool = ...,
        dte: bool = ...,
        assigned_flag: bool = ...,
    ) -> ExtendedPositionReportB: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def speed_over_ground(self) -> FieldState[float]: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def course_over_ground(self) -> FieldState[float]: ...
    @property
    def true_heading(self) -> FieldState[int]: ...
    @property
    def timestamp(self) -> FieldState[Timestamp]: ...
    @property
    def vessel_name(self) -> FieldState[str]: ...
    @property
    def ship_type(self) -> FieldState[int]: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> FieldState[EpfdType]: ...
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
    name is kept; all padding is `FieldState.NotAvailable()`. `dimensions`
    carries a field state per member and is all not available for virtual
    AtoN and reference points. `aton_status` is the plain 8-bit field.
    """

    def __new__(
        cls,
        mmsi: int = ...,
        aton_type: AtonType = ...,
        name: FieldState[str] | str | None = ...,
        position_accuracy: bool = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        dimensions: Dimensions | None = ...,
        epfd: FieldState[EpfdType] | EpfdType | None = ...,
        timestamp: FieldState[Timestamp] | Timestamp | None = ...,
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
    def name(self) -> FieldState[str]: ...
    @property
    def position_accuracy(self) -> bool: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def dimensions(self) -> Dimensions: ...
    @property
    def epfd(self) -> FieldState[EpfdType]: ...
    @property
    def timestamp(self) -> FieldState[Timestamp]: ...
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
        vessel_name: FieldState[str] | str | None = ...,
    ) -> StaticDataB24A: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def vessel_name(self) -> FieldState[str]: ...

@final
class StaticDataB24B:
    """Class B static data Part B (Type 24 Part B). `extent` is a
    `Type24BExtent`: dimensions for every ordinary MMSI, the mother ship's
    MMSI for an auxiliary craft; it defaults to dimensions with every
    member not available."""

    def __new__(
        cls,
        mmsi: int = ...,
        ship_type: FieldState[int] | int | None = ...,
        vendor_id: FieldState[str] | str | None = ...,
        call_sign: FieldState[str] | str | None = ...,
        extent: Type24BExtent | None = ...,
        epfd: FieldState[EpfdType] | EpfdType | None = ...,
    ) -> StaticDataB24B: ...
    @property
    def mmsi(self) -> int: ...
    @property
    def ship_type(self) -> FieldState[int]: ...
    @property
    def vendor_id(self) -> FieldState[str]: ...
    @property
    def call_sign(self) -> FieldState[str]: ...
    @property
    def extent(self) -> Type24BExtent: ...
    @property
    def epfd(self) -> FieldState[EpfdType]: ...

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
    "PositioningStatus",
    "RateOfTurn",
    "ReassemblyError",
    "SarAircraftPositionReport",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "Timestamp",
    "TurnDirection",
    "Type24BExtent",
]
