//! RMC — Recommended Minimum Specific GNSS Data.
//!
//! Single-sentence carrier of UTC time, date, position, speed,
//! course-over-ground, and magnetic variation, with a validity status
//! byte and an optional mode indicator. Many marine instruments emit
//! RMC as their primary fix sentence even when GGA and VTG are also
//! available.
//!
//! Three NMEA generations of the sentence in the wild:
//! - **Pre-2.3**: 11 fields (no mode indicator).
//! - **2.3+**: 12 fields, adds the mode indicator (A/D/E/N/M/S).
//! - **4.10+**: 13 fields, adds a navigational status byte (S/C/U/V).
//!
//! This decoder accepts all three forms — the trailing fields are
//! optional and not available when absent or empty.

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::util::{code, latitude, longitude, number, optional, signed_ew};
use crate::Nmea0183DecodeError;

use super::{DataStatus, UtcDate, UtcTime, VtgMode};

/// Decoded fields of an `$__RMC` sentence.
///
/// The talker ID is preserved — `$GPRMC`, `$GNRMC`, `$INRMC` all decode
/// to `RmcData` with distinct [`talker`](Self::talker) values.
///
/// An empty NMEA field is not available. The [`status`](Self::status)
/// field is the receiver's own validity assertion; safety-critical
/// consumers should reject `DataStatus::Void` regardless of how
/// well-formed the other fields look. The status does not change the
/// other fields' state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmcData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` is not expected
    /// here — RMC is not proprietary — but the field shape matches
    /// [`RawSentence::talker`].
    pub talker: Option<[u8; 2]>,
    /// UTC time-of-day of the position fix.
    pub utc: FieldState<UtcTime>,
    /// Validity status — `A` (active/valid) or `V` (void/invalid).
    pub status: FieldState<DataStatus>,
    /// Latitude in signed decimal degrees (north positive). A paired
    /// field with the `N`/`S` hemisphere letter.
    pub latitude_deg: FieldState<f64>,
    /// Longitude in signed decimal degrees (east positive). A paired
    /// field with the `E`/`W` hemisphere letter.
    pub longitude_deg: FieldState<f64>,
    /// Speed over ground in knots.
    pub speed_knots: FieldState<f32>,
    /// Course over ground, true (degrees).
    pub course_true_deg: FieldState<f32>,
    /// UTC date (`ddmmyy`). The 2-digit year is preserved as raw —
    /// callers apply their own century-resolution rule.
    pub date: FieldState<UtcDate>,
    /// Magnetic variation in signed decimal degrees (east positive,
    /// west negative). A paired field with the `E`/`W` direction letter.
    pub magnetic_variation_deg: FieldState<f32>,
    /// Mode indicator (NMEA 2.3+). Not available if the sentence
    /// predates 2.3 or the field is present but empty.
    pub mode: FieldState<VtgMode>,
    /// Navigational status (NMEA 4.10+). Not available for sentences
    /// that predate 4.10 or where the field is empty.
    pub nav_status: FieldState<RmcNavStatus>,
}

/// NMEA 4.10+ navigational-status byte — 13th field of RMC.
///
/// Used by ECDIS-aware receivers to signal a higher-level safety
/// assessment than the receiver-internal `Status` byte. An unnamed byte
/// decodes to the invalid field state with the byte as its raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RmcNavStatus {
    /// `S` — Safe.
    Safe,
    /// `C` — Caution.
    Caution,
    /// `U` — Unsafe.
    Unsafe,
    /// `V` — Navigational status not valid (equipment doesn't compute it).
    NotValid,
}

impl RmcNavStatus {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'S' | b's' => Some(Self::Safe),
            b'C' | b'c' => Some(Self::Caution),
            b'U' | b'u' => Some(Self::Unsafe),
            b'V' | b'v' => Some(Self::NotValid),
            _ => None,
        }
    }
}

/// RMC has at least 11 fields in pre-NMEA-2.3 form:
///
/// ```text
/// 0  : UTC time             hhmmss[.ss]
/// 1  : Status               A=valid, V=void
/// 2  : Latitude             ddmm.mmmm
/// 3  : N/S
/// 4  : Longitude            dddmm.mmmm
/// 5  : E/W
/// 6  : Speed over ground    knots
/// 7  : Course over ground   degrees true
/// 8  : Date                 ddmmyy
/// 9  : Magnetic variation   degrees (magnitude)
/// 10 : Magnetic variation   E/W (sign)
/// 11 : Mode indicator       (NMEA 2.3+)   — optional
/// 12 : Nav status           (NMEA 4.10+)  — optional
/// ```
const RMC_MIN_FIELDS: usize = 11;

