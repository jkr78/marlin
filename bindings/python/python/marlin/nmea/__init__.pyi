"""Type stubs for marlin.nmea — typed NMEA 0183 decoders."""

from __future__ import annotations

from types import TracebackType
from typing import Literal, Union, final

from typing_extensions import TypeAlias

from .. import MarlinError
from ..envelope import RawSentence
from ..field import FieldState

class DecodeError(MarlinError):
    """A typed decode failed: the sentence has fewer fields than its decoder's floor.

    The one reason a typed decode fails; a field's value never does, it
    decodes to a `FieldState`.
    """

@final
class GgaFixQuality:
    """The sender's own statement of fix quality; `NO_FIX` is a value it reported.

    An undefined digit decodes to `FieldState.Invalid(digit)` on the message.
    """

    NO_FIX: GgaFixQuality
    GPS_FIX: GgaFixQuality
    DGPS_FIX: GgaFixQuality
    PPS_FIX: GgaFixQuality
    RTK_FIXED: GgaFixQuality
    RTK_FLOAT: GgaFixQuality
    DEAD_RECKONING: GgaFixQuality
    MANUAL_INPUT: GgaFixQuality
    SIMULATOR: GgaFixQuality
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class VtgMode:
    NOT_VALID: VtgMode
    AUTONOMOUS: VtgMode
    DIFFERENTIAL: VtgMode
    ESTIMATED: VtgMode
    MANUAL: VtgMode
    SIMULATOR: VtgMode
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class DataStatus:
    ACTIVE: DataStatus
    VOID: DataStatus
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class RmcNavStatus:
    SAFE: RmcNavStatus
    CAUTION: RmcNavStatus
    UNSAFE: RmcNavStatus
    NOT_VALID: RmcNavStatus
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class PsxnSlot:
    ROLL: PsxnSlot
    PITCH: PsxnSlot
    HEAVE: PsxnSlot
    ROLL_SINE_ENCODED: PsxnSlot
    PITCH_SINE_ENCODED: PsxnSlot
    IGNORED: PsxnSlot
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class PrdidDialect:
    UNKNOWN: PrdidDialect
    PITCH_ROLL_HEADING: PrdidDialect
    ROLL_PITCH_HEADING: PrdidDialect
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class TargetStatus:
    LOST: TargetStatus
    QUERY: TargetStatus
    TRACKING: TargetStatus
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AngleReference:
    TRUE: AngleReference
    RELATIVE: AngleReference
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class DistanceUnits:
    NAUTICAL: DistanceUnits
    KILOMETERS: DistanceUnits
    STATUTE: DistanceUnits
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AcquisitionType:
    AUTOMATIC: AcquisitionType
    MANUAL: AcquisitionType
    REPORTED: AcquisitionType
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class PsxnLayout:
    @staticmethod
    def from_str(s: str) -> PsxnLayout: ...

@final
class UtcTime:
    def __new__(
        cls, hour: int, minute: int, second: int, millisecond: int
    ) -> UtcTime: ...
    @property
    def hour(self) -> int: ...
    @property
    def minute(self) -> int: ...
    @property
    def second(self) -> int: ...
    @property
    def millisecond(self) -> int: ...

@final
class UtcDate:
    def __new__(cls, day: int, month: int, year_yy: int) -> UtcDate: ...
    @property
    def day(self) -> int: ...
    @property
    def month(self) -> int: ...
    @property
    def year_yy(self) -> int: ...

# Every message attribute but `talker` is a field state. A getter returns
# `FieldState[T]`; a constructor accepts `FieldState[T] | T | None` and
# coerces a bare value to `FieldState.Value` and `None` to
# `FieldState.NotAvailable()`, which is also the default.

