//! GGA — Global Positioning System Fix Data.

use marlin_field::{FieldState, Invalid, RawCode};
use marlin_nmea_envelope::RawSentence;

use crate::util::{latitude, longitude, number};
use crate::DecodeError;

use super::UtcTime;

/// Decoded fields of a `$__GGA` sentence.
///
/// The talker is preserved so `$GPGGA`, `$INGGA`, and `$GNGGA` all
/// decode to `GgaData` with distinct [`talker`](Self::talker) values —
/// the talker is source metadata, not dispatch.
///
/// An empty NMEA field is not available. This is semantically distinct
/// from zero and must not be conflated — a receiver that cannot compute
/// HDOP reports an empty field, not `0.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct GgaData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` is not expected
    /// here — GGA is not proprietary — but the field is `Option` to
    /// match [`RawSentence::talker`]'s shape.
    pub talker: Option<[u8; 2]>,
    /// UTC time of the position fix.
    pub utc: FieldState<UtcTime>,
    /// Latitude in signed decimal degrees (north positive). A paired
    /// field with the `N`/`S` hemisphere letter.
    pub latitude_deg: FieldState<f64>,
    /// Longitude in signed decimal degrees (east positive). A paired
    /// field with the `E`/`W` hemisphere letter.
    pub longitude_deg: FieldState<f64>,
    /// Fix quality indicator. A digit the standard leaves undefined is
    /// invalid with the digit as its raw code.
    pub fix_quality: FieldState<GgaFixQuality>,
    /// Number of satellites used in the fix.
    pub satellites_used: FieldState<u8>,
    /// Horizontal dilution of precision.
    pub hdop: FieldState<f32>,
    /// Altitude above mean sea level, in metres.
    pub altitude_m: FieldState<f32>,
    /// Geoidal separation (difference between WGS-84 ellipsoid and MSL),
    /// in metres.
    pub geoid_separation_m: FieldState<f32>,
    /// Age of differential GPS corrections, in seconds.
    pub dgps_age_s: FieldState<f32>,
    /// Differential reference station ID.
    pub dgps_station_id: FieldState<u16>,
}

/// GPS fix quality indicator — first field after time/lat/lon/hemi.
///
/// A status field: the sender's own statement of fix quality. `0` is
/// [`Self::NoFix`], a value the sender reported, not the invalid field
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GgaFixQuality {
    /// `0` — no fix.
    NoFix,
    /// `1` — GPS fix (standalone).
    GpsFix,
    /// `2` — differential GPS fix.
    DgpsFix,
    /// `3` — Precise Positioning Service fix.
    PpsFix,
    /// `4` — Real-Time Kinematic, fixed ambiguities.
    RtkFixed,
    /// `5` — Real-Time Kinematic, float ambiguities.
    RtkFloat,
    /// `6` — dead reckoning.
    DeadReckoning,
    /// `7` — manual input.
    ManualInput,
    /// `8` — simulator mode.
    Simulator,
}

impl GgaFixQuality {
    /// The variant for a fix-quality digit, `None` for a digit the
    /// standard leaves undefined.
    fn from_digit(digit: u8) -> Option<Self> {
        match digit {
            0 => Some(Self::NoFix),
            1 => Some(Self::GpsFix),
            2 => Some(Self::DgpsFix),
            3 => Some(Self::PpsFix),
            4 => Some(Self::RtkFixed),
            5 => Some(Self::RtkFloat),
            6 => Some(Self::DeadReckoning),
            7 => Some(Self::ManualInput),
            8 => Some(Self::Simulator),
            _ => None,
        }
    }
}

/// Field indices within the GGA payload (0-based). GGA has 14 fields
/// after the address (`__GGA`):
///
/// ```text
/// 0  : UTC time        (hhmmss[.ss])
/// 1  : Latitude        (ddmm.mmmm)
/// 2  : N/S indicator
/// 3  : Longitude       (dddmm.mmmm)
/// 4  : E/W indicator
/// 5  : Fix quality     (0..=8)
/// 6  : Satellites used (integer)
/// 7  : HDOP
/// 8  : Altitude (MSL)
/// 9  : Altitude unit (always M)
/// 10 : Geoid separation
/// 11 : Geoid separation unit (always M)
/// 12 : Age of DGPS correction (s)
/// 13 : DGPS station ID
/// ```
const GGA_MIN_FIELDS: usize = 14;

/// Decode a GGA sentence into typed fields.
///
/// The caller is responsible for verifying `raw.sentence_type == "GGA"`
/// before calling this — the top-level [`decode`](crate::decode)
/// dispatcher does so.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if the payload has fewer than 14
///   fields.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_gga(raw: &RawSentence<'_>) -> Result<GgaData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < GGA_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: GGA_MIN_FIELDS,
            got: f.len(),
        });
    }

    Ok(GgaData {
        talker: raw.talker,
        utc: number(f[0]),
        latitude_deg: latitude(f[1], f[2]),
        longitude_deg: longitude(f[3], f[4]),
        fix_quality: fix_quality(f[5]),
        satellites_used: number(f[6]),
        hdop: number(f[7]),
        altitude_m: number(f[8]),
        geoid_separation_m: number(f[10]),
        dgps_age_s: number(f[12]),
        dgps_station_id: number(f[13]),
    })
}

