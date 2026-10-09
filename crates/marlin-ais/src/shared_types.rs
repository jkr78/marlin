//! Types, wire codes and field readers shared across AIS message variants.

use alloc::string::String;
use core::ops::RangeInclusive;

use marlin_field::{FieldState, Invalid, RawCode};

use crate::BitReader;

/// Raw wire codes that mean "not available" or "this value or higher".
///
/// Crate-private: a decoded field carries its state as a [`FieldState`],
/// so a client never compares against these. The decoders and their
/// tests spell the codes by name.
pub(crate) mod sentinel {
    /// Longitude "not available": 181° in 1/10 000 minute (28-bit two's
    /// complement).
    pub const LON_NOT_AVAILABLE: i64 = 181 * 600_000;
    /// Latitude "not available": 91° in 1/10 000 minute (27-bit two's
    /// complement).
    pub const LAT_NOT_AVAILABLE: i64 = 91 * 600_000;
    /// Course over ground "not available", in 0.1°.
    pub const COG_NOT_AVAILABLE: u16 = 3600;
    /// True heading "not available", in whole degrees.
    pub const HEADING_NOT_AVAILABLE: u16 = 511;
    /// Vessel speed over ground "not available", in 0.1 kn (Types 1/2/3, 18,
    /// 19).
    pub const SOG_NOT_AVAILABLE: u16 = 1023;
    /// Vessel speed over ground "102.2 kn or higher", in 0.1 kn.
    pub const SOG_OVER_RANGE: u16 = 1022;
    /// Rate of turn "not available" (8-bit two's complement).
    pub const ROT_NOT_AVAILABLE: i8 = -128;
    /// Rate of turn "turning right at more than 5° per 30 s, no turn
    /// indicator".
    pub const ROT_NO_INDICATOR_RIGHT: i8 = 127;
    /// Rate of turn "turning left at more than 5° per 30 s, no turn indicator".
    pub const ROT_NO_INDICATOR_LEFT: i8 = -127;
    /// Navigation status "not defined".
    pub const NAV_STATUS_NOT_AVAILABLE: u8 = 15;
    /// Special manoeuvre indicator "not available".
    pub const MANEUVER_NOT_AVAILABLE: u8 = 0;
    /// EPFD type "undefined".
    pub const EPFD_NOT_AVAILABLE: u8 = 0;
    /// Ship and cargo type "not available".
    pub const SHIP_TYPE_NOT_AVAILABLE: u8 = 0;
    /// IMO number "not available".
    pub const IMO_NOT_AVAILABLE: u32 = 0;
    /// ETA month "not available".
    pub const ETA_MONTH_NOT_AVAILABLE: u8 = 0;
    /// ETA day "not available".
    pub const ETA_DAY_NOT_AVAILABLE: u8 = 0;
    /// ETA hour "not available".
    pub const ETA_HOUR_NOT_AVAILABLE: u8 = 24;
    /// ETA minute "not available".
    pub const ETA_MINUTE_NOT_AVAILABLE: u8 = 60;
    /// Draught "not available", in 0.1 m.
    pub const DRAUGHT_NOT_AVAILABLE: u8 = 0;
    /// Draught "25.5 m or greater", in 0.1 m.
    pub const DRAUGHT_OVER_RANGE: u8 = 255;
    /// SAR aircraft altitude "not available", in metres (Type 9).
    pub const ALTITUDE_NOT_AVAILABLE: u16 = 4095;
    /// SAR aircraft altitude "4094 m or higher".
    pub const ALTITUDE_OVER_RANGE: u16 = 4094;
    /// SAR aircraft speed over ground "not available", in whole knots (Type 9).
    pub const AIRCRAFT_SOG_NOT_AVAILABLE: u16 = 1023;
    /// SAR aircraft speed over ground "1022 kn or higher", in whole knots.
    pub const AIRCRAFT_SOG_OVER_RANGE: u16 = 1022;
    /// Dimension "not available" (every 9- and 6-bit field).
    pub const DIMENSION_NOT_AVAILABLE: u16 = 0;
    /// Dimension to bow / stern "511 m or greater" (9-bit fields).
    pub const DIMENSION_LONG_OVER_RANGE: u16 = 511;
    /// Dimension to port / starboard "63 m or greater" (6-bit fields).
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

/// Longitude in 1/10 000 minute: ±180° defined, +181° not available,
/// every other code invalid.
pub(crate) const LONGITUDE: Partition<f64> = Partition {
    not_available: sentinel::LON_NOT_AVAILABLE,
    over_range: None,
    defined: -180 * 600_000..=180 * 600_000,
    scale: degrees,
};

/// Latitude in 1/10 000 minute: ±90° defined, +91° not available, every
/// other code invalid.
pub(crate) const LATITUDE: Partition<f64> = Partition {
    not_available: sentinel::LAT_NOT_AVAILABLE,
    over_range: None,
    defined: -90 * 600_000..=90 * 600_000,
    scale: degrees,
};

/// Course over ground in 0.1°: 0..=3599 defined, 3600 not available,
/// 3601..=4095 invalid.
pub(crate) const COURSE_OVER_GROUND: Partition<f32> = Partition {
    not_available: sentinel::COG_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 0..=3599,
    scale: tenths,
};

/// True heading in whole degrees: 0..=359 defined, 511 not available,
/// 360..=510 invalid.
pub(crate) const TRUE_HEADING: Partition<u16> = Partition {
    not_available: sentinel::HEADING_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 0..=359,
    scale: whole_u16,
};

/// Vessel speed over ground in 0.1 kn (Types 1/2/3, 18, 19): 0..=1021
/// defined, 1022 at least 102.2 kn, 1023 not available.
pub(crate) const SPEED_OVER_GROUND: Partition<f32> = Partition {
    not_available: sentinel::SOG_NOT_AVAILABLE as i64,
    over_range: Some(sentinel::SOG_OVER_RANGE as i64),
    defined: 0..=1021,
    scale: tenths,
};

/// Ship and cargo type (Types 5, 19, 24 Part B): 0 not available, every
/// other code a value. The crate claims no knowledge of ITU-R M.1371-5
/// Table 53, so the regional and reserved ranges are values too.
pub(crate) const SHIP_TYPE: Partition<u8> = Partition {
    not_available: sentinel::SHIP_TYPE_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 1..=255,
    scale: whole_u8,
};

/// Dimension to bow or stern, 9 bits: 0 not available, 511 at least
/// 511 m.
const DIMENSION_LONG: Partition<u16> = Partition {
    not_available: sentinel::DIMENSION_NOT_AVAILABLE as i64,
    over_range: Some(sentinel::DIMENSION_LONG_OVER_RANGE as i64),
    defined: 1..=510,
    scale: whole_u16,
};

/// Dimension to port or starboard, 6 bits: 0 not available, 63 at
/// least 63 m.
const DIMENSION_SHORT: Partition<u8> = Partition {
    not_available: sentinel::DIMENSION_NOT_AVAILABLE as i64,
    over_range: Some(sentinel::DIMENSION_SHORT_OVER_RANGE as i64),
    defined: 1..=62,
    scale: whole_u8,
};

/// The 6-bit timestamp field of a position report: the second of the
/// UTC minute of the fix, or a status of the positioning system.
///
/// A status-carrying field: codes 61–63 are statuses and decode to a
/// value of this enum, so only code 60 is the not-available field
/// state. Exhaustive: the six-bit code space is fully assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timestamp {
    /// Codes 0..=59: the second within the UTC minute of the position
    /// fix.
    Second(u8),
    /// Codes 61..=63: the positioning system reports a status instead of
    /// a second.
    PositioningStatus(PositioningStatus),
}

