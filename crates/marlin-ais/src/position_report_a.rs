//! Class A position reports — AIS message types 1, 2, and 3.
//!
//! All three types share the same 168-bit field layout (ITU-R M.1371-5
//! §5.3.1). They differ only semantically:
//!
//! - **Type 1** — scheduled position report.
//! - **Type 2** — assigned scheduled position report.
//! - **Type 3** — special position report (response to interrogation).
//!
//! The top-level AIS dispatcher decodes the first 6 bits of the
//! payload to determine the type, then calls [`decode_position_report_a`]
//! for all three. The returned [`PositionReportA`] is identical across
//! types; the dispatcher wraps it in the appropriate enum variant
//! (`AisMessage::Type1`/`Type2`/`Type3` — added in a later round).
//!
//! # Sentinel values
//!
//! Per ITU-R M.1371-5 §5.3.1, several fields carry sentinel values
//! meaning "not available". The decoder maps sentinels to `None` on
//! `Option<T>` fields:
//!
//! | Field | Sentinel | Meaning |
//! | --- | --- | --- |
//! | Rate of turn | `-128` (0x80) | Not available |
//! | Speed over ground | `1023` | Not available |
//! | Longitude | `181°` (108 600 000 × 10⁻⁴ minutes) | Not available |
//! | Latitude | `91°` (54 600 000 × 10⁻⁴ minutes) | Not available |
//! | Course over ground | `3600` (× 10⁻¹ degrees) | Not available |
//! | True heading | `511` | Not available |
//!
//! Rate of turn `±127` is a sentinel that carries a status ("turning
//! right/left at more than 5° per 30 s, no turn indicator") rather than
//! "not available", so it decodes to [`RateOfTurn::NoIndicator`] instead
//! of `None` or a fabricated rate.
//!
//! The `timestamp` field keeps its raw `u8` because the values
//! 0..=59 are seconds, 60 means "not available", and 61..=63 carry
//! positioning-system-status information that some callers want to
//! inspect directly.

use crate::shared_types::{cog_deg, heading_deg, lat_deg, lon_deg, sentinel, sog_tenths_kn};
use crate::{AisError, BitReader};

/// Decoded Class A position report (AIS Type 1, 2, or 3).
///
/// All scalar quantities are in their natural human-readable units:
/// degrees, knots, seconds, etc. Raw AIS encoding has been normalized
/// and sentinels mapped to `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionReportA {
    /// Maritime Mobile Service Identity of the reporting vessel.
    pub mmsi: u32,
    /// Navigation status (one of the 15 defined values, plus
    /// `Reserved` for 9..=13 and future expansion).
    pub navigation_status: NavStatus,
    /// Rate of turn: a measured rate in degrees per minute, or the
    /// "turning faster than 5° per 30 s, no turn indicator" status
    /// for raw ±127. `None` when the sentinel `-128` is emitted.
    pub rate_of_turn: Option<RateOfTurn>,
    /// Speed over ground in knots. `None` when the sentinel `1023`
    /// is emitted.
    pub speed_over_ground: Option<f32>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes
    /// (typically ≤ 10 m), `false` for unaided GNSS (≤ 100 m).
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees (east positive). `None`
    /// when the sentinel `181°` is emitted.
    pub longitude_deg: Option<f64>,
    /// Latitude in signed decimal degrees (north positive). `None`
    /// when the sentinel `91°` is emitted.
    pub latitude_deg: Option<f64>,
    /// Course over ground in degrees (0..360). `None` when the
    /// sentinel `3600` is emitted.
    pub course_over_ground: Option<f32>,
    /// True heading in degrees (0..359). `None` when the sentinel
    /// `511` is emitted.
    pub true_heading: Option<u16>,
    /// Raw timestamp field: seconds within the UTC minute (0..=59),
    /// `60` means "not available", `61..=63` carry positioning-system
    /// status flags. Kept as `u8` so callers can distinguish all
    /// three semantic categories.
    pub timestamp: u8,
    /// Special maneuver indicator.
    pub special_maneuver: ManeuverIndicator,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Radio status field — synchronization and communication state,
    /// 19 bits. Layout depends on the underlying SOTDMA/ITDMA slot
    /// negotiation and is typically consumed by lower-layer tooling.
    pub radio_status: u32,
}

