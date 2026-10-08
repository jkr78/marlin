//! Class A position reports — AIS message types 1, 2, and 3.
//!
//! All three types share the same 168-bit field layout (ITU-R M.1371-5
//! Annex 8 §3.1, Table 48). They differ only semantically:
//!
//! - **Type 1** — scheduled position report.
//! - **Type 2** — assigned scheduled position report.
//! - **Type 3** — special position report (response to interrogation).
//!
//! The top-level AIS dispatcher decodes the first 6 bits of the
//! payload to determine the type, then calls [`decode_position_report_a`]
//! for all three. The returned [`PositionReportA`] is identical across
//! types; the dispatcher wraps it in the appropriate enum variant
//! ([`AisMessageBody::Type1`](crate::AisMessageBody::Type1), `Type2` or
//! `Type3`).
//!
//! # Field states
//!
//! Per ITU-R M.1371-5 Annex 8 §3.1, Table 48, several fields carry codes
//! that mean "not available" or lie outside the defined range. Each such
//! field is a [`FieldState`]:
//!
//! | Field | Not available | Over-range | Invalid (raw code kept) |
//! | --- | --- | --- | --- |
//! | Navigation status | `15` | | `9..=13` |
//! | Rate of turn | `-128` | | |
//! | Speed over ground | `1023` | `1022` → `AtLeast(102.2)` | |
//! | Longitude | `181°` | | any other code beyond ±180° |
//! | Latitude | `91°` | | any other code beyond ±90° |
//! | Course over ground | `3600` | | `3601..=4095` |
//! | True heading | `511` | | `360..=510` |
//! | Timestamp | `60` | | |
//! | Special manoeuvre | `0` | | `3` |
//!
//! Rate of turn `±127` and timestamp `61..=63` carry a status rather than
//! "not available", so they decode to a value of the field's own enum
//! ([`RateOfTurn::NoIndicator`], [`Timestamp::PositioningStatus`]) rather
//! than to a field state or a fabricated number.

use marlin_field::FieldState;

use crate::shared_types::{
    coded, sentinel, timestamp, COURSE_OVER_GROUND, LATITUDE, LONGITUDE, SPEED_OVER_GROUND,
    TRUE_HEADING,
};
use crate::{AisError, BitReader, Timestamp};

/// Decoded Class A position report (AIS Type 1, 2, or 3).
///
/// All scalar quantities are in their natural human-readable units:
/// degrees, knots, seconds, etc. Raw AIS encoding has been normalized
/// and every field the wire can leave without a value is a
/// [`FieldState`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionReportA {
    /// Maritime Mobile Service Identity of the reporting vessel.
    pub mmsi: u32,
    /// Navigation status. Code 15 (not defined) is not available; the
    /// reserved codes 9..=13 are invalid with the raw code.
    pub navigation_status: FieldState<NavStatus>,
    /// Rate of turn: a measured rate in degrees per minute, or the
    /// "turning faster than 5° per 30 s, no turn indicator" status
    /// for raw ±127. Not available on the code `-128`.
    pub rate_of_turn: FieldState<RateOfTurn>,
    /// Speed over ground in knots. Not available on `1023`;
    /// `AtLeast(102.2)` on the over-range code `1022`.
    pub speed_over_ground: FieldState<f32>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes
    /// (typically ≤ 10 m), `false` for unaided GNSS (≤ 100 m).
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees (east positive). Not
    /// available on `181°`; invalid with the raw code (in 1/10 000
    /// minute) on any other code beyond ±180°.
    pub longitude_deg: FieldState<f64>,
    /// Latitude in signed decimal degrees (north positive). Not
    /// available on `91°`; invalid with the raw code on any other code
    /// beyond ±90°.
    pub latitude_deg: FieldState<f64>,
    /// Course over ground in degrees (0..360). Not available on `3600`;
    /// invalid on `3601..=4095`.
    pub course_over_ground: FieldState<f32>,
    /// True heading in degrees (0..=359). Not available on `511`;
    /// invalid on `360..=510`.
    pub true_heading: FieldState<u16>,
    /// Second of the UTC minute of the position fix, or a
    /// positioning-system status (codes 61..=63). Not available on `60`.
    pub timestamp: FieldState<Timestamp>,
    /// Special manoeuvre indicator. Code 0 is not available; the
    /// reserved code 3 is invalid.
    pub special_maneuver: FieldState<ManeuverIndicator>,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Radio status field — synchronization and communication state,
    /// 19 bits. Layout depends on the underlying SOTDMA/ITDMA slot
    /// negotiation and is typically consumed by lower-layer tooling.
    pub radio_status: u32,
}