/// The positioning-system statuses the timestamp field can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositioningStatus {
    /// 61 — positioning system in manual input mode.
    ManualInput,
    /// 62 — positioning system in estimated (dead reckoning) mode.
    DeadReckoning,
    /// 63 — positioning system inoperative.
    Inoperative,
}

impl PositioningStatus {
    /// The 6-bit timestamp code this status was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::ManualInput => sentinel::TIMESTAMP_MANUAL_INPUT,
            Self::DeadReckoning => sentinel::TIMESTAMP_DEAD_RECKONING,
            Self::Inoperative => sentinel::TIMESTAMP_INOPERATIVE,
        }
    }

    /// The variant for a status code; `None` for every other timestamp
    /// code.
    fn from_code(code: u8) -> Option<Self> {
        match code {
            sentinel::TIMESTAMP_MANUAL_INPUT => Some(Self::ManualInput),
            sentinel::TIMESTAMP_DEAD_RECKONING => Some(Self::DeadReckoning),
            sentinel::TIMESTAMP_INOPERATIVE => Some(Self::Inoperative),
            _ => None,
        }
    }
}

/// Extent of a station from its position-reference point, as emitted
/// in Type 5 (Class A static), Type 19 (Class B extended position
/// report), Type 21 (aid-to-navigation report) and Type 24 Part B
/// (Class B static part B).
///
/// The four wire fields measure the distances from the position-reference
/// point (typically the antenna) to the bow, stern, port, and
/// starboard edges of the vessel, in metres, and each carries its own
/// field state: `0` is not available and the maximum of each field
/// (511 m or 63 m) is the over-range bound `AtLeast(511)` / `AtLeast(63)`.
/// On a Type 21 they are the extent of the aid; virtual AtoN and
/// reference points send all zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    /// Distance from position-reference to bow, in metres (9 bits).
    pub to_bow_m: FieldState<u16>,
    /// Distance from position-reference to stern, in metres (9 bits).
    pub to_stern_m: FieldState<u16>,
    /// Distance from position-reference to port side, in metres (6 bits).
    pub to_port_m: FieldState<u8>,
    /// Distance from position-reference to starboard side, in metres
    /// (6 bits).
    pub to_starboard_m: FieldState<u8>,
}