@final
class Gga:
    def __new__(
        cls,
        talker: bytes | None,
        utc: FieldState[UtcTime] | UtcTime | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        fix_quality: FieldState[GgaFixQuality] | GgaFixQuality | None = ...,
        satellites_used: FieldState[int] | int | None = ...,
        hdop: FieldState[float] | float | None = ...,
        altitude_m: FieldState[float] | float | None = ...,
        geoid_separation_m: FieldState[float] | float | None = ...,
        dgps_age_s: FieldState[float] | float | None = ...,
        dgps_station_id: FieldState[int] | int | None = ...,
    ) -> Gga: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def utc(self) -> FieldState[UtcTime]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def fix_quality(self) -> FieldState[GgaFixQuality]: ...
    @property
    def satellites_used(self) -> FieldState[int]: ...
    @property
    def hdop(self) -> FieldState[float]: ...
    @property
    def altitude_m(self) -> FieldState[float]: ...
    @property
    def geoid_separation_m(self) -> FieldState[float]: ...
    @property
    def dgps_age_s(self) -> FieldState[float]: ...
    @property
    def dgps_station_id(self) -> FieldState[int]: ...

@final
class Vtg:
    def __new__(
        cls,
        talker: bytes | None,
        course_true_deg: FieldState[float] | float | None = ...,
        course_magnetic_deg: FieldState[float] | float | None = ...,
        speed_knots: FieldState[float] | float | None = ...,
        speed_kmh: FieldState[float] | float | None = ...,
        mode: FieldState[VtgMode] | VtgMode | None = ...,
    ) -> Vtg: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def course_true_deg(self) -> FieldState[float]: ...
    @property
    def course_magnetic_deg(self) -> FieldState[float]: ...
    @property
    def speed_knots(self) -> FieldState[float]: ...
    @property
    def speed_kmh(self) -> FieldState[float]: ...
    @property
    def mode(self) -> FieldState[VtgMode]: ...

@final
class Hdt:
    def __new__(
        cls,
        talker: bytes | None,
        heading_true_deg: FieldState[float] | float | None = ...,
    ) -> Hdt: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def heading_true_deg(self) -> FieldState[float]: ...

@final
class Hdg:
    def __new__(
        cls,
        talker: bytes | None,
        heading_magnetic_deg: FieldState[float] | float | None = ...,
        deviation_deg: FieldState[float] | float | None = ...,
        variation_deg: FieldState[float] | float | None = ...,
    ) -> Hdg: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def heading_magnetic_deg(self) -> FieldState[float]: ...
    @property
    def deviation_deg(self) -> FieldState[float]: ...
    @property
    def variation_deg(self) -> FieldState[float]: ...

@final
class Ttm:
    @property
    def talker(self) -> bytes | None: ...
    @property
    def target_number(self) -> FieldState[int]: ...
    @property
    def distance(self) -> FieldState[float]: ...
    @property
    def bearing_deg(self) -> FieldState[float]: ...
    @property
    def bearing_reference(self) -> FieldState[AngleReference]: ...
    @property
    def speed(self) -> FieldState[float]: ...
    @property
    def course_deg(self) -> FieldState[float]: ...
    @property
    def course_reference(self) -> FieldState[AngleReference]: ...
    @property
    def cpa(self) -> FieldState[float]: ...
    @property
    def tcpa(self) -> FieldState[float]: ...
    @property
    def units(self) -> FieldState[DistanceUnits]: ...
    @property
    def name(self) -> FieldState[str]: ...
    @property
    def status(self) -> FieldState[TargetStatus]: ...
    @property
    def reference_target(self) -> FieldState[bool]: ...
    @property
    def utc_time(self) -> FieldState[UtcTime]: ...
    @property
    def acquisition(self) -> FieldState[AcquisitionType]: ...

@final
class Tll:
    @property
    def talker(self) -> bytes | None: ...
    @property
    def target_number(self) -> FieldState[int]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def name(self) -> FieldState[str]: ...
    @property
    def utc_time(self) -> FieldState[UtcTime]: ...
    @property
    def status(self) -> FieldState[TargetStatus]: ...
    @property
    def reference_target(self) -> FieldState[bool]: ...