/// Decoded rate-of-turn field — 8 bits (ITU-R M.1371-5 Annex 8 Table 48).
///
/// A status-carrying field: `±127` is a status and a value of this
/// enum, not a field state. Exhaustive: the wire codes are fully
/// specified and cannot grow.
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
/// Code 15 (not defined) is the not-available field state and the
/// reserved codes 9..=13 are the invalid field state with the raw
/// code; neither is a variant. `#[non_exhaustive]` because a future
/// edition may assign one of the reserved codes.
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
}

impl NavStatus {
    /// The variant for a defined 4-bit code; `None` for the
    /// not-available code 15 and the reserved codes 9..=13.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::UnderwayUsingEngine),
            1 => Some(Self::AtAnchor),
            2 => Some(Self::NotUnderCommand),
            3 => Some(Self::RestrictedManeuverability),
            4 => Some(Self::ConstrainedByDraft),
            5 => Some(Self::Moored),
            6 => Some(Self::Aground),
            7 => Some(Self::EngagedInFishing),
            8 => Some(Self::UnderwaySailing),
            14 => Some(Self::AisSartActive),
            _ => None,
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
        }
    }
}

/// Special manoeuvre indicator — 2 bits.
///
/// Code 0 is the not-available field state and the reserved code 3 is
/// the invalid field state; neither is a variant. `#[non_exhaustive]`
/// because a future edition may assign code 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ManeuverIndicator {
    /// 1 — no special manoeuvre.
    NoSpecial,
    /// 2 — special manoeuvre (e.g. regional passing arrangement).
    Special,
}

impl ManeuverIndicator {
    /// The variant for a defined 2-bit code; `None` for the
    /// not-available code 0 and the reserved code 3.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::NoSpecial),
            2 => Some(Self::Special),
            _ => None,
        }
    }

    /// The 2-bit wire code this variant was decoded from.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::NoSpecial => 1,
            Self::Special => 2,
        }
    }
}

