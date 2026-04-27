"""Type stubs for marlin.nmea — typed NMEA 0183 decoders."""

from __future__ import annotations

from types import TracebackType
from typing import Union

from typing_extensions import TypeAlias

from .. import MarlinError
from ..envelope import RawSentence

class DecodeError(MarlinError): ...

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
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class VtgMode:
    NOT_VALID: VtgMode
    AUTONOMOUS: VtgMode
    DIFFERENTIAL: VtgMode
    ESTIMATED: VtgMode
    MANUAL: VtgMode
    SIMULATOR: VtgMode
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class PsxnSlot:
    ROLL: PsxnSlot
    PITCH: PsxnSlot
    HEAVE: PsxnSlot
    ROLL_SINE_ENCODED: PsxnSlot
    PITCH_SINE_ENCODED: PsxnSlot
    IGNORED: PsxnSlot
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class PrdidDialect:
    UNKNOWN: PrdidDialect
    PITCH_ROLL_HEADING: PrdidDialect
    ROLL_PITCH_HEADING: PrdidDialect
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class PsxnLayout:
    @staticmethod
    def from_str(s: str) -> PsxnLayout: ...

class UtcTime:
    def __init__(
        self, hour: int, minute: int, second: int, millisecond: int
    ) -> None: ...
    @property
    def hour(self) -> int: ...
    @property
    def minute(self) -> int: ...
    @property
    def second(self) -> int: ...
    @property
    def millisecond(self) -> int: ...

class Gga:
    def __init__(
        self,
        *,
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
    ) -> None: ...
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

class Vtg:
    def __init__(
        self,
        *,
        talker: bytes | None,
        course_true_deg: float | None,
        course_magnetic_deg: float | None,
        speed_knots: float | None,
        speed_kmh: float | None,
        mode: VtgMode | None,
    ) -> None: ...
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

class Hdt:
    def __init__(
        self,
        *,
        talker: bytes | None,
        heading_true_deg: float | None,
    ) -> None: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def heading_true_deg(self) -> float | None: ...

class Unknown:
    def __init__(
        self, *, talker: bytes | None, sentence_type: str
    ) -> None: ...
    @property
    def talker(self) -> bytes | None: ...
    @property
    def sentence_type(self) -> str: ...

class Psxn:
    def __init__(
        self,
        id: int | None = ...,
        token: bytes | None = ...,
        roll_deg: float | None = ...,
        pitch_deg: float | None = ...,
        heave_m: float | None = ...,
    ) -> None: ...
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

class PrdidPitchRollHeading:
    def __init__(
        self,
        pitch_deg: float | None = ...,
        roll_deg: float | None = ...,
        heading_deg: float | None = ...,
    ) -> None: ...
    @property
    def pitch_deg(self) -> float | None: ...
    @property
    def roll_deg(self) -> float | None: ...
    @property
    def heading_deg(self) -> float | None: ...

class PrdidRollPitchHeading:
    def __init__(
        self,
        roll_deg: float | None = ...,
        pitch_deg: float | None = ...,
        heading_deg: float | None = ...,
    ) -> None: ...
    @property
    def roll_deg(self) -> float | None: ...
    @property
    def pitch_deg(self) -> float | None: ...
    @property
    def heading_deg(self) -> float | None: ...

class PrdidRaw:
    def __init__(self, fields: list[bytes]) -> None: ...
    @property
    def fields(self) -> tuple[bytes, ...]: ...

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

Nmea0183Message: TypeAlias = Union[Gga, Vtg, Hdt, Psxn, Prdid, Unknown]

class DecodeOptions:
    def __init__(self) -> None: ...
    def with_psxn_layout(self, layout: PsxnLayout) -> DecodeOptions: ...
    def with_prdid_dialect(self, dialect: PrdidDialect) -> DecodeOptions: ...

class _NmeaIterator:
    def __iter__(self) -> _NmeaIterator: ...
    def __next__(self) -> Nmea0183Message: ...

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
    ) -> bool: ...

def decode(raw: RawSentence) -> Nmea0183Message: ...
def decode_with(raw: RawSentence, options: DecodeOptions) -> Nmea0183Message: ...
def decode_gga(raw: RawSentence) -> Gga: ...
def decode_vtg(raw: RawSentence) -> Vtg: ...
def decode_hdt(raw: RawSentence) -> Hdt: ...
def decode_psxn(raw: RawSentence, layout: PsxnLayout) -> Psxn: ...
def decode_prdid(raw: RawSentence, dialect: PrdidDialect) -> Prdid: ...

__all__ = [
    "DecodeError",
    "DecodeOptions",
    "Gga",
    "GgaFixQuality",
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
    "Unknown",
    "UtcTime",
    "Vtg",
    "VtgMode",
    "decode",
    "decode_gga",
    "decode_hdt",
    "decode_prdid",
    "decode_psxn",
    "decode_vtg",
    "decode_with",
]