@final
class Rmc:
    def __new__(
        cls,
        talker: bytes | None,
        utc: FieldState[UtcTime] | UtcTime | None = ...,
        status: FieldState[DataStatus] | DataStatus | None = ...,
        latitude_deg: FieldState[float] | float | None = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        speed_knots: FieldState[float] | float | None = ...,
        course_true_deg: FieldState[float] | float | None = ...,
        date: FieldState[UtcDate] | UtcDate | None = ...,
        magnetic_variation_deg: FieldState[float] | float | None = ...,
        mode: FieldState[VtgMode] | VtgMode | None = ...,
        nav_status: FieldState[RmcNavStatus] | RmcNavStatus | None = ...,
    ) -> Rmc: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def utc(self) -> FieldState[UtcTime]: ...
    @property
    def status(self) -> FieldState[DataStatus]: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def speed_knots(self) -> FieldState[float]: ...
    @property
    def course_true_deg(self) -> FieldState[float]: ...
    @property
    def date(self) -> FieldState[UtcDate]: ...
    @property
    def magnetic_variation_deg(self) -> FieldState[float]: ...
    @property
    def mode(self) -> FieldState[VtgMode]: ...
    @property
    def nav_status(self) -> FieldState[RmcNavStatus]: ...

@final
class Gll:
    def __new__(
        cls,
        talker: bytes | None,
        latitude_deg: FieldState[float] | float | None = ...,
        longitude_deg: FieldState[float] | float | None = ...,
        utc: FieldState[UtcTime] | UtcTime | None = ...,
        status: FieldState[DataStatus] | DataStatus | None = ...,
        mode: FieldState[VtgMode] | VtgMode | None = ...,
    ) -> Gll: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def latitude_deg(self) -> FieldState[float]: ...
    @property
    def longitude_deg(self) -> FieldState[float]: ...
    @property
    def utc(self) -> FieldState[UtcTime]: ...
    @property
    def status(self) -> FieldState[DataStatus]: ...
    @property
    def mode(self) -> FieldState[VtgMode]: ...

@final
class Unknown:
    def __new__(
        cls, talker: bytes | None, sentence_type: str
    ) -> Unknown: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def sentence_type(self) -> str: ...

@final
class Psxn:
    def __new__(
        cls,
        id: FieldState[int] | int | None = ...,
        token: FieldState[bytes] | bytes | None = ...,
        roll_deg: FieldState[float] | float | None = ...,
        pitch_deg: FieldState[float] | float | None = ...,
        heave_m: FieldState[float] | float | None = ...,
    ) -> Psxn: ...
    @property
    def id(self) -> FieldState[int]: ...
    @property
    def token(self) -> FieldState[bytes]: ...
    @property
    def roll_deg(self) -> FieldState[float]: ...
    @property
    def pitch_deg(self) -> FieldState[float]: ...
    @property
    def heave_m(self) -> FieldState[float]: ...

@final
class PrdidPitchRollHeading:
    def __new__(
        cls,
        pitch_deg: FieldState[float] | float | None = ...,
        roll_deg: FieldState[float] | float | None = ...,
        heading_deg: FieldState[float] | float | None = ...,
    ) -> PrdidPitchRollHeading: ...
    @property
    def pitch_deg(self) -> FieldState[float]: ...
    @property
    def roll_deg(self) -> FieldState[float]: ...
    @property
    def heading_deg(self) -> FieldState[float]: ...

@final
class PrdidRollPitchHeading:
    def __new__(
        cls,
        roll_deg: FieldState[float] | float | None = ...,
        pitch_deg: FieldState[float] | float | None = ...,
        heading_deg: FieldState[float] | float | None = ...,
    ) -> PrdidRollPitchHeading: ...
    @property
    def roll_deg(self) -> FieldState[float]: ...
    @property
    def pitch_deg(self) -> FieldState[float]: ...
    @property
    def heading_deg(self) -> FieldState[float]: ...

@final
class PrdidRaw:
    def __new__(cls, fields: list[bytes]) -> PrdidRaw: ...
    @property
    def fields(self) -> tuple[bytes, ...]: ...