/// Minimum valid payload size for Types 1/2/3 (ITU-R M.1371-5 Annex 8
/// §3.1, Table 48).
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
/// Returns [`AisError::PayloadTooShort`] if `total_bits < 168`. Bits
/// past 168 are ignored.
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
    let navigation_status = coded(
        r.u(4),
        sentinel::NAV_STATUS_NOT_AVAILABLE,
        NavStatus::from_code,
    );
    let rate_of_turn = decode_rate_of_turn(r.i(8));
    let speed_over_ground = SPEED_OVER_GROUND.read(r.u(10));
    let position_accuracy = r.b();
    let longitude_deg = LONGITUDE.read_signed(r.i(28));
    let latitude_deg = LATITUDE.read_signed(r.i(27));
    let course_over_ground = COURSE_OVER_GROUND.read(r.u(12));
    let true_heading = TRUE_HEADING.read(r.u(9));
    let timestamp = timestamp(r.u(6));
    let special_maneuver = coded(
        r.u(2),
        sentinel::MANEUVER_NOT_AVAILABLE,
        ManeuverIndicator::from_code,
    );
    let _ = r.u(3); // spare
    let raim = r.b();
    // radio_status: 19 bits.
    let radio_status = (r.u(19) & 0x7_FFFF) as u32;

    Ok(PositionReportA {
        mmsi,
        navigation_status,
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
/// status and carries only a direction. The code `-128` is not
/// available.
#[allow(clippy::cast_possible_truncation)]
fn decode_rate_of_turn(raw: i64) -> FieldState<RateOfTurn> {
    // r.i(8) always produces a value in the i8 range; narrow it.
    let raw = raw as i8;
    match raw {
        sentinel::ROT_NOT_AVAILABLE => FieldState::NotAvailable,
        sentinel::ROT_NO_INDICATOR_RIGHT => {
            FieldState::Value(RateOfTurn::NoIndicator(TurnDirection::Right))
        }
        sentinel::ROT_NO_INDICATOR_LEFT => {
            FieldState::Value(RateOfTurn::NoIndicator(TurnDirection::Left))
        }
        measured => {
            let sign = if measured < 0 { -1.0_f32 } else { 1.0 };
            let magnitude = f32::from(measured).abs() / 4.733;
            FieldState::Value(RateOfTurn::DegPerMin(sign * magnitude * magnitude))
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
    use crate::testing::{armor_encode, undefined, BitWriter};
    use crate::PositioningStatus;

    /// Every Table 48 field a test may want to vary. The spare bits are
    /// zero.
    struct Fields {
        msg_type: u8,
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
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                msg_type: 1,
                mmsi: 1,
                nav: 0,
                rot: 0,
                sog: 0,
                pos_acc: false,
                lon_raw: 0,
                lat_raw: 0,
                cog: 0,
                heading: 0,
                timestamp: 0,
                maneuver: 0,
                raim: false,
                radio: 0,
            }
        }
    }

    /// Pack `f` into a 168-bit Type 1/2/3 payload (Table 48).
    fn build_pra(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, u64::from(f.msg_type));
        w.u(2, 0); // repeat
        w.u(30, u64::from(f.mmsi));
        w.u(4, u64::from(f.nav));
        w.i(8, i64::from(f.rot));
        w.u(10, u64::from(f.sog));
        w.b(f.pos_acc);
        w.i(28, i64::from(f.lon_raw));
        w.i(27, i64::from(f.lat_raw));
        w.u(12, u64::from(f.cog));
        w.u(9, u64::from(f.heading));
        w.u(6, u64::from(f.timestamp));
        w.u(2, u64::from(f.maneuver));
        w.u(3, 0); // spare
        w.b(f.raim);
        w.u(19, u64::from(f.radio));
        w.finish()
    }

    fn decode(f: &Fields) -> PositionReportA {
        let (bits, total) = build_pra(f);
        decode_position_report_a(&bits, total).unwrap()
    }

    /// Type 1 payload with the given MMSI and raw rate of turn; every
    /// other field zero. The rate-of-turn tests vary only these two.
    fn build_pra_rot(mmsi: u32, rot: i8) -> (alloc::vec::Vec<u8>, usize) {
        build_pra(&Fields {
            mmsi,
            rot,
            ..Fields::default()
        })
    }

    /// Unwrap a measured rate; panics on a status or a non-value state.
    fn deg_per_min(rot: FieldState<RateOfTurn>) -> f32 {
        match rot {
            FieldState::Value(RateOfTurn::DegPerMin(v)) => v,
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
        assert_eq!(
            pra.navigation_status,
            FieldState::Value(NavStatus::UnderwayUsingEngine)
        );
        assert_eq!(pra.speed_over_ground, FieldState::Value(0.0));
        assert_eq!(pra.true_heading, FieldState::NotAvailable);
        assert_eq!(pra.timestamp, FieldState::Value(Timestamp::Second(40)));
    }

    // -----------------------------------------------------------------
    // Every not-available code at once
    // -----------------------------------------------------------------

    #[test]
    fn every_not_available_code_decodes_to_not_available() {
        let pra = decode(&Fields {
            mmsi: 123_456_789,
            nav: sentinel::NAV_STATUS_NOT_AVAILABLE,
            rot: sentinel::ROT_NOT_AVAILABLE,
            sog: sentinel::SOG_NOT_AVAILABLE,
            lon_raw: sentinel::LON_NOT_AVAILABLE as i32,
            lat_raw: sentinel::LAT_NOT_AVAILABLE as i32,
            cog: sentinel::COG_NOT_AVAILABLE,
            heading: sentinel::HEADING_NOT_AVAILABLE,
            timestamp: sentinel::TIMESTAMP_NOT_AVAILABLE,
            maneuver: sentinel::MANEUVER_NOT_AVAILABLE,
            ..Fields::default()
        });

        assert_eq!(pra.mmsi, 123_456_789);
        assert_eq!(pra.navigation_status, FieldState::NotAvailable);
        assert_eq!(pra.rate_of_turn, FieldState::NotAvailable);
        assert_eq!(pra.speed_over_ground, FieldState::NotAvailable);
        assert!(!pra.position_accuracy);
        assert_eq!(pra.longitude_deg, FieldState::NotAvailable);
        assert_eq!(pra.latitude_deg, FieldState::NotAvailable);
        assert_eq!(pra.course_over_ground, FieldState::NotAvailable);
        assert_eq!(pra.true_heading, FieldState::NotAvailable);
        assert_eq!(pra.timestamp, FieldState::NotAvailable);
        assert_eq!(pra.special_maneuver, FieldState::NotAvailable);
        assert!(!pra.raim);
        assert_eq!(pra.radio_status, 0);
    }

    // -----------------------------------------------------------------
    // Longitude and latitude: sign, boundaries, undefined codes
    // -----------------------------------------------------------------

    #[test]
    fn northern_eastern_coordinates_are_positive() {
        // 48.1173° N, 11.5167° E (classic Munich demo position).
        let pra = decode(&Fields {
            lat_raw: (48.1173_f64 * MINUTES_FRAC_PER_DEGREE) as i32,
            lon_raw: (11.5167_f64 * MINUTES_FRAC_PER_DEGREE) as i32,
            ..Fields::default()
        });
        assert!((pra.latitude_deg.value().unwrap() - 48.1173).abs() < 1e-4);
        assert!((pra.longitude_deg.value().unwrap() - 11.5167).abs() < 1e-4);
    }

    #[test]
    fn southern_western_coordinates_are_negative() {
        // 48.1173° S, 11.5167° W.
        let pra = decode(&Fields {
            lat_raw: -(48.1173_f64 * MINUTES_FRAC_PER_DEGREE) as i32,
            lon_raw: -(11.5167_f64 * MINUTES_FRAC_PER_DEGREE) as i32,
            ..Fields::default()
        });
        assert!((pra.latitude_deg.value().unwrap() + 48.1173).abs() < 1e-4);
        assert!((pra.longitude_deg.value().unwrap() + 11.5167).abs() < 1e-4);
    }

    #[test]
    fn equator_and_prime_meridian_decode_to_zero() {
        let pra = decode(&Fields::default());
        assert_eq!(pra.latitude_deg, FieldState::Value(0.0));
        assert_eq!(pra.longitude_deg, FieldState::Value(0.0));
    }

    #[test]
    fn longitude_rows() {
        // ±180° exactly are the last defined codes; one code beyond each
        // is invalid with the raw code; +181° alone is not available.
        let east = 180 * 600_000;
        for (raw, expected) in [
            (east, FieldState::Value(180.0)),
            (-east, FieldState::Value(-180.0)),
            (
                east + 1,
                FieldState::Invalid(undefined(i64::from(east) + 1)),
            ),
            (
                -east - 1,
                FieldState::Invalid(undefined(-i64::from(east) - 1)),
            ),
            (sentinel::LON_NOT_AVAILABLE as i32, FieldState::NotAvailable),
            (
                sentinel::LON_NOT_AVAILABLE as i32 + 1,
                FieldState::Invalid(undefined(sentinel::LON_NOT_AVAILABLE + 1)),
            ),
        ] {
            let pra = decode(&Fields {
                lon_raw: raw,
                ..Fields::default()
            });
            assert_eq!(pra.longitude_deg, expected, "raw {raw}");
        }
    }

    #[test]
    fn latitude_rows() {
        let north = 90 * 600_000;
        for (raw, expected) in [
            (north, FieldState::Value(90.0)),
            (-north, FieldState::Value(-90.0)),
            (
                north + 1,
                FieldState::Invalid(undefined(i64::from(north) + 1)),
            ),
            (
                -north - 1,
                FieldState::Invalid(undefined(-i64::from(north) - 1)),
            ),
            (sentinel::LAT_NOT_AVAILABLE as i32, FieldState::NotAvailable),
            // The 27-bit field's most negative code.
            (-(1 << 26), FieldState::Invalid(undefined(-(1 << 26)))),
        ] {
            let pra = decode(&Fields {
                lat_raw: raw,
                ..Fields::default()
            });
            assert_eq!(pra.latitude_deg, expected, "raw {raw}");
        }
    }

    // -----------------------------------------------------------------
    // Speed, course and heading rows
    // -----------------------------------------------------------------

    #[test]
    fn speed_over_ground_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0.0)),
            (255, FieldState::Value(25.5)),
            (1021, FieldState::Value(102.1)),
            (sentinel::SOG_OVER_RANGE, FieldState::AtLeast(102.2)),
            (sentinel::SOG_NOT_AVAILABLE, FieldState::NotAvailable),
        ] {
            let pra = decode(&Fields {
                sog: raw,
                ..Fields::default()
            });
            match (pra.speed_over_ground, expected) {
                (FieldState::Value(got), FieldState::Value(want))
                | (FieldState::AtLeast(got), FieldState::AtLeast(want)) => {
                    assert!((got - want).abs() < 1e-4, "raw {raw}");
                }
                (got, want) => assert_eq!(got, want, "raw {raw}"),
            }
        }
    }

    #[test]
    fn course_over_ground_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0.0)),
            (1234, FieldState::Value(123.4)),
            (3599, FieldState::Value(359.9)),
            (sentinel::COG_NOT_AVAILABLE, FieldState::NotAvailable),
            (3601, FieldState::Invalid(undefined(3601))),
            (4095, FieldState::Invalid(undefined(4095))),
        ] {
            let pra = decode(&Fields {
                cog: raw,
                ..Fields::default()
            });
            match (pra.course_over_ground, expected) {
                (FieldState::Value(got), FieldState::Value(want)) => {
                    assert!((got - want).abs() < 1e-4, "raw {raw}");
                }
                (got, want) => assert_eq!(got, want, "raw {raw}"),
            }
        }
    }

    #[test]
    fn true_heading_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0)),
            (42, FieldState::Value(42)),
            (359, FieldState::Value(359)),
            (360, FieldState::Invalid(undefined(360))),
            (510, FieldState::Invalid(undefined(510))),
            (sentinel::HEADING_NOT_AVAILABLE, FieldState::NotAvailable),
        ] {
            let pra = decode(&Fields {
                heading: raw,
                ..Fields::default()
            });
            assert_eq!(pra.true_heading, expected, "raw {raw}");
        }
    }

    // -----------------------------------------------------------------
    // Timestamp: seconds, the not-available code and the three statuses
    // -----------------------------------------------------------------

    #[test]
    fn timestamp_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(Timestamp::Second(0))),
            (59, FieldState::Value(Timestamp::Second(59))),
            (sentinel::TIMESTAMP_NOT_AVAILABLE, FieldState::NotAvailable),
            (
                sentinel::TIMESTAMP_MANUAL_INPUT,
                FieldState::Value(Timestamp::PositioningStatus(PositioningStatus::ManualInput)),
            ),
            (
                sentinel::TIMESTAMP_DEAD_RECKONING,
                FieldState::Value(Timestamp::PositioningStatus(
                    PositioningStatus::DeadReckoning,
                )),
            ),
            (
                sentinel::TIMESTAMP_INOPERATIVE,
                FieldState::Value(Timestamp::PositioningStatus(PositioningStatus::Inoperative)),
            ),
        ] {
            let pra = decode(&Fields {
                timestamp: raw,
                ..Fields::default()
            });
            assert_eq!(pra.timestamp, expected, "raw {raw}");
        }
    }

    // -----------------------------------------------------------------
    // Navigation status and manoeuvre indicator rows
    // -----------------------------------------------------------------

    #[test]
    fn navigation_status_defined_codes_are_values() {
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
        ] {
            let pra = decode(&Fields {
                nav: code,
                ..Fields::default()
            });
            assert_eq!(
                pra.navigation_status,
                FieldState::Value(expected),
                "code {code}"
            );
            assert_eq!(expected.code(), code);
        }
    }

    #[test]
    fn navigation_status_reserved_codes_are_invalid_with_the_raw_code() {
        for code in 9u8..=13 {
            let pra = decode(&Fields {
                nav: code,
                ..Fields::default()
            });
            assert_eq!(
                pra.navigation_status,
                FieldState::Invalid(undefined(i64::from(code))),
                "code {code}"
            );
        }
        let pra = decode(&Fields {
            nav: 15,
            ..Fields::default()
        });
        assert_eq!(pra.navigation_status, FieldState::NotAvailable);
    }

    #[test]
    fn maneuver_indicator_rows() {
        for (code, expected) in [
            (0u8, FieldState::NotAvailable),
            (1, FieldState::Value(ManeuverIndicator::NoSpecial)),
            (2, FieldState::Value(ManeuverIndicator::Special)),
            (3, FieldState::Invalid(undefined(3))),
        ] {
            let pra = decode(&Fields {
                maneuver: code,
                ..Fields::default()
            });
            assert_eq!(pra.special_maneuver, expected, "code {code}");
        }
        assert_eq!(ManeuverIndicator::NoSpecial.code(), 1);
        assert_eq!(ManeuverIndicator::Special.code(), 2);
    }

    // -----------------------------------------------------------------
    // Rate of turn (signed decoding is the single most
    // error-prone area)
    // -----------------------------------------------------------------

    #[test]
    fn rate_of_turn_minus_128_is_not_available() {
        let (bits, total) = build_pra_rot(1, sentinel::ROT_NOT_AVAILABLE);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.rate_of_turn, FieldState::NotAvailable);
    }

    #[test]
    fn rate_of_turn_zero_means_not_turning() {
        let (bits, total) = build_pra_rot(1, 0);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(
            pra.rate_of_turn,
            FieldState::Value(RateOfTurn::DegPerMin(0.0))
        );
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
            FieldState::Value(RateOfTurn::NoIndicator(TurnDirection::Right))
        );
    }

    #[test]
    fn rate_of_turn_minus_127_is_no_indicator_left() {
        let (bits, total) = build_pra_rot(1, sentinel::ROT_NO_INDICATOR_LEFT);
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(
            pra.rate_of_turn,
            FieldState::Value(RateOfTurn::NoIndicator(TurnDirection::Left))
        );
    }

    /// Pins the armored form of the ±127 payloads so the Python unit
    /// test `test_position_report_a_rate_of_turn_no_indicator` (bindings)
    /// can decode the same bits without a Python-side bit packer. 168
    /// bits armor to exactly 28 characters with zero fill bits.
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

    /// Pins the armored form of a payload whose every partitioned field
    /// carries an undefined or over-range code, so the Python unit test
    /// `test_position_report_a_invalid_and_over_range_states` (bindings)
    /// can decode the same bits: nav 9, SOG 1022, longitude +181° + 1,
    /// latitude +91° + 1, COG 3601, heading 360, timestamp 61, manoeuvre 3.
    #[test]
    fn undefined_code_payload_armors_to_known_string() {
        let (bits, total) = build_pra(&Fields {
            mmsi: 123_456_789,
            nav: 9,
            sog: sentinel::SOG_OVER_RANGE,
            lon_raw: sentinel::LON_NOT_AVAILABLE as i32 + 1,
            lat_raw: sentinel::LAT_NOT_AVAILABLE as i32 + 1,
            cog: 3601,
            heading: 360,
            timestamp: sentinel::TIMESTAMP_MANUAL_INPUT,
            maneuver: 3,
            ..Fields::default()
        });
        let pra = decode_position_report_a(&bits, total).unwrap();
        assert_eq!(pra.navigation_status, FieldState::Invalid(undefined(9)));
        assert_eq!(pra.speed_over_ground, FieldState::AtLeast(102.2));
        assert_eq!(
            pra.longitude_deg,
            FieldState::Invalid(undefined(sentinel::LON_NOT_AVAILABLE + 1))
        );
        assert_eq!(
            pra.latitude_deg,
            FieldState::Invalid(undefined(sentinel::LAT_NOT_AVAILABLE + 1))
        );
        assert_eq!(pra.course_over_ground, FieldState::Invalid(undefined(3601)));
        assert_eq!(pra.true_heading, FieldState::Invalid(undefined(360)));
        assert_eq!(
            pra.timestamp,
            FieldState::Value(Timestamp::PositioningStatus(PositioningStatus::ManualInput))
        );
        assert_eq!(pra.special_maneuver, FieldState::Invalid(undefined(3)));
        let (armored, fill) = armor_encode(&bits, total);
        assert_eq!(fill, 0);
        assert_eq!(crate::armor::decode(&armored, 0).unwrap(), (bits, total));
        assert_eq!(
            core::str::from_utf8(&armored).unwrap(),
            "11mg=5I0?v<tSF2l4Q@N4KAsP000"
        );
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
            let pra = decode(&Fields {
                msg_type,
                mmsi: 987_654_321,
                nav: 1,
                sog: 100,
                pos_acc: true,
                heading: 180,
                ..Fields::default()
            });
            assert_eq!(pra.mmsi, 987_654_321);
            assert_eq!(pra.speed_over_ground, FieldState::Value(10.0));
            assert_eq!(pra.true_heading, FieldState::Value(180));
        }
    }

    // -----------------------------------------------------------------
    // RAIM flag round-trips
    // -----------------------------------------------------------------

    #[test]
    fn raim_flag_round_trips() {
        for raim in [false, true] {
            let pra = decode(&Fields {
                raim,
                ..Fields::default()
            });
            assert_eq!(pra.raim, raim);
        }
    }
}
