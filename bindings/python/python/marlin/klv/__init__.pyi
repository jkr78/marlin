"""Type stubs for marlin.klv — MISB ST 0601 (UAS Datalink Local Set) KLV."""

from __future__ import annotations

from typing import Literal, final

from .. import MarlinError
from ..field import FieldState

class KlvError(MarlinError):
    """A KLV decode failed for a structural reason: framing, the checksum,
    or the mandatory Tag 2 timestamp absent or malformed. A field's value
    never fails the set, it decodes to a `FieldState`. `variant` names the
    reason: `"truncated"`, `"length_overflow"`, `"bad_checksum"`,
    `"missing_checksum"`, `"bad_timestamp"`, `"bad_key"` or `"other"`."""

    variant: str

class KlvEncodeError(KlvError):
    """A KLV encode failed: `tag` is in a field state the wire cannot carry
    (`variant` `"unencodable"`, `kind` the state's kind such as
    `"at_least"`), or holds a value outside the tag's range or NaN
    (`variant` `"out_of_range"`, `kind` `None`)."""

    # Narrows the base `str`: the attribute is set once by the raising code and
    # never reassigned, so the invariance pyright guards against cannot bite.
    variant: Literal["unencodable", "out_of_range"]  # pyright: ignore[reportIncompatibleVariableOverride]
    tag: int
    kind: str | None

