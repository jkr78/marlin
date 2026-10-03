//! Types, wire codes and field readers shared across AIS message variants.

use crate::BitReader;

/// Raw wire codes that mean "not available" or "this value or higher".
///
/// Re-exported as `marlin_ais::sentinel`. Decoders map the
/// not-available codes to `None`; over-range codes pass through as the
/// value they name (ADR-0001). Consumers that need to tell an over-range
/// floor from a measurement compare against these constants instead of
/// hardcoding the numbers.
pub mod sentinel {
    /// Longitude "not available": 181° in 1/10 000 minute (28-bit two's complement).
    pub const LON_NOT_AVAILABLE: i64 = 181 * 600_000;
    /// Latitude "not available": 91° in 1/10 000 minute (27-bit two's complement).
    pub const LAT_NOT_AVAILABLE: i64 = 91 * 600_000;
    /// Course over ground "not available", in 0.1°.
    pub const COG_NOT_AVAILABLE: u16 = 3600;
    /// True heading "not available", in whole degrees.
    pub const HEADING_NOT_AVAILABLE: u16 = 511;
    /// Vessel speed over ground "not available", in 0.1 kn (Types 1/2/3, 18, 19).
    pub const SOG_NOT_AVAILABLE: u16 = 1023;
    /// Vessel speed over ground "102.2 kn or higher", in 0.1 kn. Kept as a value.
    pub const SOG_OVER_RANGE: u16 = 1022;
    /// [`SOG_OVER_RANGE`] as the decoder emits it, in knots.
    pub const SOG_OVER_RANGE_KN: f32 = 1022.0 / 10.0;
    /// Rate of turn "not available" (8-bit two's complement).
    pub const ROT_NOT_AVAILABLE: i8 = -128;
    /// Rate of turn "turning right at more than 5° per 30 s, no turn indicator".
    pub const ROT_NO_INDICATOR_RIGHT: i8 = 127;
    /// Rate of turn "turning left at more than 5° per 30 s, no turn indicator".
    pub const ROT_NO_INDICATOR_LEFT: i8 = -127;
    /// SAR aircraft altitude "not available", in metres (Type 9).
    pub const ALTITUDE_NOT_AVAILABLE: u16 = 4095;
    /// SAR aircraft altitude "4094 m or higher". Kept as a value.
    pub const ALTITUDE_OVER_RANGE: u16 = 4094;
    /// SAR aircraft speed over ground "not available", in whole knots (Type 9).
    pub const AIRCRAFT_SOG_NOT_AVAILABLE: u16 = 1023;
    /// SAR aircraft speed over ground "1022 kn or higher", in whole knots. Kept as a value.
    pub const AIRCRAFT_SOG_OVER_RANGE: u16 = 1022;
    /// Dimension to bow / stern "511 m or greater" (9-bit fields). Kept as a value.
    pub const DIMENSION_LONG_OVER_RANGE: u16 = 511;
    /// Dimension to port / starboard "63 m or greater" (6-bit fields). Kept as a value.
    pub const DIMENSION_SHORT_OVER_RANGE: u8 = 63;
    /// Timestamp "not available".
    pub const TIMESTAMP_NOT_AVAILABLE: u8 = 60;
    /// Timestamp "positioning system in manual input mode".
    pub const TIMESTAMP_MANUAL_INPUT: u8 = 61;
    /// Timestamp "positioning system in estimated (dead reckoning) mode".
    pub const TIMESTAMP_DEAD_RECKONING: u8 = 62;
    /// Timestamp "positioning system inoperative".
    pub const TIMESTAMP_INOPERATIVE: u8 = 63;
}

/// Conversion factor from the on-wire ten-thousandths-of-a-minute to
/// decimal degrees (60 minutes × 10 000 = 600 000).
pub(crate) const MINUTES_FRAC_PER_DEGREE: f64 = 600_000.0;

