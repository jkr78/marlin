"""Type stubs for the `marlin._core` extension module (PyO3-generated).

Hand-maintained; grows task-by-task. Each task that adds Rust-side bindings
also adds the matching signatures here. Task 15 will do a full pass/audit.
"""

from __future__ import annotations

from types import TracebackType

__version__: str

class MarlinError(Exception): ...
class EnvelopeError(MarlinError):
    variant: str

class DecodeError(MarlinError): ...
class AisError(MarlinError): ...
class ReassemblyError(AisError): ...

# --- Submodule: envelope ---

class _EnvelopeModule:
    class RawSentence:
        @property
        def start_delimiter(self) -> bytes: ...
        @property
        def talker(self) -> bytes | None: ...
        @property
        def sentence_type(self) -> str: ...
        @property
        def fields(self) -> tuple[bytes, ...]: ...
        @property
        def tag_block(self) -> bytes | None: ...
        @property
        def checksum_ok(self) -> bool: ...
        @property
        def raw(self) -> bytes: ...
        def as_dict(self) -> dict[str, object]: ...
        def __repr__(self) -> str: ...
        def __eq__(self, other: object) -> bool: ...
        def __hash__(self) -> int: ...

    class _EnvelopeIterator:
        def __iter__(self) -> "_EnvelopeModule._EnvelopeIterator": ...
        def __next__(self) -> "_EnvelopeModule.RawSentence": ...

    class OneShotParser:
        def __init__(self) -> None: ...
        def feed(self, data: bytes) -> None: ...
        def next_sentence(self) -> "_EnvelopeModule.RawSentence | None": ...
        def __iter__(self) -> "_EnvelopeModule._EnvelopeIterator": ...
        def iter(
            self, strict: bool = False
        ) -> "_EnvelopeModule._EnvelopeIterator": ...
        def __enter__(self) -> "_EnvelopeModule.OneShotParser": ...
        def __exit__(
            self,
            exc_type: type[BaseException] | None,
            exc_val: BaseException | None,
            exc_tb: TracebackType | None,
        ) -> bool: ...

    class StreamingParser:
        def __init__(self, max_size: int = 65_536) -> None: ...
        def feed(self, data: bytes) -> None: ...
        def next_sentence(self) -> "_EnvelopeModule.RawSentence | None": ...
        def __iter__(self) -> "_EnvelopeModule._EnvelopeIterator": ...
        def iter(
            self, strict: bool = False
        ) -> "_EnvelopeModule._EnvelopeIterator": ...
        def __enter__(self) -> "_EnvelopeModule.StreamingParser": ...
        def __exit__(
            self,
            exc_type: type[BaseException] | None,
            exc_val: BaseException | None,
            exc_tb: TracebackType | None,
        ) -> bool: ...

    @staticmethod
    def parse(data: bytes) -> "_EnvelopeModule.RawSentence": ...

envelope: _EnvelopeModule

# --- Submodule: nmea ---

