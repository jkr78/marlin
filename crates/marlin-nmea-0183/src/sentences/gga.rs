//! GGA — Global Positioning System Fix Data.

use marlin_nmea_envelope::RawSentence;

use crate::util::{optional_coordinate, optional_f32, optional_u16, optional_u8};
use crate::DecodeError;

use super::UtcTime;

/// Decoded fields of a `$__GGA` sentence.
///
/// The talker ID is preserved so `$GPGGA`, `$INGGA`, and `$GNGGA` all
/// decode to `GgaData` with distinct [`talker`](Self::talker) values —
/// per PRD §D6, talker is source metadata, not dispatch.
///
/// Empty NMEA fields decode to `None`. This is semantically distinct
/// from zero and must not be conflated — a receiver that cannot compute
/// HDOP reports an empty field, not `0.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct GgaData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` is not expected
    /// here — GGA is not proprietary — but the field is `Option` to
    /// match [`RawSentence::talker`]'s shape.
    pub talker: Option<[u8; 2]>,
    /// UTC time of the position fix.
    pub utc: Option<UtcTime>,
    /// Latitude in signed decimal degrees (north positive).
    pub latitude_deg: Option<f64>,
    /// Longitude in signed decimal degrees (east positive).
    pub longitude_deg: Option<f64>,
    /// Fix quality indicator.
    pub fix_quality: GgaFixQuality,
    /// Number of satellites used in the fix.
    pub satellites_used: Option<u8>,
    /// Horizontal dilution of precision.
    pub hdop: Option<f32>,
    /// Altitude above mean sea level, in metres.
    pub altitude_m: Option<f32>,
    /// Geoidal separation (difference between WGS-84 ellipsoid and MSL),
    /// in metres.
    pub geoid_separation_m: Option<f32>,
    /// Age of differential GPS corrections, in seconds.
    pub dgps_age_s: Option<f32>,
    /// Differential reference station ID.
    pub dgps_station_id: Option<u16>,
}

/// GPS fix quality indicator — first field after time/lat/lon/hemi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GgaFixQuality {
    /// No fix.
    Invalid,
    /// GPS fix (standalone).
    GpsFix,
    /// Differential GPS fix.
    DgpsFix,
    /// Precise Positioning Service fix.
    PpsFix,
    /// Real-Time Kinematic, fixed ambiguities.
    RtkFixed,
    /// Real-Time Kinematic, float ambiguities.
    RtkFloat,
    /// Dead reckoning.
    DeadReckoning,
    /// Manual input.
    ManualInput,
    /// Simulator mode.
    Simulator,
    /// Any value not covered above; the raw byte is preserved.
    Other(u8),
}