/// Decode an RMC sentence into typed fields.
///
/// The caller asserts the type by calling this — the top-level
/// [`decode`](crate::decode) dispatcher does so.
///
/// # Errors
///
/// - [`Nmea0183DecodeError::NotEnoughFields`] if the payload has fewer than 11
///   fields.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_rmc(raw: &RawSentence<'_>) -> Result<RmcData, Nmea0183DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < RMC_MIN_FIELDS {
        return Err(Nmea0183DecodeError::NotEnoughFields {
            expected: RMC_MIN_FIELDS,
            got: f.len(),
        });
    }

    Ok(RmcData {
        talker: raw.talker,
        utc: number(f[0]),
        status: code(f[1], DataStatus::from_byte),
        latitude_deg: latitude(f[2], f[3]),
        longitude_deg: longitude(f[4], f[5]),
        speed_knots: number(f[6]),
        course_true_deg: number(f[7]),
        date: number(f[8]),
        magnetic_variation_deg: signed_ew(f[9], f[10]),
        mode: code(optional(f, 11), VtgMode::from_byte),
        nav_status: code(optional(f, 12), RmcNavStatus::from_byte),
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use marlin_field::{Invalid, RawCode};

    use super::*;
    use crate::testing::{build, parse_raw, unparsable};

    fn decode(body: &[u8]) -> RmcData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_rmc(&raw).expect("parse")
    }

    // -----------------------------------------------------------------
    // Happy path — NMEA 2.3+ full sentence
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_full_with_mode() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A");

        assert_eq!(rmc.talker, Some(*b"GP"));
        assert_eq!(
            rmc.utc,
            FieldState::Value(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert_eq!(rmc.status, FieldState::Value(DataStatus::Active));
        assert!((rmc.latitude_deg.value().unwrap() - 48.1173).abs() < 0.0001);
        assert!((rmc.longitude_deg.value().unwrap() - 11.51667).abs() < 0.0001);
        assert!((rmc.speed_knots.value().unwrap() - 22.4).abs() < 0.01);
        assert!((rmc.course_true_deg.value().unwrap() - 84.4).abs() < 0.01);
        assert_eq!(
            rmc.date,
            FieldState::Value(UtcDate {
                day: 23,
                month: 3,
                year_yy: 94
            })
        );
        assert!((rmc.magnetic_variation_deg.value().unwrap() - (-3.1)).abs() < 0.01);
        assert_eq!(rmc.mode, FieldState::Value(VtgMode::Autonomous));
        assert_eq!(rmc.nav_status, FieldState::NotAvailable);
    }

    #[test]
    fn decode_rmc_every_empty_field_is_not_available() {
        let rmc = decode(b"GPRMC,,,,,,,,,,,,,");
        assert_eq!(rmc.utc, FieldState::NotAvailable);
        assert_eq!(rmc.status, FieldState::NotAvailable);
        assert_eq!(rmc.latitude_deg, FieldState::NotAvailable);
        assert_eq!(rmc.longitude_deg, FieldState::NotAvailable);
        assert_eq!(rmc.speed_knots, FieldState::NotAvailable);
        assert_eq!(rmc.course_true_deg, FieldState::NotAvailable);
        assert_eq!(rmc.date, FieldState::NotAvailable);
        assert_eq!(rmc.magnetic_variation_deg, FieldState::NotAvailable);
        assert_eq!(rmc.mode, FieldState::NotAvailable);
        assert_eq!(rmc.nav_status, FieldState::NotAvailable);
    }

    #[test]
    fn decode_rmc_every_unreadable_field_is_invalid() {
        let rmc = decode(b"GPRMC,x,AV,x,N,x,E,x,x,x,x,W,AD,SC");
        assert_eq!(rmc.utc, unparsable());
        assert_eq!(rmc.status, unparsable());
        assert_eq!(rmc.latitude_deg, unparsable());
        assert_eq!(rmc.longitude_deg, unparsable());
        assert_eq!(rmc.speed_knots, unparsable());
        assert_eq!(rmc.course_true_deg, unparsable());
        assert_eq!(rmc.date, unparsable());
        assert_eq!(rmc.magnetic_variation_deg, unparsable());
        assert_eq!(rmc.mode, unparsable());
        assert_eq!(rmc.nav_status, unparsable());
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form — no mode indicator field at all
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_pre_nmea_2_3_trailing_fields_are_not_available() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 11);
        let rmc = decode_rmc(&raw).expect("parse");
        assert_eq!(rmc.mode, FieldState::NotAvailable);
        assert_eq!(rmc.nav_status, FieldState::NotAvailable);
    }

    // -----------------------------------------------------------------
    // NMEA 4.10+ form — mode + nav status
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_with_nav_status() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,S");
        assert_eq!(rmc.mode, FieldState::Value(VtgMode::Autonomous));
        assert_eq!(rmc.nav_status, FieldState::Value(RmcNavStatus::Safe));
    }

    #[test]
    fn decode_rmc_every_nav_status_letter() {
        for (letter, expected) in [
            (b"S", RmcNavStatus::Safe),
            (b"C", RmcNavStatus::Caution),
            (b"U", RmcNavStatus::Unsafe),
            (b"V", RmcNavStatus::NotValid),
            (b"c", RmcNavStatus::Caution),
        ] {
            let mut body =
                b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,".to_vec();
            body.extend_from_slice(letter);
            assert_eq!(decode(&body).nav_status, FieldState::Value(expected));
        }
    }

    #[test]
    fn decode_rmc_unnamed_nav_status_is_invalid_with_the_byte() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,X");
        assert_eq!(
            rmc.nav_status,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_rmc_two_byte_nav_status_is_invalid_without_a_code() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,SC");
        assert_eq!(rmc.nav_status, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Status field: void qualifies, never stamps the other fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_void_status_leaves_sibling_states_alone() {
        let rmc = decode(b"GPRMC,,V,,,,,,,,,,N");
        assert_eq!(rmc.status, FieldState::Value(DataStatus::Void));
        assert_eq!(rmc.utc, FieldState::NotAvailable);
        assert_eq!(rmc.latitude_deg, FieldState::NotAvailable);
        assert_eq!(rmc.magnetic_variation_deg, FieldState::NotAvailable);
        assert_eq!(rmc.mode, FieldState::Value(VtgMode::NotValid));
    }

    #[test]
    fn decode_rmc_empty_status_is_not_available() {
        let rmc = decode(b"GPRMC,123519,,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A");
        assert_eq!(rmc.status, FieldState::NotAvailable);
    }

    #[test]
    fn decode_rmc_unnamed_status_byte_is_invalid_with_the_byte() {
        let rmc = decode(b"GPRMC,123519,Z,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A");
        assert_eq!(
            rmc.status,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'Z'))))
        );
    }

    // -----------------------------------------------------------------
    // Magnetic variation: a paired field
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_eastern_variation_is_positive() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,E,A");
        assert!((rmc.magnetic_variation_deg.value().unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn decode_rmc_variation_magnitude_without_direction_is_invalid() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,,A");
        assert_eq!(
            rmc.magnetic_variation_deg,
            FieldState::Invalid(Invalid::Unparsable)
        );
        assert_eq!(rmc.mode, FieldState::Value(VtgMode::Autonomous));
    }

    #[test]
    fn decode_rmc_variation_direction_without_magnitude_is_invalid() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,,W,A");
        assert_eq!(
            rmc.magnetic_variation_deg,
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn decode_rmc_variation_direction_outside_the_pair_is_invalid_with_the_byte() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,N,A");
        assert_eq!(
            rmc.magnetic_variation_deg,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'N'))))
        );
    }

    // -----------------------------------------------------------------
    // Date and numbers
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_unparsable_date_is_invalid_and_the_rest_decodes() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,231394,003.1,W,A");
        assert_eq!(rmc.date, FieldState::Invalid(Invalid::Unparsable));
        assert!((rmc.speed_knots.value().unwrap() - 22.4).abs() < 0.01);
    }

    #[test]
    fn decode_rmc_zero_day_date_is_invalid() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,000394,003.1,W,A");
        assert_eq!(rmc.date, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_rmc_unparsable_speed_is_invalid() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,N,01131.000,E,fast,084.4,230394,003.1,W,A");
        assert_eq!(rmc.speed_knots, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_rmc_half_filled_latitude_pair_is_invalid() {
        let rmc = decode(b"GPRMC,123519,A,4807.038,,01131.000,E,022.4,084.4,230394,003.1,W,A");
        assert_eq!(rmc.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
        assert!(rmc.longitude_deg.value().is_some());
    }

    // -----------------------------------------------------------------
    // Field count gate
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_rejects_too_few_fields() {
        let bytes = build(b"GPRMC,123519,A,4807.038");
        let raw = parse_raw(&bytes);
        match decode_rmc(&raw) {
            Err(Nmea0183DecodeError::NotEnoughFields {
                expected: 11,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }
}