class _NmeaModule:
    class GgaFixQuality:
        INVALID: "_NmeaModule.GgaFixQuality"
        GPS_FIX: "_NmeaModule.GgaFixQuality"
        DGPS_FIX: "_NmeaModule.GgaFixQuality"
        PPS_FIX: "_NmeaModule.GgaFixQuality"
        RTK_FIXED: "_NmeaModule.GgaFixQuality"
        RTK_FLOAT: "_NmeaModule.GgaFixQuality"
        DEAD_RECKONING: "_NmeaModule.GgaFixQuality"
        MANUAL_INPUT: "_NmeaModule.GgaFixQuality"
        SIMULATOR: "_NmeaModule.GgaFixQuality"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

    class VtgMode:
        NOT_VALID: "_NmeaModule.VtgMode"
        AUTONOMOUS: "_NmeaModule.VtgMode"
        DIFFERENTIAL: "_NmeaModule.VtgMode"
        ESTIMATED: "_NmeaModule.VtgMode"
        MANUAL: "_NmeaModule.VtgMode"
        SIMULATOR: "_NmeaModule.VtgMode"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

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
            utc: "_NmeaModule.UtcTime | None",
            latitude_deg: float | None,
            longitude_deg: float | None,
            fix_quality: "_NmeaModule.GgaFixQuality",
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
        def utc(self) -> "_NmeaModule.UtcTime | None": ...
        @property
        def latitude_deg(self) -> float | None: ...
        @property
        def longitude_deg(self) -> float | None: ...
        @property
        def fix_quality(self) -> "_NmeaModule.GgaFixQuality": ...
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
            mode: "_NmeaModule.VtgMode | None",
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
        def mode(self) -> "_NmeaModule.VtgMode | None": ...

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

    class PsxnSlot:
        ROLL: "_NmeaModule.PsxnSlot"
        PITCH: "_NmeaModule.PsxnSlot"
        HEAVE: "_NmeaModule.PsxnSlot"
        ROLL_SINE_ENCODED: "_NmeaModule.PsxnSlot"
        PITCH_SINE_ENCODED: "_NmeaModule.PsxnSlot"
        IGNORED: "_NmeaModule.PsxnSlot"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

    class PsxnLayout:
        @staticmethod
        def from_str(s: str) -> "_NmeaModule.PsxnLayout": ...

    class PrdidDialect:
        UNKNOWN: "_NmeaModule.PrdidDialect"
        PITCH_ROLL_HEADING: "_NmeaModule.PrdidDialect"
        ROLL_PITCH_HEADING: "_NmeaModule.PrdidDialect"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

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
        ) -> "_NmeaModule.Prdid": ...
        @staticmethod
        def roll_pitch_heading(
            roll_deg: float | None = ...,
            pitch_deg: float | None = ...,
            heading_deg: float | None = ...,
        ) -> "_NmeaModule.Prdid": ...
        @staticmethod
        def raw(fields: list[bytes]) -> "_NmeaModule.Prdid": ...
        @property
        def variant(self) -> str: ...
        @property
        def body(
            self,
        ) -> "_NmeaModule.PrdidPitchRollHeading | _NmeaModule.PrdidRollPitchHeading | _NmeaModule.PrdidRaw": ...

    class DecodeOptions:
        def __init__(self) -> None: ...
        def with_psxn_layout(
            self, layout: "_NmeaModule.PsxnLayout"
        ) -> "_NmeaModule.DecodeOptions": ...
        def with_prdid_dialect(
            self, dialect: "_NmeaModule.PrdidDialect"
        ) -> "_NmeaModule.DecodeOptions": ...

    class _NmeaIterator:
        def __iter__(self) -> "_NmeaModule._NmeaIterator": ...
        def __next__(
            self,
        ) -> "_NmeaModule.Gga | _NmeaModule.Vtg | _NmeaModule.Hdt | _NmeaModule.Psxn | _NmeaModule.Prdid | _NmeaModule.Unknown": ...

    class Nmea0183Parser:
        @staticmethod
        def one_shot(
            options: "_NmeaModule.DecodeOptions | None" = ...,
        ) -> "_NmeaModule.Nmea0183Parser": ...
        @staticmethod
        def streaming(
            options: "_NmeaModule.DecodeOptions | None" = ...,
            max_size: int = ...,
        ) -> "_NmeaModule.Nmea0183Parser": ...
        def feed(self, data: bytes) -> None: ...
        def next_message(
            self,
        ) -> "_NmeaModule.Gga | _NmeaModule.Vtg | _NmeaModule.Hdt | _NmeaModule.Psxn | _NmeaModule.Prdid | _NmeaModule.Unknown | None": ...
        def __iter__(self) -> "_NmeaModule._NmeaIterator": ...
        def iter(self, strict: bool = False) -> "_NmeaModule._NmeaIterator": ...
        def __enter__(self) -> "_NmeaModule.Nmea0183Parser": ...
        def __exit__(
            self,
            exc_type: type[BaseException] | None,
            exc_val: BaseException | None,
            exc_tb: TracebackType | None,
        ) -> bool: ...

    # Per-sentence decode extension points (Task 9).
    @staticmethod
    def decode(
        raw: "_EnvelopeModule.RawSentence",
    ) -> "_NmeaModule.Gga | _NmeaModule.Vtg | _NmeaModule.Hdt | _NmeaModule.Psxn | _NmeaModule.Prdid | _NmeaModule.Unknown": ...
    @staticmethod
    def decode_with(
        raw: "_EnvelopeModule.RawSentence",
        options: "_NmeaModule.DecodeOptions",
    ) -> "_NmeaModule.Gga | _NmeaModule.Vtg | _NmeaModule.Hdt | _NmeaModule.Psxn | _NmeaModule.Prdid | _NmeaModule.Unknown": ...
    @staticmethod
    def decode_gga(raw: "_EnvelopeModule.RawSentence") -> "_NmeaModule.Gga": ...
    @staticmethod
    def decode_vtg(raw: "_EnvelopeModule.RawSentence") -> "_NmeaModule.Vtg": ...
    @staticmethod
    def decode_hdt(raw: "_EnvelopeModule.RawSentence") -> "_NmeaModule.Hdt": ...
    @staticmethod
    def decode_psxn(
        raw: "_EnvelopeModule.RawSentence",
        layout: "_NmeaModule.PsxnLayout",
    ) -> "_NmeaModule.Psxn": ...
    @staticmethod
    def decode_prdid(
        raw: "_EnvelopeModule.RawSentence",
        dialect: "_NmeaModule.PrdidDialect",
    ) -> "_NmeaModule.Prdid": ...