/// Extent of a station from its position-reference point, as emitted
/// in Type 5 (Class A static), Type 19 (Class B extended position
/// report) and Type 24 Part B (Class B static part B).
///
/// The four fields measure the distances from the position-reference
/// point (typically the antenna) to the bow, stern, port, and
/// starboard edges of the vessel, in metres. `0` is the "not
/// available" sentinel and maps to `None`; the maximum of each field
/// (511 m or 63 m) means "this value or greater" and is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimensions {
    /// Distance from position-reference to bow, in metres.
    /// Wire width: 9 bits, max 511 m. `None` on sentinel `0`.
    pub to_bow_m: Option<u16>,
    /// Distance from position-reference to stern, in metres.
    /// Wire width: 9 bits, max 511 m. `None` on sentinel `0`.
    pub to_stern_m: Option<u16>,
    /// Distance from position-reference to port side, in metres.
    /// Wire width: 6 bits, max 63 m. `None` on sentinel `0`.
    pub to_port_m: Option<u8>,
    /// Distance from position-reference to starboard side, in metres.
    /// Wire width: 6 bits, max 63 m. `None` on sentinel `0`.
    pub to_starboard_m: Option<u8>,
}

/// Electronic Position-Fixing Device type — 4-bit field in Types 5 and 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpfdType {
    /// 0 — undefined (default).
    Undefined,
    /// 1 — GPS.
    Gps,
    /// 2 — GLONASS.
    Glonass,
    /// 3 — Combined GPS/GLONASS.
    CombinedGpsGlonass,
    /// 4 — Loran-C.
    LoranC,
    /// 5 — Chayka.
    Chayka,
    /// 6 — Integrated navigation system.
    IntegratedNavigation,
    /// 7 — Surveyed position.
    Surveyed,
    /// 8 — Galileo.
    Galileo,
    /// 15 — Internal GNSS.
    InternalGnss,
    /// 9..=14 reserved; carried as raw byte.
    Reserved(u8),
}

impl EpfdType {
    pub(crate) fn from_u4(v: u8) -> Self {
        match v {
            0 => Self::Undefined,
            1 => Self::Gps,
            2 => Self::Glonass,
            3 => Self::CombinedGpsGlonass,
            4 => Self::LoranC,
            5 => Self::Chayka,
            6 => Self::IntegratedNavigation,
            7 => Self::Surveyed,
            8 => Self::Galileo,
            15 => Self::InternalGnss,
            other => Self::Reserved(other),
        }
    }

    /// The 4-bit wire code this variant was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Undefined => 0,
            Self::Gps => 1,
            Self::Glonass => 2,
            Self::CombinedGpsGlonass => 3,
            Self::LoranC => 4,
            Self::Chayka => 5,
            Self::IntegratedNavigation => 6,
            Self::Surveyed => 7,
            Self::Galileo => 8,
            Self::InternalGnss => 15,
            Self::Reserved(c) => c,
        }
    }
}

/// Whether `mmsi` identifies an auxiliary craft attached to a mother
/// ship (`98MIDxxxx`, the gpsd and USCG convention; ADR-0002).
///
/// A Type 24 Part B from such an MMSI carries the mother ship's MMSI in
/// the bits that otherwise hold dimensions.
#[must_use]
pub const fn is_auxiliary_craft_mmsi(mmsi: u32) -> bool {
    mmsi / 10_000_000 == 98
}

/// Trim an AIS 6-bit string (decoded from [`crate::BitReader::string`]),
/// returning `None` when the content is entirely padding.
///
/// AIS pads short strings with `@` (value 0) at the end and sometimes
/// with trailing spaces. A field consisting solely of these characters
/// means "not available".
pub(crate) fn trim_ais_string(s: alloc::string::String) -> Option<alloc::string::String> {
    let trimmed_len = s.trim_end_matches(['@', ' ']).len();
    if trimmed_len == 0 {
        None
    } else {
        let mut out = s;
        out.truncate(trimmed_len);
        Some(out)
    }
}