/// Decoded rate-of-turn field — 8 bits (ITU-R M.1371-5 Annex 8 Table 48).
///
/// Exhaustive: the wire codes are fully specified and cannot grow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateOfTurn {
    /// Measured rate in degrees per minute, starboard positive:
    /// `sign · (|raw| / 4.733)²` for raw `0..=±126`.
    DegPerMin(f32),
    /// Raw `±127`: turning right/left at more than 5° per 30 s with no
    /// turn indicator available. A status, not a rate — the square law
    /// would fabricate ±720 °/min here.
    NoIndicator(TurnDirection),
}

/// Which way a vessel is turning when only the direction is known
/// ([`RateOfTurn::NoIndicator`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDirection {
    /// Raw `+127` — turning right (to starboard).
    Right,
    /// Raw `−127` — turning left (to port).
    Left,
}

/// Navigation status field — 4 bits.
///
/// Values 9..=13 are reserved by the spec; we surface them as
/// [`Self::Reserved`] with the raw byte so callers can log unusual
/// values without this crate needing to bump its enum for each spec
/// revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NavStatus {
    /// 0 — under way using engine.
    UnderwayUsingEngine,
    /// 1 — at anchor.
    AtAnchor,
    /// 2 — not under command.
    NotUnderCommand,
    /// 3 — restricted maneuverability.
    RestrictedManeuverability,
    /// 4 — constrained by her draft.
    ConstrainedByDraft,
    /// 5 — moored.
    Moored,
    /// 6 — aground.
    Aground,
    /// 7 — engaged in fishing.
    EngagedInFishing,
    /// 8 — under way sailing.
    UnderwaySailing,
    /// 14 — AIS-SART (search-and-rescue transmitter) is active.
    AisSartActive,
    /// 15 — not defined (default).
    NotDefined,
    /// Reserved values (9..=13) — carried verbatim so callers can log
    /// or route without losing information.
    Reserved(u8),
}

impl NavStatus {
    fn from_u4(v: u8) -> Self {
        match v {
            0 => Self::UnderwayUsingEngine,
            1 => Self::AtAnchor,
            2 => Self::NotUnderCommand,
            3 => Self::RestrictedManeuverability,
            4 => Self::ConstrainedByDraft,
            5 => Self::Moored,
            6 => Self::Aground,
            7 => Self::EngagedInFishing,
            8 => Self::UnderwaySailing,
            14 => Self::AisSartActive,
            15 => Self::NotDefined,
            other => Self::Reserved(other),
        }
    }

    /// The 4-bit wire code this variant was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::UnderwayUsingEngine => 0,
            Self::AtAnchor => 1,
            Self::NotUnderCommand => 2,
            Self::RestrictedManeuverability => 3,
            Self::ConstrainedByDraft => 4,
            Self::Moored => 5,
            Self::Aground => 6,
            Self::EngagedInFishing => 7,
            Self::UnderwaySailing => 8,
            Self::AisSartActive => 14,
            Self::NotDefined => 15,
            Self::Reserved(c) => c,
        }
    }
}

/// Special maneuver indicator — 2 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManeuverIndicator {
    /// 0 — not available (default).
    NotAvailable,
    /// 1 — no special maneuver.
    NoSpecial,
    /// 2 — special maneuver (e.g. regional passing arrangement).
    Special,
    /// 3 — reserved.
    Reserved,
}

impl ManeuverIndicator {
    fn from_u2(v: u8) -> Self {
        match v {
            1 => Self::NoSpecial,
            2 => Self::Special,
            3 => Self::Reserved,
            _ => Self::NotAvailable,
        }
    }

    /// The 2-bit wire code this variant was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::NotAvailable => 0,
            Self::NoSpecial => 1,
            Self::Special => 2,
            Self::Reserved => 3,
        }
    }
}

/// Minimum valid payload size for Types 1/2/3 (ITU-R M.1371-5 §5.3.1).
pub const POSITION_REPORT_A_BITS: usize = 168;