nmea: _NmeaModule

# --- Submodule: ais ---

class _AisModule:
    class NavStatus:
        UNDERWAY_USING_ENGINE: "_AisModule.NavStatus"
        AT_ANCHOR: "_AisModule.NavStatus"
        NOT_UNDER_COMMAND: "_AisModule.NavStatus"
        RESTRICTED_MANEUVERABILITY: "_AisModule.NavStatus"
        CONSTRAINED_BY_DRAFT: "_AisModule.NavStatus"
        MOORED: "_AisModule.NavStatus"
        AGROUND: "_AisModule.NavStatus"
        ENGAGED_IN_FISHING: "_AisModule.NavStatus"
        UNDERWAY_SAILING: "_AisModule.NavStatus"
        AIS_SART_ACTIVE: "_AisModule.NavStatus"
        NOT_DEFINED: "_AisModule.NavStatus"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

    class ManeuverIndicator:
        NOT_AVAILABLE: "_AisModule.ManeuverIndicator"
        NO_SPECIAL: "_AisModule.ManeuverIndicator"
        SPECIAL: "_AisModule.ManeuverIndicator"
        RESERVED: "_AisModule.ManeuverIndicator"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

    class EpfdType:
        UNDEFINED: "_AisModule.EpfdType"
        GPS: "_AisModule.EpfdType"
        GLONASS: "_AisModule.EpfdType"
        COMBINED_GPS_GLONASS: "_AisModule.EpfdType"
        LORAN_C: "_AisModule.EpfdType"
        CHAYKA: "_AisModule.EpfdType"
        INTEGRATED_NAVIGATION: "_AisModule.EpfdType"
        SURVEYED: "_AisModule.EpfdType"
        GALILEO: "_AisModule.EpfdType"
        INTERNAL_GNSS: "_AisModule.EpfdType"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

    class AisVersion:
        ITU1371V1: "_AisModule.AisVersion"
        ITU1371V3: "_AisModule.AisVersion"
        ITU1371V5: "_AisModule.AisVersion"
        FUTURE: "_AisModule.AisVersion"
        def __int__(self) -> int: ...
        def __eq__(self, other: object) -> bool: ...

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

    # --- Task 11: typed message variants ---

    class PositionReportA:
        def __init__(
            self,
            mmsi: int = ...,
            navigation_status: "_AisModule.NavStatus" = ...,
            rate_of_turn: float | None = ...,
            speed_over_ground: float | None = ...,
            position_accuracy: bool = ...,
            longitude_deg: float | None = ...,
            latitude_deg: float | None = ...,
            course_over_ground: float | None = ...,
            true_heading: int | None = ...,
            timestamp: int = ...,
            special_maneuver: "_AisModule.ManeuverIndicator" = ...,
            raim: bool = ...,
            radio_status: int = ...,
        ) -> None: ...
        @property
        def mmsi(self) -> int: ...
        @property
        def navigation_status(self) -> "_AisModule.NavStatus": ...
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
        def special_maneuver(self) -> "_AisModule.ManeuverIndicator": ...
        @property
        def raim(self) -> bool: ...
        @property
        def radio_status(self) -> int: ...

    class StaticAndVoyageA:
        def __init__(
            self,
            mmsi: int = ...,
            ais_version: "_AisModule.AisVersion" = ...,
            imo_number: int | None = ...,
            call_sign: str | None = ...,
            vessel_name: str | None = ...,
            ship_type: int = ...,
            dimensions: "_AisModule.Dimensions | None" = ...,
            epfd: "_AisModule.EpfdType" = ...,
            eta: "_AisModule.Eta | None" = ...,
            draught_m: float | None = ...,
            destination: str | None = ...,
            dte: bool = ...,
        ) -> None: ...
        @property
        def mmsi(self) -> int: ...
        @property
        def ais_version(self) -> "_AisModule.AisVersion": ...
        @property
        def imo_number(self) -> int | None: ...
        @property
        def call_sign(self) -> str | None: ...
        @property
        def vessel_name(self) -> str | None: ...
        @property
        def ship_type(self) -> int: ...
        @property
        def dimensions(self) -> "_AisModule.Dimensions": ...
        @property
        def epfd(self) -> "_AisModule.EpfdType": ...
        @property
        def eta(self) -> "_AisModule.Eta": ...
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
            dimensions: "_AisModule.Dimensions | None" = ...,
            epfd: "_AisModule.EpfdType" = ...,
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
        def dimensions(self) -> "_AisModule.Dimensions": ...
        @property
        def epfd(self) -> "_AisModule.EpfdType": ...
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
            dimensions: "_AisModule.Dimensions | None" = ...,
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
        def dimensions(self) -> "_AisModule.Dimensions": ...

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

    # --- Task 14: AisParser with reassembly + clock modes ---

    class AisParser:
        @staticmethod
        def one_shot(
            timeout_ms: int | None = ...,
            clock: str | None = ...,
        ) -> "_AisModule.AisParser": ...
        @staticmethod
        def streaming(
            timeout_ms: int | None = ...,
            clock: str | None = ...,
            max_size: int = ...,
        ) -> "_AisModule.AisParser": ...
        def feed(self, data: bytes) -> None: ...
        def tick(self, now_ms: int) -> None: ...
        def next_message(self) -> "_AisModule.AisMessage | None": ...
        def __iter__(self) -> "_AisModule._AisIterator": ...
        def iter(self, strict: bool = False) -> "_AisModule._AisIterator": ...
        def __enter__(self) -> "_AisModule.AisParser": ...
        def __exit__(
            self,
            exc_type: type[BaseException] | None,
            exc_val: BaseException | None,
            exc_tb: TracebackType | None,
        ) -> bool: ...

    class _AisIterator:
        def __iter__(self) -> "_AisModule._AisIterator": ...
        def __next__(self) -> "_AisModule.AisMessage": ...

    # --- Task 13: BitReader primitive ---

    class BitReader:
        def __init__(self, data: bytes, total_bits: int) -> None: ...
        def u(self, n: int) -> int: ...
        def i(self, n: int) -> int: ...
        def b(self) -> bool: ...
        def string(self, chars: int) -> str: ...
        def remaining(self) -> int: ...

    # --- Task 12: AisMessage wrapper ---

    class AisMessage:
        def __init__(
            self,
            is_own_ship: bool,
            type_tag: str,
            body: "_AisModule.PositionReportA | _AisModule.StaticAndVoyageA | _AisModule.PositionReportB | _AisModule.ExtendedPositionReportB | _AisModule.StaticDataB24A | _AisModule.StaticDataB24B | _AisModule.Other",
        ) -> None: ...
        @property
        def is_own_ship(self) -> bool: ...
        @property
        def type_tag(self) -> str: ...
        @property
        def body(
            self,
        ) -> "_AisModule.PositionReportA | _AisModule.StaticAndVoyageA | _AisModule.PositionReportB | _AisModule.ExtendedPositionReportB | _AisModule.StaticDataB24A | _AisModule.StaticDataB24B | _AisModule.Other": ...

ais: _AisModule