/// Decode a 28-bit two's-complement longitude in 1/10 000 minute to
/// signed decimal degrees. `None` on [`sentinel::LON_NOT_AVAILABLE`].
#[allow(clippy::cast_precision_loss)] // 28-bit raw is exact in f64
pub(crate) fn lon_deg(raw: i64) -> Option<f64> {
    if raw == sentinel::LON_NOT_AVAILABLE {
        None
    } else {
        Some((raw as f64) / MINUTES_FRAC_PER_DEGREE)
    }
}

/// Decode a 27-bit two's-complement latitude in 1/10 000 minute to
/// signed decimal degrees. `None` on [`sentinel::LAT_NOT_AVAILABLE`].
#[allow(clippy::cast_precision_loss)] // 27-bit raw is exact in f64
pub(crate) fn lat_deg(raw: i64) -> Option<f64> {
    if raw == sentinel::LAT_NOT_AVAILABLE {
        None
    } else {
        Some((raw as f64) / MINUTES_FRAC_PER_DEGREE)
    }
}

/// Decode a 12-bit course over ground in 0.1° to degrees.
/// `None` on [`sentinel::COG_NOT_AVAILABLE`].
#[allow(clippy::cast_precision_loss)] // 12-bit raw is exact in f32
pub(crate) fn cog_deg(raw: u64) -> Option<f32> {
    if raw == u64::from(sentinel::COG_NOT_AVAILABLE) {
        None
    } else {
        Some((raw as f32) / 10.0)
    }
}

/// Decode a 9-bit true heading in whole degrees.
/// `None` on [`sentinel::HEADING_NOT_AVAILABLE`].
#[allow(clippy::cast_possible_truncation)] // masked to 9 bits
pub(crate) fn heading_deg(raw: u64) -> Option<u16> {
    if raw == u64::from(sentinel::HEADING_NOT_AVAILABLE) {
        None
    } else {
        Some((raw & 0x1FF) as u16)
    }
}

/// Decode a 10-bit vessel speed over ground in 0.1 kn to knots.
/// `None` on [`sentinel::SOG_NOT_AVAILABLE`]; [`sentinel::SOG_OVER_RANGE`]
/// passes through as 102.2.
#[allow(clippy::cast_precision_loss)] // 10-bit raw is exact in f32
pub(crate) fn sog_tenths_kn(raw: u64) -> Option<f32> {
    if raw == u64::from(sentinel::SOG_NOT_AVAILABLE) {
        None
    } else {
        Some((raw as f32) / 10.0)
    }
}

/// Decode a whole-unit field of at most 16 bits (Type 9 altitude in
/// metres, aircraft SOG in knots), mapping `not_available` to `None`
/// and passing every other value, over-range codes included, through.
#[allow(clippy::cast_possible_truncation)] // callers pass fields of at most 16 bits
pub(crate) fn whole_u16(raw: u64, not_available: u16) -> Option<u16> {
    if raw == u64::from(not_available) {
        None
    } else {
        Some((raw & 0xFFFF) as u16)
    }
}

/// Read the 30-bit dimension block (A 9, B 9, C 6, D 6 bits) at the
/// reader's current position.
pub(crate) fn read_dimensions(r: &mut BitReader<'_>) -> Dimensions {
    Dimensions {
        to_bow_m: dim_u9(r.u(9)),
        to_stern_m: dim_u9(r.u(9)),
        to_port_m: dim_u6(r.u(6)),
        to_starboard_m: dim_u6(r.u(6)),
    }
}

/// Decode a 9-bit dimension field to `Option<u16>` (sentinel `0` → `None`).
#[allow(clippy::cast_possible_truncation)] // masked to 9 bits
fn dim_u9(raw: u64) -> Option<u16> {
    if raw == 0 {
        None
    } else {
        Some((raw & 0x1FF) as u16)
    }
}

