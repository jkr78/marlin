"""Type stubs for marlin.nmea — typed NMEA 0183 decoders."""

from __future__ import annotations

from types import TracebackType
from typing import Literal, Union, final

from typing_extensions import TypeAlias

from .. import MarlinError
from ..envelope import RawSentence

class DecodeError(MarlinError): ...

@final
class GgaFixQuality:
    INVALID: GgaFixQuality
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
    UNKNOWN: TargetStatus
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AngleReference:
    TRUE: AngleReference
    RELATIVE: AngleReference
    UNKNOWN: AngleReference
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class DistanceUnits:
    NAUTICAL: DistanceUnits
    KILOMETERS: DistanceUnits
    STATUTE: DistanceUnits
    UNKNOWN: DistanceUnits
    def __int__(self) -> int: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...

@final
class AcquisitionType:
    AUTOMATIC: AcquisitionType
    MANUAL: AcquisitionType
    REPORTED: AcquisitionType
    UNKNOWN: AcquisitionType
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

@final
class Gga:
    def __new__(
        cls,
        talker: bytes | None,
        utc: UtcTime | None,
        latitude_deg: float | None,
        longitude_deg: float | None,
        fix_quality: GgaFixQuality,
        satellites_used: int | None,
        hdop: float | None,
        altitude_m: float | None,
        geoid_separation_m: float | None,
        dgps_age_s: float | None,
        dgps_station_id: int | None,
    ) -> Gga: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def utc(self) -> UtcTime | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def fix_quality(self) -> GgaFixQuality: ...
    @property
    def satellites_used(self) -> int | None: ...
    @property
    def hdop(self) -> float | None: ...
    @property
    def altitude_m(self) -> float | None: ...
    @property
    def geoid_separation_m(self) -> float | None: ...
    @property
    def dgps_age_s(self) -> float | None: ...
    @property
    def dgps_station_id(self) -> int | None: ...

@final
class Vtg:
    def __new__(
        cls,
        talker: bytes | None,
        course_true_deg: float | None,
        course_magnetic_deg: float | None,
        speed_knots: float | None,
        speed_kmh: float | None,
        mode: VtgMode | None,
    ) -> Vtg: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def course_true_deg(self) -> float | None: ...
    @property
    def course_magnetic_deg(self) -> float | None: ...
    @property
    def speed_knots(self) -> float | None: ...
    @property
    def speed_kmh(self) -> float | None: ...
    @property
    def mode(self) -> VtgMode | None: ...

@final
class Hdt:
    def __new__(
        cls,
        talker: bytes | None,
        heading_true_deg: float | None,
    ) -> Hdt: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def heading_true_deg(self) -> float | None: ...

@final
class Hdg:
    def __new__(
        cls,
        talker: bytes | None,
        heading_magnetic_deg: float | None,
        deviation_deg: float | None,
        variation_deg: float | None,
    ) -> Hdg: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def heading_magnetic_deg(self) -> float | None: ...
    @property
    def deviation_deg(self) -> float | None: ...
    @property
    def variation_deg(self) -> float | None: ...

@final
class Ttm:
    @property
    def talker(self) -> bytes | None: ...
    @property
    def target_number(self) -> int | None: ...
    @property
    def distance(self) -> float | None: ...
    @property
    def bearing_deg(self) -> float | None: ...
    @property
    def bearing_reference(self) -> AngleReference | None: ...
    @property
    def speed(self) -> float | None: ...
    @property
    def course_deg(self) -> float | None: ...
    @property
    def course_reference(self) -> AngleReference | None: ...
    @property
    def cpa(self) -> float | None: ...
    @property
    def tcpa(self) -> float | None: ...
    @property
    def units(self) -> DistanceUnits | None: ...
    @property
    def name(self) -> str | None: ...
    @property
    def status(self) -> TargetStatus | None: ...
    @property
    def reference_target(self) -> bool: ...
    @property
    def utc_time(self) -> UtcTime | None: ...
    @property
    def acquisition(self) -> AcquisitionType | None: ...

@final
class Tll:
    @property
    def talker(self) -> bytes | None: ...
    @property
    def target_number(self) -> int | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def name(self) -> str | None: ...
    @property
    def utc_time(self) -> UtcTime | None: ...
    @property
    def status(self) -> TargetStatus | None: ...
    @property
    def reference_target(self) -> bool: ...

@final
class Rmc:
    def __new__(
        cls,
        talker: bytes | None,
        utc: UtcTime | None,
        status: DataStatus,
        latitude_deg: float | None,
        longitude_deg: float | None,
        speed_knots: float | None,
        course_true_deg: float | None,
        date: UtcDate | None,
        magnetic_variation_deg: float | None,
        mode: VtgMode | None,
        nav_status: RmcNavStatus | None,
    ) -> Rmc: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def utc(self) -> UtcTime | None: ...
    @property
    def status(self) -> DataStatus: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def speed_knots(self) -> float | None: ...
    @property
    def course_true_deg(self) -> float | None: ...
    @property
    def date(self) -> UtcDate | None: ...
    @property
    def magnetic_variation_deg(self) -> float | None: ...
    @property
    def mode(self) -> VtgMode | None: ...
    @property
    def nav_status(self) -> RmcNavStatus | None: ...

@final
class Gll:
    def __new__(
        cls,
        talker: bytes | None,
        latitude_deg: float | None,
        longitude_deg: float | None,
        utc: UtcTime | None,
        status: DataStatus,
        mode: VtgMode | None,
    ) -> Gll: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def latitude_deg(self) -> float | None: ...
    @property
    def longitude_deg(self) -> float | None: ...
    @property
    def utc(self) -> UtcTime | None: ...
    @property
    def status(self) -> DataStatus: ...
    @property
    def mode(self) -> VtgMode | None: ...

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
        id: int | None = ...,
        token: bytes | None = ...,
        roll_deg: float | None = ...,
        pitch_deg: float | None = ...,
        heave_m: float | None = ...,
    ) -> Psxn: ...
    @property
    def id(self) -> int | None: ...
    @property
    def token(self) -> bytes | None: ...
    @property
    def roll_deg(self) -> float | None: ...
    @property
    def pitch_deg(self) -> float | None: ...
    @property
    def heave_m(self) -> float | None: ...

@final
class PrdidPitchRollHeading:
    def __new__(
        cls,
        pitch_deg: float | None = ...,
        roll_deg: float | None = ...,
        heading_deg: float | None = ...,
    ) -> PrdidPitchRollHeading: ...
    @property
    def pitch_deg(self) -> float | None: ...
    @property
    def roll_deg(self) -> float | None: ...
    @property
    def heading_deg(self) -> float | None: ...

@final
class PrdidRollPitchHeading:
    def __new__(
        cls,
        roll_deg: float | None = ...,
        pitch_deg: float | None = ...,
        heading_deg: float | None = ...,
    ) -> PrdidRollPitchHeading: ...
    @property
    def roll_deg(self) -> float | None: ...
    @property
    def pitch_deg(self) -> float | None: ...
    @property
    def heading_deg(self) -> float | None: ...

@final
class PrdidRaw:
    def __new__(cls, fields: list[bytes]) -> PrdidRaw: ...
    @property
    def fields(self) -> tuple[bytes, ...]: ...

@final
class Prdid:
    @staticmethod
    def pitch_roll_heading(
        pitch_deg: float | None = ...,
        roll_deg: float | None = ...,
        heading_deg: float | None = ...,
    ) -> Prdid: ...
    @staticmethod
    def roll_pitch_heading(
        roll_deg: float | None = ...,
        pitch_deg: float | None = ...,
        heading_deg: float | None = ...,
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