impl GgaFixQuality {
    fn from_byte(b: u8) -> Self {
        match b {
            b'0' => Self::Invalid,
            b'1' => Self::GpsFix,
            b'2' => Self::DgpsFix,
            b'3' => Self::PpsFix,
            b'4' => Self::RtkFixed,
            b'5' => Self::RtkFloat,
            b'6' => Self::DeadReckoning,
            b'7' => Self::ManualInput,
            b'8' => Self::Simulator,
            other => Self::Other(other),
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
/// - [`DecodeError::InvalidNumber`], [`DecodeError::InvalidUtcTime`],
///   [`DecodeError::InvalidHemisphere`], [`DecodeError::OutOfRange`]
///   for per-field malformations.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_gga(raw: &RawSentence<'_>) -> Result<GgaData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < GGA_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: GGA_MIN_FIELDS,
            got: f.len(),
        });
    }

    // PRD §D2: empty-means-None must propagate through the whole row.
    let utc = if f[0].is_empty() {
        None
    } else {
        Some(UtcTime::parse(f[0], 0)?)
    };

    let latitude_deg = optional_coordinate(f[1], f[2], 1, 2, false)?;
    let longitude_deg = optional_coordinate(f[3], f[4], 3, 4, true)?;

    let fix_quality = match f[5] {
        b"" => GgaFixQuality::Invalid,
        single if single.len() == 1 => GgaFixQuality::from_byte(single[0]),
        _ => {
            return Err(DecodeError::InvalidNumber { field_index: 5 });
        }
    };

    let satellites_used = optional_u8(f[6], 6)?;
    let hdop = optional_f32(f[7], 7)?;
    let altitude_m = optional_f32(f[8], 8)?;
    let geoid_separation_m = optional_f32(f[10], 10)?;
    let dgps_age_s = optional_f32(f[12], 12)?;
    let dgps_station_id = optional_u16(f[13], 13)?;

    Ok(GgaData {
        talker: raw.talker,
        utc,
        latitude_deg,
        longitude_deg,
        fix_quality,
        satellites_used,
        hdop,
        altitude_m,
        geoid_separation_m,
        dgps_age_s,
        dgps_station_id,
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
    use super::*;
    use crate::testing::{build, parse_raw};

    #[test]
    fn decode_gga_classic_spec_example() {
        let raw = parse_raw(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
        let gga = decode_gga(&raw).expect("parse");

        assert_eq!(gga.talker, Some(*b"GP"));
        assert_eq!(
            gga.utc,
            Some(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert!((gga.latitude_deg.unwrap() - 48.1173).abs() < 0.0001);
        assert!((gga.longitude_deg.unwrap() - 11.51667).abs() < 0.0001);
        assert_eq!(gga.fix_quality, GgaFixQuality::GpsFix);
        assert_eq!(gga.satellites_used, Some(8));
        assert!((gga.hdop.unwrap() - 0.9).abs() < 0.001);
        assert!((gga.altitude_m.unwrap() - 545.4).abs() < 0.01);
        assert!((gga.geoid_separation_m.unwrap() - 46.9).abs() < 0.01);
        assert_eq!(gga.dgps_age_s, None);
        assert_eq!(gga.dgps_station_id, None);
    }

    #[test]
    fn decode_gga_all_empty_fields_decode_to_none() {
        let bytes = build(b"GPGGA,,,,,,0,,,,,,,,");
        let raw = parse_raw(&bytes);
        let gga = decode_gga(&raw).expect("parse");

        assert_eq!(gga.utc, None);
        assert_eq!(gga.latitude_deg, None);
        assert_eq!(gga.longitude_deg, None);
        assert_eq!(gga.fix_quality, GgaFixQuality::Invalid);
        assert_eq!(gga.satellites_used, None);
        assert_eq!(gga.hdop, None);
        assert_eq!(gga.altitude_m, None);
        assert_eq!(gga.geoid_separation_m, None);
    }

    #[test]
    fn decode_gga_southern_western_coordinates_are_negative() {
        let bytes = build(b"GPGGA,123519,4807.038,S,01131.000,W,1,08,0.9,545.4,M,46.9,M,,");
        let raw = parse_raw(&bytes);
        let gga = decode_gga(&raw).expect("parse");
        assert!(gga.latitude_deg.unwrap() < 0.0);
        assert!(gga.longitude_deg.unwrap() < 0.0);
    }

    #[test]
    fn decode_gga_various_fix_qualities() {
        for (byte, expected) in [
            (b'0', GgaFixQuality::Invalid),
            (b'1', GgaFixQuality::GpsFix),
            (b'2', GgaFixQuality::DgpsFix),
            (b'3', GgaFixQuality::PpsFix),
            (b'4', GgaFixQuality::RtkFixed),
            (b'5', GgaFixQuality::RtkFloat),
            (b'6', GgaFixQuality::DeadReckoning),
            (b'7', GgaFixQuality::ManualInput),
            (b'8', GgaFixQuality::Simulator),
            (b'9', GgaFixQuality::Other(b'9')),
        ] {
            assert_eq!(GgaFixQuality::from_byte(byte), expected);
        }
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