/// Decode a 6-bit dimension field to `Option<u8>` (sentinel `0` → `None`).
#[allow(clippy::cast_possible_truncation)] // masked to 6 bits
fn dim_u6(raw: u64) -> Option<u8> {
    if raw == 0 {
        None
    } else {
        Some((raw & 0x3F) as u8)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::testing::BitWriter;

    #[test]
    fn lon_lat_sentinels_decode_to_none() {
        assert_eq!(lon_deg(sentinel::LON_NOT_AVAILABLE), None);
        assert_eq!(lat_deg(sentinel::LAT_NOT_AVAILABLE), None);
    }

    #[test]
    fn lon_lat_scale_and_keep_sign() {
        assert!((lon_deg(6_600_000).unwrap() - 11.0).abs() < 1e-9);
        assert!((lat_deg(-2_880_000).unwrap() + 4.8).abs() < 1e-9);
    }

    #[test]
    fn cog_sentinel_is_none_and_tenths_scale() {
        assert_eq!(cog_deg(u64::from(sentinel::COG_NOT_AVAILABLE)), None);
        assert!((cog_deg(1234).unwrap() - 123.4).abs() < 1e-4);
    }

    #[test]
    fn heading_sentinel_is_none_and_value_passes() {
        assert_eq!(
            heading_deg(u64::from(sentinel::HEADING_NOT_AVAILABLE)),
            None
        );
        assert_eq!(heading_deg(359), Some(359));
    }

    #[test]
    fn sog_sentinel_is_none_and_over_range_is_kept() {
        assert_eq!(sog_tenths_kn(u64::from(sentinel::SOG_NOT_AVAILABLE)), None);
        let over = sog_tenths_kn(u64::from(sentinel::SOG_OVER_RANGE)).unwrap();
        assert!((over - sentinel::SOG_OVER_RANGE_KN).abs() < 1e-6);
        assert!((over - 102.2).abs() < 1e-4);
    }

    #[test]
    fn whole_u16_sentinel_is_none_and_over_range_is_kept() {
        assert_eq!(
            whole_u16(
                u64::from(sentinel::ALTITUDE_NOT_AVAILABLE),
                sentinel::ALTITUDE_NOT_AVAILABLE
            ),
            None
        );
        assert_eq!(
            whole_u16(
                u64::from(sentinel::ALTITUDE_OVER_RANGE),
                sentinel::ALTITUDE_NOT_AVAILABLE
            ),
            Some(4094)
        );
    }

    #[test]
    fn read_dimensions_zeros_are_none() {
        let mut w = BitWriter::new();
        w.u(30, 0);
        let (bits, total) = w.finish();
        let mut r = BitReader::new(&bits, total);
        assert_eq!(read_dimensions(&mut r), Dimensions::default());
    }

    #[test]
    fn read_dimensions_keeps_over_range_maxima() {
        let mut w = BitWriter::new();
        w.u(9, u64::from(sentinel::DIMENSION_LONG_OVER_RANGE));
        w.u(9, 10);
        w.u(6, u64::from(sentinel::DIMENSION_SHORT_OVER_RANGE));
        w.u(6, 3);
        let (bits, total) = w.finish();
        let mut r = BitReader::new(&bits, total);
        assert_eq!(
            read_dimensions(&mut r),
            Dimensions {
                to_bow_m: Some(511),
                to_stern_m: Some(10),
                to_port_m: Some(63),
                to_starboard_m: Some(3),
            }
        );
    }

    #[test]
    fn epfd_code_round_trips_every_variant() {
        for code in 0u8..16 {
            assert_eq!(EpfdType::from_u4(code).code(), code);
        }
        assert_eq!(EpfdType::Reserved(9).code(), 9);
    }

    #[test]
    fn auxiliary_craft_mmsi_boundaries() {
        assert!(!is_auxiliary_craft_mmsi(979_999_999));
        assert!(is_auxiliary_craft_mmsi(980_000_000));
        assert!(is_auxiliary_craft_mmsi(989_999_999));
        assert!(!is_auxiliary_craft_mmsi(990_000_000));
    }

    #[test]
    fn trim_ais_string_strips_padding_and_spaces() {
        assert_eq!(trim_ais_string("NAME@@  @".into()).as_deref(), Some("NAME"));
        assert_eq!(trim_ais_string("@@@@".into()), None);
    }
}