/// Decode a Class A position report (Type 1, 2, or 3) from a
/// bit-packed payload.
///
/// The caller is responsible for having already dispatched on the
/// first 6 bits (`msg_type`) to choose this function; the decoder
/// consumes the `msg_type` field but does not verify it. The three
/// message types share an identical layout, so a single decoder
/// handles all three.
///
/// # Errors
///
/// Returns [`AisError::PayloadTooShort`] if `total_bits < 168`. Bit
/// reads are otherwise saturating (past-end yields zero), so a
/// partial payload produces a [`PositionReportA`] with some fields
/// defaulted — use the bit-count check to reject short messages
/// before calling if you want strict validation.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_position_report_a(
    bits: &[u8],
    total_bits: usize,
) -> Result<PositionReportA, AisError> {
    if total_bits < POSITION_REPORT_A_BITS {
        return Err(AisError::PayloadTooShort);
    }

    let mut r = BitReader::new(bits, total_bits);
    // msg_type (6 bits) — dispatched on externally; we consume and ignore.
    let _ = r.u(6);
    // repeat indicator (2 bits) — not exposed on the typed struct.
    let _ = r.u(2);

    // u(30) fits easily in u32.
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let nav_status = NavStatus::from_u4((r.u(4) & 0x0F) as u8);

    // Rate of turn: 8-bit two's-complement "ROT_AIS" indicator.
    let rot_raw = r.i(8);
    let rate_of_turn = decode_rate_of_turn(rot_raw);

    let speed_over_ground = sog_tenths_kn(r.u(10));
    let position_accuracy = r.b();
    let longitude_deg = lon_deg(r.i(28));
    let latitude_deg = lat_deg(r.i(27));
    let course_over_ground = cog_deg(r.u(12));
    let true_heading = heading_deg(r.u(9));

    let timestamp = (r.u(6) & 0x3F) as u8;
    let special_maneuver = ManeuverIndicator::from_u2((r.u(2) & 0x03) as u8);
    let _ = r.u(3); // spare
    let raim = r.b();
    // radio_status: 19 bits.
    let radio_status = (r.u(19) & 0x7_FFFF) as u32;

    Ok(PositionReportA {
        mmsi,
        navigation_status: nav_status,
        rate_of_turn,
        speed_over_ground,
        position_accuracy,
        longitude_deg,
        latitude_deg,
        course_over_ground,
        true_heading,
        timestamp,
        special_maneuver,
        raim,
        radio_status,
    })
}

