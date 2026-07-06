use alloc::vec::Vec;

use crate::ber::{be_u16, be_u64, ber_decode_len, ber_encode_len, read_bytes};
use crate::checksum::bcc;
use crate::error::Error;

/// MISB ST 0601 UAS Datalink Local Set 16-byte universal label (SMPTE UL key).
pub const UAS_LS_KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
];

/// MISB ST 0601 UAS Datalink Local Set — the typed core. Unrecognized 1-byte tags round-trip
/// via [`St0601::unknown`], so broadening the typed set is additive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct St0601 {
    /// Tag 2: precision timestamp, microseconds since the UNIX epoch (UTC). Mandatory —
    /// always encoded, always present in a successfully decoded set.
    pub timestamp_us: u64,
    /// Tag 65: UAS LS document version number.
    pub version: Option<u8>,
    /// Tag 5: platform heading, raw wire count (u16, 0..360° linear, full unsigned range).
    pub platform_heading: Option<u16>,
    /// Tag 6: platform pitch, raw wire count (i16, ±20° linear; `0x8000` = error sentinel).
    pub platform_pitch: Option<i16>,
    /// Tag 7: platform roll angle, raw wire count (i16, ±50° linear; `0x8000` = error sentinel).
    pub platform_roll: Option<i16>,
    /// Tag 8: platform true airspeed, raw wire count (u8, identity m/s).
    pub platform_true_airspeed: Option<u8>,
    /// Tag 13: sensor latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub sensor_latitude: Option<i32>,
    /// Tag 14: sensor longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub sensor_longitude: Option<i32>,
    /// Tag 15: sensor true altitude, raw wire count (u16, linear −900..19000 m MSL).
    pub sensor_true_altitude: Option<u16>,
    /// Tag 16: sensor horizontal field of view, raw wire count (u16, 0..180° linear).
    pub sensor_horizontal_fov: Option<u16>,
    /// Tag 17: sensor vertical field of view, raw wire count (u16, 0..180° linear).
    pub sensor_vertical_fov: Option<u16>,
    /// Tag 18: sensor relative azimuth, raw wire count (u32, 0..360° linear, full unsigned range).
    pub sensor_relative_azimuth: Option<u32>,
    /// Tag 19: sensor relative elevation, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub sensor_relative_elevation: Option<i32>,
    /// Tag 20: sensor relative roll, raw wire count (u32, 0..360° linear, full unsigned range).
    pub sensor_relative_roll: Option<u32>,
    /// Tag 21: slant range, raw wire count (u32, 0..5,000,000 m linear).
    pub slant_range: Option<u32>,
    /// Tag 22: target width, raw wire count (u16, 0..10,000 m linear).
    pub target_width: Option<u16>,
    /// Tag 23: frame center latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub frame_center_latitude: Option<i32>,
    /// Tag 24: frame center longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub frame_center_longitude: Option<i32>,
    /// Tag 25: frame center elevation, raw wire count (u16, linear −900..19000 m MSL).
    pub frame_center_elevation: Option<u16>,
    /// Tag 40: target location latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub target_location_latitude: Option<i32>,
    /// Tag 41: target location longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub target_location_longitude: Option<i32>,
    /// Tag 42: target location elevation, raw wire count (u16, linear −900..19000 m MSL).
    pub target_location_elevation: Option<u16>,
    /// Tags this crate does not type, preserved verbatim as `(tag, value)` in wire order
    /// and re-emitted on encode. A known tag decoded with an unexpected wire length also
    /// lands here (tolerant decode). Tag 2 (mandatory timestamp) is never routed here —
    /// a malformed Tag 2 fails the whole decode instead.
    pub unknown: Vec<(u8, Vec<u8>)>,
}