/// Electronic Position-Fixing Device type — 4-bit field in Types 5, 19,
/// 21 and 24 Part B.
///
/// Code 0 (undefined) is the not-available field state and codes
/// 9..=14 are the invalid field state with the raw code; neither is a
/// variant. `#[non_exhaustive]` because a future edition may assign
/// one of the reserved codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpfdType {
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
}

impl EpfdType {
    /// The 4-bit wire code this variant was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Gps => 1,
            Self::Glonass => 2,
            Self::CombinedGpsGlonass => 3,
            Self::LoranC => 4,
            Self::Chayka => 5,
            Self::IntegratedNavigation => 6,
            Self::Surveyed => 7,
            Self::Galileo => 8,
            Self::InternalGnss => 15,
        }
    }

    /// The variant for a defined 4-bit code; `None` for the
    /// not-available code 0 and the reserved codes 9..=14.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Gps),
            2 => Some(Self::Glonass),
            3 => Some(Self::CombinedGpsGlonass),
            4 => Some(Self::LoranC),
            5 => Some(Self::Chayka),
            6 => Some(Self::IntegratedNavigation),
            7 => Some(Self::Surveyed),
            8 => Some(Self::Galileo),
            15 => Some(Self::InternalGnss),
            _ => None,
        }
    }
}

/// The code-space partition of one numeric wire field: which raw code
/// is not available, which (if any) is the over-range bound, which
/// range the specification defines, and how a defined code scales to
/// engineering units.
///
/// [`read`](Self::read) applies the partition in that order: the
/// not-available code, then the over-range code (whose bound is the
/// scaled code), then the defined range as a value, and every other
/// code inside the field's width as `Invalid(Undefined(RawCode))`. The
/// raw code is always the wire integer, never the scaled value.
pub(crate) struct Partition<T> {
    pub(crate) not_available: i64,
    pub(crate) over_range: Option<i64>,
    pub(crate) defined: RangeInclusive<i64>,
    pub(crate) scale: fn(i64) -> T,
}

impl<T> Partition<T> {
    /// Partition an unsigned wire read.
    pub(crate) fn read(&self, raw: u64) -> FieldState<T> {
        // No partitioned field is wider than 30 bits; the fallback is
        // unreachable and would read as invalid.
        self.read_signed(i64::try_from(raw).unwrap_or(i64::MAX))
    }