@final
class Prdid:
    @staticmethod
    def pitch_roll_heading(
        pitch_deg: FieldState[float] | float | None = ...,
        roll_deg: FieldState[float] | float | None = ...,
        heading_deg: FieldState[float] | float | None = ...,
    ) -> Prdid: ...
    @staticmethod
    def roll_pitch_heading(
        roll_deg: FieldState[float] | float | None = ...,
        pitch_deg: FieldState[float] | float | None = ...,
        heading_deg: FieldState[float] | float | None = ...,
    ) -> Prdid: ...
    @staticmethod
    def raw(fields: list[bytes]) -> Prdid: ...
    @property
    def variant(self) -> str: ...
    @property
    def body(
        self,
    ) -> PrdidPitchRollHeading | PrdidRollPitchHeading | PrdidRaw: ...

Nmea0183Message: TypeAlias = Union[
    Gga, Gll, Hdt, Rmc, Vtg, Hdg, Ttm, Tll, Psxn, Prdid, Unknown
]

@final
class DecodeOptions:
    def __new__(cls) -> DecodeOptions: ...
    def with_psxn_layout(self, layout: PsxnLayout) -> DecodeOptions: ...
    def with_prdid_dialect(self, dialect: PrdidDialect) -> DecodeOptions: ...

@final
class _NmeaIterator:
    def __iter__(self) -> _NmeaIterator: ...
    def __next__(self) -> Nmea0183Message: ...

@final
class Nmea0183Parser:
    @staticmethod
    def one_shot(options: DecodeOptions | None = ...) -> Nmea0183Parser: ...
    @staticmethod
    def streaming(
        options: DecodeOptions | None = ...,
        max_size: int = ...,
    ) -> Nmea0183Parser: ...
    def feed(self, data: bytes) -> None: ...
    def next_message(self) -> Nmea0183Message | None: ...
    def __iter__(self) -> _NmeaIterator: ...
    def iter(self, strict: bool = ...) -> _NmeaIterator: ...
    def __enter__(self) -> Nmea0183Parser: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> Literal[False]: ...

def decode(raw: RawSentence) -> Nmea0183Message: ...
def decode_with(raw: RawSentence, options: DecodeOptions) -> Nmea0183Message: ...
def decode_gga(raw: RawSentence) -> Gga: ...
def decode_gll(raw: RawSentence) -> Gll: ...
def decode_vtg(raw: RawSentence) -> Vtg: ...
def decode_hdt(raw: RawSentence) -> Hdt: ...
def decode_hdg(raw: RawSentence) -> Hdg: ...
def decode_ttm(raw: RawSentence) -> Ttm: ...
def decode_tll(raw: RawSentence) -> Tll: ...
def decode_rmc(raw: RawSentence) -> Rmc: ...
def decode_psxn(raw: RawSentence, layout: PsxnLayout) -> Psxn: ...
def decode_prdid(raw: RawSentence, dialect: PrdidDialect) -> Prdid: ...

__all__ = [
    "AcquisitionType",
    "AngleReference",
    "DataStatus",
    "DecodeError",
    "DecodeOptions",
    "DistanceUnits",
    "Gga",
    "GgaFixQuality",
    "Gll",
    "Hdg",
    "Hdt",
    "Nmea0183Message",
    "Nmea0183Parser",
    "Prdid",
    "PrdidDialect",
    "PrdidPitchRollHeading",
    "PrdidRaw",
    "PrdidRollPitchHeading",
    "Psxn",
    "PsxnLayout",
    "PsxnSlot",
    "Rmc",
    "RmcNavStatus",
    "TargetStatus",
    "Tll",
    "Ttm",
    "Unknown",
    "UtcDate",
    "UtcTime",
    "Vtg",
    "VtgMode",
    "decode",
    "decode_gga",
    "decode_gll",
    "decode_hdg",
    "decode_hdt",
    "decode_prdid",
    "decode_psxn",
    "decode_rmc",
    "decode_tll",
    "decode_ttm",
    "decode_vtg",
    "decode_with",
]