/// Decode the fix-quality field: a number read as a digit code.
///
/// Empty → `NotAvailable`; a defined digit → `Value`; another number →
/// `Invalid(Undefined(RawCode(n)))` with the digit, not the ASCII byte,
/// as the raw code; non-numeric text → `Invalid(Unparsable)`.
fn fix_quality(bytes: &[u8]) -> FieldState<GgaFixQuality> {
    match number::<u8>(bytes) {
        FieldState::Value(digit) => GgaFixQuality::from_digit(digit).map_or(
            FieldState::Invalid(Invalid::Undefined(RawCode(digit.into()))),
            FieldState::Value,
        ),
        FieldState::NotAvailable => FieldState::NotAvailable,
        FieldState::Invalid(why) => FieldState::Invalid(why),
        FieldState::AtLeast(_) | FieldState::SenderError(_) => {
            // `number` never produces these states.
            FieldState::Invalid(Invalid::Unparsable)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::testing::{build, parse_raw, unparsable};

    fn decode(body: &[u8]) -> GgaData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_gga(&raw).expect("parse")
    }

    #[test]
    fn decode_gga_classic_spec_example() {
        let raw = parse_raw(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
        let gga = decode_gga(&raw).expect("parse");

        assert_eq!(gga.talker, Some(*b"GP"));
        assert_eq!(
            gga.utc,
            FieldState::Value(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert!((gga.latitude_deg.value().unwrap() - 48.1173).abs() < 0.0001);
        assert!((gga.longitude_deg.value().unwrap() - 11.51667).abs() < 0.0001);
        assert_eq!(gga.fix_quality, FieldState::Value(GgaFixQuality::GpsFix));
        assert_eq!(gga.satellites_used, FieldState::Value(8));
        assert!((gga.hdop.value().unwrap() - 0.9).abs() < 0.001);
        assert!((gga.altitude_m.value().unwrap() - 545.4).abs() < 0.01);
        assert!((gga.geoid_separation_m.value().unwrap() - 46.9).abs() < 0.01);
        assert_eq!(gga.dgps_age_s, FieldState::NotAvailable);
        assert_eq!(gga.dgps_station_id, FieldState::NotAvailable);
    }

    #[test]
    fn decode_gga_all_empty_fields_are_not_available() {
        let gga = decode(b"GPGGA,,,,,,,,,,,,,,");

        assert_eq!(gga.utc, FieldState::NotAvailable);
        assert_eq!(gga.latitude_deg, FieldState::NotAvailable);
        assert_eq!(gga.longitude_deg, FieldState::NotAvailable);
        assert_eq!(gga.fix_quality, FieldState::NotAvailable);
        assert_eq!(gga.satellites_used, FieldState::NotAvailable);
        assert_eq!(gga.hdop, FieldState::NotAvailable);
        assert_eq!(gga.altitude_m, FieldState::NotAvailable);
        assert_eq!(gga.geoid_separation_m, FieldState::NotAvailable);
        assert_eq!(gga.dgps_age_s, FieldState::NotAvailable);
        assert_eq!(gga.dgps_station_id, FieldState::NotAvailable);
    }

    #[test]
    fn decode_gga_every_unreadable_field_is_invalid() {
        let gga = decode(b"GPGGA,x,x,N,x,E,x,x,x,x,M,x,M,x,x");

        assert_eq!(gga.utc, unparsable());
        assert_eq!(gga.latitude_deg, unparsable());
        assert_eq!(gga.longitude_deg, unparsable());
        assert_eq!(gga.fix_quality, unparsable());
        assert_eq!(gga.satellites_used, unparsable());
        assert_eq!(gga.hdop, unparsable());
        assert_eq!(gga.altitude_m, unparsable());
        assert_eq!(gga.geoid_separation_m, unparsable());
        assert_eq!(gga.dgps_age_s, unparsable());
        assert_eq!(gga.dgps_station_id, unparsable());
    }

    #[test]
    fn decode_gga_southern_western_coordinates_are_negative() {
        let gga = decode(b"GPGGA,123519,4807.038,S,01131.000,W,1,08,0.9,545.4,M,46.9,M,,");
        assert!(gga.latitude_deg.value().unwrap() < 0.0);
        assert!(gga.longitude_deg.value().unwrap() < 0.0);
    }

    #[test]
    fn decode_gga_lowercase_hemisphere_is_accepted() {
        let gga = decode(b"GPGGA,123519,4807.038,s,01131.000,w,1,08,0.9,545.4,M,46.9,M,,");
        assert!(gga.latitude_deg.value().unwrap() < 0.0);
        assert!(gga.longitude_deg.value().unwrap() < 0.0);
    }

    // -----------------------------------------------------------------
    // Paired fields: latitude and longitude
    // -----------------------------------------------------------------

    #[test]
    fn decode_gga_latitude_without_hemisphere_is_invalid() {
        let gga = decode(b"GPGGA,123519,4807.038,,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
        assert!(gga.longitude_deg.value().is_some());
    }

    #[test]
    fn decode_gga_hemisphere_without_latitude_is_invalid() {
        let gga = decode(b"GPGGA,123519,,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_unnamed_hemisphere_is_invalid_with_the_byte() {
        let gga = decode(b"GPGGA,123519,4807.038,X,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(
            gga.latitude_deg,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_gga_longitude_letter_in_latitude_pair_is_invalid_with_the_byte() {
        let gga = decode(b"GPGGA,123519,4807.038,E,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(
            gga.latitude_deg,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'E'))))
        );
    }

    #[test]
    fn decode_gga_two_byte_hemisphere_is_invalid_without_a_code() {
        let gga = decode(b"GPGGA,123519,4807.038,NS,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_unparsable_latitude_magnitude_is_invalid() {
        let gga = decode(b"GPGGA,123519,north,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_non_finite_latitude_magnitude_is_invalid() {
        let gga = decode(b"GPGGA,123519,nan,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_latitude_beyond_90_degrees_is_invalid() {
        let gga = decode(b"GPGGA,123519,9500.000,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_longitude_beyond_180_degrees_is_invalid() {
        let gga = decode(b"GPGGA,123519,4807.038,N,18100.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.longitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_boundary_coordinates_are_values() {
        let gga = decode(b"GPGGA,123519,9000.000,S,18000.000,W,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Value(-90.0));
        assert_eq!(gga.longitude_deg, FieldState::Value(-180.0));
    }

    #[test]
    fn decode_gga_negative_magnitude_is_invalid() {
        // The sign lives in the hemisphere letter; a signed magnitude is
        // not a coordinate.
        let gga = decode(b"GPGGA,123519,-4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.latitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Fix quality: a digit code
    // -----------------------------------------------------------------

    #[test]
    fn decode_gga_every_defined_fix_quality_digit() {
        for (digit, expected) in [
            (b"0", GgaFixQuality::NoFix),
            (b"1", GgaFixQuality::GpsFix),
            (b"2", GgaFixQuality::DgpsFix),
            (b"3", GgaFixQuality::PpsFix),
            (b"4", GgaFixQuality::RtkFixed),
            (b"5", GgaFixQuality::RtkFloat),
            (b"6", GgaFixQuality::DeadReckoning),
            (b"7", GgaFixQuality::ManualInput),
            (b"8", GgaFixQuality::Simulator),
        ] {
            let mut body = b"GPGGA,123519,4807.038,N,01131.000,E,".to_vec();
            body.extend_from_slice(digit);
            body.extend_from_slice(b",08,0.9,545.4,M,46.9,M,,");
            assert_eq!(decode(&body).fix_quality, FieldState::Value(expected));
        }
    }

    #[test]
    fn decode_gga_undefined_fix_quality_digit_is_invalid_with_the_digit() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,9,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(
            gga.fix_quality,
            FieldState::Invalid(Invalid::Undefined(RawCode(9)))
        );
    }

    #[test]
    fn decode_gga_two_digit_fix_quality_is_invalid_with_the_number() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,12,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(
            gga.fix_quality,
            FieldState::Invalid(Invalid::Undefined(RawCode(12)))
        );
    }

    #[test]
    fn decode_gga_non_numeric_fix_quality_is_invalid_without_a_code() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,A,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.fix_quality, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Numbers
    // -----------------------------------------------------------------

    #[test]
    fn decode_gga_unparsable_utc_is_invalid_and_the_rest_decodes() {
        let gga = decode(b"GPGGA,12351,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.utc, FieldState::Invalid(Invalid::Unparsable));
        assert!(gga.latitude_deg.value().is_some());
        assert_eq!(gga.satellites_used, FieldState::Value(8));
    }

    #[test]
    fn decode_gga_utc_hour_out_of_range_is_invalid() {
        let gga = decode(b"GPGGA,243519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(gga.utc, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_satellites_beyond_u8_is_invalid() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,1,300,0.9,545.4,M,46.9,M,,");
        assert_eq!(
            gga.satellites_used,
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn decode_gga_unparsable_hdop_is_invalid() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,1,08,x,545.4,M,46.9,M,,");
        assert_eq!(gga.hdop, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gga_unparsable_station_id_is_invalid() {
        let gga = decode(b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,2.0,id");
        assert!((gga.dgps_age_s.value().unwrap() - 2.0).abs() < 0.001);
        assert_eq!(
            gga.dgps_station_id,
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn decode_gga_rejects_too_few_fields() {
        let bytes = build(b"GPGGA,1,2,3");
        let raw = parse_raw(&bytes);
        match decode_gga(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 14,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }
}