    /// Partition a two's-complement wire read.
    pub(crate) fn read_signed(&self, raw: i64) -> FieldState<T> {
        if raw == self.not_available {
            FieldState::NotAvailable
        } else if self.over_range == Some(raw) {
            FieldState::AtLeast((self.scale)(raw))
        } else if self.defined.contains(&raw) {
            FieldState::Value((self.scale)(raw))
        } else {
            FieldState::Invalid(Invalid::Undefined(RawCode(raw)))
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

/// Decode a code-space enum field: `not_available` is the one
/// not-available code, `from_code` names the defined codes, and every
/// other code is `Invalid(Undefined(RawCode(code)))`.
pub(crate) fn coded<E>(
    raw: u64,
    not_available: u8,
    from_code: fn(u8) -> Option<E>,
) -> FieldState<E> {
    // Every coded field is at most 6 bits wide; the fallback is unreachable.
    let code = u8::try_from(raw).unwrap_or(u8::MAX);
    if code == not_available {
        return FieldState::NotAvailable;
    }
    from_code(code).map_or(
        FieldState::Invalid(Invalid::Undefined(RawCode(code.into()))),
        FieldState::Value,
    )
}

/// Decode the 6-bit timestamp field: 60 is not available, 61..=63 are
/// positioning statuses, every other code is a second.
pub(crate) fn timestamp(raw: u64) -> FieldState<Timestamp> {
    coded(raw, sentinel::TIMESTAMP_NOT_AVAILABLE, |code| {
        Some(
            PositioningStatus::from_code(code)
                .map_or(Timestamp::Second(code), Timestamp::PositioningStatus),
        )
    })
}

/// Decode the 4-bit EPFD field at the reader's current position.
pub(crate) fn read_epfd(r: &mut BitReader<'_>) -> FieldState<EpfdType> {
    coded(r.u(4), sentinel::EPFD_NOT_AVAILABLE, EpfdType::from_code)
}

/// Read the 30-bit dimension block (A 9, B 9, C 6, D 6 bits) at the
/// reader's current position.
pub(crate) fn read_dimensions(r: &mut BitReader<'_>) -> Dimensions {
    Dimensions {
        to_bow_m: DIMENSION_LONG.read(r.u(9)),
        to_stern_m: DIMENSION_LONG.read(r.u(9)),
        to_port_m: DIMENSION_SHORT.read(r.u(6)),
        to_starboard_m: DIMENSION_SHORT.read(r.u(6)),
    }
}

/// Trim an AIS 6-bit string (decoded from [`crate::BitReader::string`])
/// to its field state: `NotAvailable` when the content is entirely
/// padding, else the trimmed text as a value.
///
/// AIS pads short strings with `@` (value 0) at the end and sometimes
/// with trailing spaces. A field consisting solely of these characters
/// means "not available".
pub(crate) fn trim_ais_string(s: String) -> FieldState<String> {
    let trimmed_len = s.trim_end_matches(['@', ' ']).len();
    if trimmed_len == 0 {
        FieldState::NotAvailable
    } else {
        let mut out = s;
        out.truncate(trimmed_len);
        FieldState::Value(out)
    }
}

/// Tenths of a unit to the unit.
#[allow(clippy::cast_precision_loss)] // a 12-bit raw is exact in f32
pub(crate) fn tenths(raw: i64) -> f32 {
    (raw as f32) / 10.0
}

/// A whole-unit code of at most 16 bits.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // partitioned to 0..=65_535
pub(crate) fn whole_u16(raw: i64) -> u16 {
    (raw & 0xFFFF) as u16
}

/// A whole-unit code of at most 8 bits.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // partitioned to 0..=255
pub(crate) fn whole_u8(raw: i64) -> u8 {
    (raw & 0xFF) as u8
}

/// 1/10 000 minute to decimal degrees.
#[allow(clippy::cast_precision_loss)] // a 28-bit raw is exact in f64
fn degrees(raw: i64) -> f64 {
    (raw as f64) / MINUTES_FRAC_PER_DEGREE
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
    use crate::testing::{dimensions_not_available, BitWriter};

    #[test]
    fn read_dimensions_zeros_are_not_available() {
        let mut w = BitWriter::new();
        w.u(30, 0);
        let (bits, total) = w.finish();
        let mut r = BitReader::new(&bits, total);
        assert_eq!(read_dimensions(&mut r), dimensions_not_available());
    }

    #[test]
    fn read_dimensions_maxima_are_over_range_bounds_and_neighbours_values() {
        let mut w = BitWriter::new();
        w.u(9, u64::from(sentinel::DIMENSION_LONG_OVER_RANGE));
        w.u(9, 510);
        w.u(6, u64::from(sentinel::DIMENSION_SHORT_OVER_RANGE));
        w.u(6, 62);
        let (bits, total) = w.finish();
        let mut r = BitReader::new(&bits, total);
        assert_eq!(
            read_dimensions(&mut r),
            Dimensions {
                to_bow_m: FieldState::AtLeast(511),
                to_stern_m: FieldState::Value(510),
                to_port_m: FieldState::AtLeast(63),
                to_starboard_m: FieldState::Value(62),
            }
        );
    }

    #[test]
    fn epfd_code_round_trips_every_defined_variant() {
        for code in [1u8, 2, 3, 4, 5, 6, 7, 8, 15] {
            assert_eq!(EpfdType::from_code(code).unwrap().code(), code);
        }
        assert_eq!(EpfdType::from_code(0), None);
        for code in 9u8..=14 {
            assert_eq!(EpfdType::from_code(code), None, "code {code}");
        }
    }

    #[test]
    fn positioning_status_code_names_the_three_timestamp_statuses() {
        assert_eq!(PositioningStatus::ManualInput.code(), 61);
        assert_eq!(PositioningStatus::DeadReckoning.code(), 62);
        assert_eq!(PositioningStatus::Inoperative.code(), 63);
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
        assert_eq!(
            trim_ais_string("NAME@@  @".into()),
            FieldState::Value(String::from("NAME"))
        );
        assert_eq!(trim_ais_string("@@@@".into()), FieldState::NotAvailable);
    }
}