/// Decode the 8-bit `ROT_AIS` indicator (Table 48).
///
/// `0..=±126` is sign-preserved with a square-law expansion,
/// `R = (X / 4.733)² × sign(X)`. `±127` is the no-turn-indicator
/// status and carries only a direction. The sentinel `-128` maps to
/// `None`.
#[allow(clippy::cast_possible_truncation)]
fn decode_rate_of_turn(raw: i64) -> Option<RateOfTurn> {
    // r.i(8) always produces a value in the i8 range; narrow it.
    let raw = raw as i8;
    match raw {
        sentinel::ROT_NOT_AVAILABLE => None,
        sentinel::ROT_NO_INDICATOR_RIGHT => Some(RateOfTurn::NoIndicator(TurnDirection::Right)),
        sentinel::ROT_NO_INDICATOR_LEFT => Some(RateOfTurn::NoIndicator(TurnDirection::Left)),
        measured => {
            let sign = if measured < 0 { -1.0_f32 } else { 1.0 };
            let magnitude = f32::from(measured).abs() / 4.733;
            Some(RateOfTurn::DegPerMin(sign * magnitude * magnitude))
        }
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
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use crate::shared_types::MINUTES_FRAC_PER_DEGREE;
    use crate::testing::{armor_encode, BitWriter};

    /// Build a 168-bit position-report payload with every field
    /// caller-specified. Used across tests to verify individual
    /// field decoding in isolation.
    #[allow(clippy::too_many_arguments)]
    fn build_pra(
        msg_type: u8,
        repeat: u8,
        mmsi: u32,
        nav: u8,
        rot: i8,
        sog: u16,
        pos_acc: bool,
        lon_raw: i32, // 28-bit signed
        lat_raw: i32, // 27-bit signed
        cog: u16,
        heading: u16,
        timestamp: u8,
        maneuver: u8,
        raim: bool,
        radio: u32,
    ) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, u64::from(msg_type));
        w.u(2, u64::from(repeat));
        w.u(30, u64::from(mmsi));
        w.u(4, u64::from(nav));
        w.i(8, i64::from(rot));
        w.u(10, u64::from(sog));
        w.b(pos_acc);
        w.i(28, i64::from(lon_raw));
        w.i(27, i64::from(lat_raw));
        w.u(12, u64::from(cog));
        w.u(9, u64::from(heading));
        w.u(6, u64::from(timestamp));
        w.u(2, u64::from(maneuver));
        w.u(3, 0); // spare
        w.b(raim);
        w.u(19, u64::from(radio));
        w.finish()
    }

    /// Type 1 payload with the given MMSI and raw rate of turn; every
    /// other field zero. The rate-of-turn tests vary only these two.
    fn build_pra_rot(mmsi: u32, rot: i8) -> (alloc::vec::Vec<u8>, usize) {
        build_pra(1, 0, mmsi, 0, rot, 0, false, 0, 0, 0, 0, 0, 0, false, 0)
    }

    /// Unwrap a measured rate; panics on a status or `None`.
    fn deg_per_min(rot: Option<RateOfTurn>) -> f32 {
        match rot {
            Some(RateOfTurn::DegPerMin(v)) => v,
            other => panic!("expected DegPerMin, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Happy path: classic ITU-R Annex 5 fixture (MMSI 244 708 736)
    // -----------------------------------------------------------------

    #[test]
    fn decodes_classic_annex5_position_report() {
        let (bits, total) = crate::armor::decode(b"13aGmP0P00PD;88MD5MTDww@2<0L", 0).unwrap();
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.mmsi, 244_708_736);
        // Further fields depend on fixture; the MMSI match is the
        // strong signal that the bit alignment is correct.
    }

    // -----------------------------------------------------------------
    // All sentinels → None
    // -----------------------------------------------------------------

    #[test]
    fn all_sentinels_decode_to_none() {
        let (bits, total) = build_pra(
            1,
            0,
            123_456_789,
            15, // NotDefined
            sentinel::ROT_NOT_AVAILABLE,
            sentinel::SOG_NOT_AVAILABLE,
            false,
            sentinel::LON_NOT_AVAILABLE as i32,
            sentinel::LAT_NOT_AVAILABLE as i32,
            sentinel::COG_NOT_AVAILABLE,
            sentinel::HEADING_NOT_AVAILABLE,
            60, // timestamp N/A
            0,  // maneuver NotAvailable
            false,
            0,
        );
        let pra = decode_position_report_a(&bits, total).unwrap();

        assert_eq!(pra.mmsi, 123_456_789);
        assert_eq!(pra.navigation_status, NavStatus::NotDefined);
        assert_eq!(pra.rate_of_turn, None);
        assert_eq!(pra.speed_over_ground, None);
        assert!(!pra.position_accuracy);
        assert_eq!(pra.longitude_deg, None);
        assert_eq!(pra.latitude_deg, None);
        assert_eq!(pra.course_over_ground, None);
        assert_eq!(pra.true_heading, None);
        assert_eq!(pra.timestamp, 60);
        assert_eq!(pra.special_maneuver, ManeuverIndicator::NotAvailable);
        assert!(!pra.raim);
        assert_eq!(pra.radio_status, 0);
    }

    // -----------------------------------------------------------------
    // Positive (northern + eastern) and negative (southern + western)
    // coordinates — PRD §A4 sign-handling requirement
    // -----------------------------------------------------------------

    #[test]
    fn northern_eastern_coordinates_are_positive() {
        // 48.1173° N, 11.5167° E (classic Munich demo position).
        let lat_raw: i32 = (48.1173_f64 * MINUTES_FRAC_PER_DEGREE) as i32;
        let lon_raw: i32 = (11.5167_f64 * MINUTES_FRAC_PER_DEGREE) as i32;
        let (bits, total) = build_pra(
            1, 0, 1, 0, 0, 0, false, lon_raw, lat_raw, 0, 0, 0, 0, false, 0,
        );
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert!((pra.latitude_deg.unwrap() - 48.1173).abs() < 1e-4);
        assert!((pra.longitude_deg.unwrap() - 11.5167).abs() < 1e-4);
    }

    #[test]
    fn southern_western_coordinates_are_negative() {
        // 48.1173° S, 11.5167° W.
        let lat_raw: i32 = -(48.1173_f64 * MINUTES_FRAC_PER_DEGREE) as i32;
        let lon_raw: i32 = -(11.5167_f64 * MINUTES_FRAC_PER_DEGREE) as i32;
        let (bits, total) = build_pra(
            1, 0, 1, 0, 0, 0, false, lon_raw, lat_raw, 0, 0, 0, 0, false, 0,
        );
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert!(pra.latitude_deg.unwrap() < 0.0);
        assert!(pra.longitude_deg.unwrap() < 0.0);
        assert!((pra.latitude_deg.unwrap() + 48.1173).abs() < 1e-4);
        assert!((pra.longitude_deg.unwrap() + 11.5167).abs() < 1e-4);
    }

    #[test]
    fn equator_and_prime_meridian_decode_to_zero() {
        let (bits, total) = build_pra(1, 0, 1, 0, 0, 0, false, 0, 0, 0, 0, 0, 0, false, 0);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.latitude_deg, Some(0.0));
        assert_eq!(pra.longitude_deg, Some(0.0));
    }

    // -----------------------------------------------------------------
    // Navigation status variants — at least one test per branch
    // -----------------------------------------------------------------

    #[test]
    fn nav_status_maps_well_defined_codes() {
        for (code, expected) in [
            (0u8, NavStatus::UnderwayUsingEngine),
            (1, NavStatus::AtAnchor),
            (2, NavStatus::NotUnderCommand),
            (3, NavStatus::RestrictedManeuverability),
            (4, NavStatus::ConstrainedByDraft),
            (5, NavStatus::Moored),
            (6, NavStatus::Aground),
            (7, NavStatus::EngagedInFishing),
            (8, NavStatus::UnderwaySailing),
            (14, NavStatus::AisSartActive),
            (15, NavStatus::NotDefined),
        ] {
            assert_eq!(NavStatus::from_u4(code), expected);
        }
    }

    #[test]
    fn nav_status_reserved_codes_preserve_raw_value() {
        for code in 9u8..=13 {
            assert_eq!(NavStatus::from_u4(code), NavStatus::Reserved(code));
        }
    }

    #[test]
    fn nav_status_code_round_trips_every_value() {
        for code in 0u8..16 {
            assert_eq!(NavStatus::from_u4(code).code(), code);
        }
    }

    // -----------------------------------------------------------------
    // Maneuver indicator
    // -----------------------------------------------------------------

    #[test]
    fn maneuver_indicator_covers_every_value() {
        assert_eq!(
            ManeuverIndicator::from_u2(0),
            ManeuverIndicator::NotAvailable
        );
        assert_eq!(ManeuverIndicator::from_u2(1), ManeuverIndicator::NoSpecial);
        assert_eq!(ManeuverIndicator::from_u2(2), ManeuverIndicator::Special);
        assert_eq!(ManeuverIndicator::from_u2(3), ManeuverIndicator::Reserved);
    }

    #[test]
    fn maneuver_indicator_code_round_trips_every_value() {
        for code in 0u8..4 {
            assert_eq!(ManeuverIndicator::from_u2(code).code(), code);
        }
    }

    // -----------------------------------------------------------------
    // Rate of turn — PRD §A2 (signed decoding is the single most
    // error-prone area)
    // -----------------------------------------------------------------

    #[test]
    fn rate_of_turn_sentinel_maps_to_none() {
        let (bits, total) = build_pra_rot(1, sentinel::ROT_NOT_AVAILABLE);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.rate_of_turn, None);
    }

    #[test]
    fn rate_of_turn_zero_means_not_turning() {
        let (bits, total) = build_pra_rot(1, 0);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.rate_of_turn, Some(RateOfTurn::DegPerMin(0.0)));
    }

    #[test]
    fn rate_of_turn_preserves_sign() {
        // Known encoding: R = 20°/min gives X ≈ 21 (floor of 4.733·sqrt(20)).
        // Decode X = 21 → (21 / 4.733)² ≈ 19.7.
        let (bits, total) = build_pra_rot(1, 21);
        let pos = decode_position_report_a(&bits, total).unwrap();
        let (bits, total) = build_pra_rot(1, -21);
        let neg = decode_position_report_a(&bits, total).unwrap();
        let p = deg_per_min(pos.rate_of_turn);
        let n = deg_per_min(neg.rate_of_turn);
        assert!((p - 19.69).abs() < 1e-2);
        assert!((p + n).abs() < 1e-4); // magnitude matches, signs cancel
    }

    #[test]
    fn rate_of_turn_126_is_the_largest_measured_rate() {
        // ±126 is the last code the square law applies to: (126 / 4.733)² ≈ 708.7.
        let (bits, total) = build_pra_rot(1, 126);
        let pos = decode_position_report_a(&bits, total).unwrap();
        let (bits, total) = build_pra_rot(1, -126);
        let neg = decode_position_report_a(&bits, total).unwrap();
        assert!((deg_per_min(pos.rate_of_turn) - 708.7).abs() < 1e-1);
        assert!((deg_per_min(neg.rate_of_turn) + 708.7).abs() < 1e-1);
    }

    #[test]
    fn rate_of_turn_127_is_no_indicator_right() {
        let (bits, total) = build_pra_rot(1, sentinel::ROT_NO_INDICATOR_RIGHT);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(
            pra.rate_of_turn,
            Some(RateOfTurn::NoIndicator(TurnDirection::Right))
        );
    }

    #[test]
    fn rate_of_turn_minus_127_is_no_indicator_left() {
        let (bits, total) = build_pra_rot(1, sentinel::ROT_NO_INDICATOR_LEFT);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(
            pra.rate_of_turn,
            Some(RateOfTurn::NoIndicator(TurnDirection::Left))
        );
    }

    /// Pins the armored form of the ±127 payloads so the Python unit
    /// test `test_position_report_a_turn_direction` (bindings) can
    /// decode the same bits without a Python-side bit packer. 168 bits
    /// armor to exactly 28 characters with zero fill bits.
    #[test]
    fn rate_of_turn_no_indicator_payloads_armor_to_known_strings() {
        let (bits, total) = build_pra_rot(123_456_789, sentinel::ROT_NO_INDICATOR_RIGHT);
        let armored = b"11mg=5@Oh0000000000000000000";
        assert_eq!(armor_encode(&bits, total), (armored.to_vec(), 0));
        assert_eq!(crate::armor::decode(armored, 0).unwrap(), (bits, total));

        let (bits, total) = build_pra_rot(123_456_789, sentinel::ROT_NO_INDICATOR_LEFT);
        let armored = b"11mg=5@P@0000000000000000000";
        assert_eq!(armor_encode(&bits, total), (armored.to_vec(), 0));
        assert_eq!(crate::armor::decode(armored, 0).unwrap(), (bits, total));
    }

    // -----------------------------------------------------------------
    // Speed / course / heading sentinels
    // -----------------------------------------------------------------

    #[test]
    fn sog_and_cog_decode_with_tenth_scaling() {
        // SOG = 255 → 25.5 knots; COG = 1234 → 123.4°.
        let (bits, total) = build_pra(1, 0, 1, 0, 0, 255, false, 0, 0, 1234, 42, 0, 0, false, 0);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert!((pra.speed_over_ground.unwrap() - 25.5).abs() < 1e-4);
        assert!((pra.course_over_ground.unwrap() - 123.4).abs() < 1e-4);
        assert_eq!(pra.true_heading, Some(42));
    }

    // -----------------------------------------------------------------
    // Error: payload shorter than 168 bits
    // -----------------------------------------------------------------

    #[test]
    fn too_short_payload_is_rejected() {
        let buf = [0u8; 10];
        match decode_position_report_a(&buf, 80) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Works for Type 2 and Type 3 as well (layout is identical)
    // -----------------------------------------------------------------

    #[test]
    fn decoder_accepts_type_2_and_type_3_headers() {
        for msg_type in [1u8, 2, 3] {
            let (bits, total) = build_pra(
                msg_type,
                0,
                987_654_321,
                1,
                0,
                100,
                true,
                0,
                0,
                0,
                180,
                0,
                0,
                false,
                0,
            );
            let pra = decode_position_report_a(&bits, total).unwrap();
            assert_eq!(pra.mmsi, 987_654_321);
            assert_eq!(pra.speed_over_ground, Some(10.0));
            assert_eq!(pra.true_heading, Some(180));
        }
    }

    // -----------------------------------------------------------------
    // RAIM flag round-trips
    // -----------------------------------------------------------------

    #[test]
    fn raim_flag_round_trips() {
        for raim_in in [false, true] {
            let (bits, total) = build_pra(1, 0, 1, 0, 0, 0, false, 0, 0, 0, 0, 0, 0, raim_in, 0);
            let pra = decode_position_report_a(&bits, total).unwrap();
            assert_eq!(pra.raim, raim_in);
        }
    }
}