@final
class St0601:
    """Mutable MISB ST 0601 local set. Construct with the mandatory
    `timestamp_us`, assign fields, then `encode(...)`; or obtain one from
    `decode(...)`.

    Every scaled tag is a `FieldState[float]` property in engineering
    units: an omitted tag reads `FieldState.NotAvailable()`, the ST 0601
    sentinel on a signed tag `FieldState.SenderError(code)`, a
    known tag with the wrong wire length `FieldState.Invalid(None)` with
    its bytes kept in `unknown`. A setter takes a `FieldState`, a bare
    number (which becomes `FieldState.Value`) or `None`
    (`FieldState.NotAvailable()`); nothing is clamped, `encode` rejects a
    value outside the tag's range."""

    def __new__(
        cls,
        timestamp_us: int,
        version: FieldState[int] | int | None = ...,
    ) -> St0601: ...
    # Tag 2: precision timestamp, microseconds since the UNIX epoch (UTC). Mandatory.
    timestamp_us: int
    # Tag 65: UAS LS document version number.
    @property
    def version(self) -> FieldState[int]: ...
    @version.setter
    def version(self, value: FieldState[int] | int | None) -> None: ...
    @property
    def platform_heading_degrees(self) -> FieldState[float]:
        """Tag 5: platform heading in degrees (0..=360)."""
    @platform_heading_degrees.setter
    def platform_heading_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def platform_pitch_degrees(self) -> FieldState[float]:
        """Tag 6: platform pitch in degrees (-20..=20); the sentinel reads
        `FieldState.SenderError(-32768)`.
        """
    @platform_pitch_degrees.setter
    def platform_pitch_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def platform_roll_degrees(self) -> FieldState[float]:
        """Tag 7: platform roll in degrees (-50..=50); the sentinel reads
        `FieldState.SenderError(-32768)`.
        """
    @platform_roll_degrees.setter
    def platform_roll_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def platform_true_airspeed_mps(self) -> FieldState[float]:
        """Tag 8: platform true airspeed in m/s (0..=255)."""
    @platform_true_airspeed_mps.setter
    def platform_true_airspeed_mps(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_latitude_degrees(self) -> FieldState[float]:
        """Tag 13: sensor latitude in degrees WGS84 (-90..=90); the sentinel reads
        `FieldState.SenderError(-2147483648)`.
        """
    @sensor_latitude_degrees.setter
    def sensor_latitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_longitude_degrees(self) -> FieldState[float]:
        """Tag 14: sensor longitude in degrees WGS84 (-180..=180); the sentinel reads
        `FieldState.SenderError(-2147483648)`.
        """
    @sensor_longitude_degrees.setter
    def sensor_longitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_true_altitude_meters(self) -> FieldState[float]:
        """Tag 15: sensor true altitude in meters MSL (-900..=19000)."""
    @sensor_true_altitude_meters.setter
    def sensor_true_altitude_meters(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_horizontal_fov_degrees(self) -> FieldState[float]:
        """Tag 16: sensor horizontal field of view in degrees (0..=180)."""
    @sensor_horizontal_fov_degrees.setter
    def sensor_horizontal_fov_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_vertical_fov_degrees(self) -> FieldState[float]:
        """Tag 17: sensor vertical field of view in degrees (0..=180)."""
    @sensor_vertical_fov_degrees.setter
    def sensor_vertical_fov_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_relative_azimuth_degrees(self) -> FieldState[float]:
        """Tag 18: sensor relative azimuth in degrees (0..=360)."""
    @sensor_relative_azimuth_degrees.setter
    def sensor_relative_azimuth_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_relative_elevation_degrees(self) -> FieldState[float]:
        """Tag 19: sensor relative elevation in degrees (-180..=180, negative = below
        horizon); the sentinel reads `FieldState.SenderError(-2147483648)`.
        """
    @sensor_relative_elevation_degrees.setter
    def sensor_relative_elevation_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def sensor_relative_roll_degrees(self) -> FieldState[float]:
        """Tag 20: sensor relative roll in degrees (0..=360, clockwise from behind the
        camera).
        """
    @sensor_relative_roll_degrees.setter
    def sensor_relative_roll_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def slant_range_meters(self) -> FieldState[float]:
        """Tag 21: slant range in meters (0..=5000000)."""
    @slant_range_meters.setter
    def slant_range_meters(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def target_width_meters(self) -> FieldState[float]:
        """Tag 22: target width in meters (0..=10000)."""
    @target_width_meters.setter
    def target_width_meters(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def frame_center_latitude_degrees(self) -> FieldState[float]:
        """Tag 23: frame center latitude in degrees WGS84 (-90..=90); the sentinel reads
        `FieldState.SenderError(-2147483648)`.
        """
    @frame_center_latitude_degrees.setter
    def frame_center_latitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def frame_center_longitude_degrees(self) -> FieldState[float]:
        """Tag 24: frame center longitude in degrees WGS84 (-180..=180); the sentinel reads
        `FieldState.SenderError(-2147483648)`.
        """
    @frame_center_longitude_degrees.setter
    def frame_center_longitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def frame_center_elevation_meters(self) -> FieldState[float]:
        """Tag 25: frame center elevation in meters MSL (-900..=19000)."""
    @frame_center_elevation_meters.setter
    def frame_center_elevation_meters(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def target_location_latitude_degrees(self) -> FieldState[float]:
        """Tag 40: target location latitude in degrees WGS84 (-90..=90); the sentinel reads
        `FieldState.SenderError(-2147483648)`.
        """
    @target_location_latitude_degrees.setter
    def target_location_latitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def target_location_longitude_degrees(self) -> FieldState[float]:
        """Tag 41: target location longitude in degrees WGS84 (-180..=180); the sentinel
        reads `FieldState.SenderError(-2147483648)`.
        """
    @target_location_longitude_degrees.setter
    def target_location_longitude_degrees(self, value: FieldState[float] | float | None) -> None: ...
    @property
    def target_location_elevation_meters(self) -> FieldState[float]:
        """Tag 42: target location elevation in meters MSL (-900..=19000)."""
    @target_location_elevation_meters.setter
    def target_location_elevation_meters(self, value: FieldState[float] | float | None) -> None: ...
    # Tags the codec does not type, as (tag, bytes) in wire order, plus the
    # bytes of any known tag that arrived with the wrong wire length.
    @property
    def unknown(self) -> list[tuple[int, bytes]]: ...

@final
class TagInfo:
    """Read-only metadata for one typed ST 0601 tag: wire number, field base
    name (e.g. ``sensor_latitude``), and engineering unit (``None`` if none)."""

    number: int
    name: str
    unit: str | None

# The 16-byte UAS Datalink Local Set Universal Label every ST 0601 packet is framed with.
UAS_LS_KEY: bytes

def decode(data: bytes) -> St0601: ...
def encode(set: St0601) -> bytes: ...
def precision_timestamp(data: bytes) -> int | None: ...
def tags() -> list[TagInfo]: ...
def tag_number(name: str) -> int | None: ...
def tag_name(number: int) -> str | None: ...

__all__ = [
    "UAS_LS_KEY",
    "KlvEncodeError",
    "KlvError",
    "St0601",
    "TagInfo",
    "decode",
    "encode",
    "precision_timestamp",
    "tag_name",
    "tag_number",
    "tags",
]
