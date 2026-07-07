//! HDG — Heading, Deviation & Variation.
//!
//! `$--HDG,x.x,x.x,a,x.x,a*hh`: magnetic sensor heading, then magnetic
//! deviation and variation each as a magnitude + `E`/`W` direction. We
//! expose the corrections as signed degrees (`E` positive, `W`
//! negative), the convention for correcting a magnetic reading toward
//! true.

use marlin_nmea_envelope::RawSentence;

use crate::util::{optional_f32, optional_signed_ew};
use crate::DecodeError;

/// Decoded fields of a `$__HDG` sentence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdgData {
    /// Two-byte talker ID (e.g. `Some(*b"HC")` for a magnetic compass).
    pub talker: Option<[u8; 2]>,
    /// Magnetic sensor heading in degrees. `None` for an empty field.
    pub heading_magnetic_deg: Option<f32>,
    /// Magnetic deviation in degrees, signed (`E` positive, `W`
    /// negative). `None` when both magnitude and direction are empty.
    pub deviation_deg: Option<f32>,
    /// Magnetic variation in degrees, signed (`E` positive, `W`
    /// negative). `None` when both magnitude and direction are empty.
    pub variation_deg: Option<f32>,
}

/// HDG carries 5 fields: heading, deviation magnitude + dir, variation
/// magnitude + dir. Heading-only devices still emit all five comma
/// positions (`$HCHDG,310.1,,,,`); a truncated shorter sentence is
/// rejected.
const HDG_MIN_FIELDS: usize = 5;

/// Decode an HDG sentence.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if fewer than 5 fields.
/// - [`DecodeError::InvalidNumber`] if a numeric field is malformed.
/// - [`DecodeError::InvalidHemisphere`] if a deviation/variation
///   magnitude is present without its `E`/`W` direction (or vice versa),
///   or the direction is not `E`/`W`.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_hdg(raw: &RawSentence<'_>) -> Result<HdgData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < HDG_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: HDG_MIN_FIELDS,
            got: f.len(),
        });
    }

    let heading_magnetic_deg = optional_f32(f[0], 0)?;
    let deviation_deg = optional_signed_ew(f[1], f[2], 1, 2)?;
    let variation_deg = optional_signed_ew(f[3], f[4], 3, 4)?;

    Ok(HdgData {
        talker: raw.talker,
        heading_magnetic_deg,
        deviation_deg,
        variation_deg,
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
    fn decode_hdg_full_signed() {
        let bytes = build(b"HCHDG,98.3,0.0,E,12.6,W");
        let raw = parse_raw(&bytes);
        let hdg = decode_hdg(&raw).expect("parse");
        assert_eq!(hdg.talker, Some(*b"HC"));
        assert!((hdg.heading_magnetic_deg.unwrap() - 98.3).abs() < 0.01);
        assert!((hdg.deviation_deg.unwrap() - 0.0).abs() < 0.01);
        assert!((hdg.variation_deg.unwrap() - -12.6).abs() < 0.01, "W is negative");
    }

    #[test]
    fn decode_hdg_heading_only_has_none_corrections() {
        let bytes = build(b"HCHDG,310.1,,,,");
        let raw = parse_raw(&bytes);
        let hdg = decode_hdg(&raw).expect("parse");
        assert!((hdg.heading_magnetic_deg.unwrap() - 310.1).abs() < 0.01);
        assert_eq!(hdg.deviation_deg, None);
        assert_eq!(hdg.variation_deg, None);
    }

    #[test]
    fn decode_hdg_east_variation_is_positive() {
        let bytes = build(b"HCHDG,98.3,1.0,W,7.1,E");
        let raw = parse_raw(&bytes);
        let hdg = decode_hdg(&raw).expect("parse");
        assert!((hdg.deviation_deg.unwrap() - -1.0).abs() < 0.01);
        assert!((hdg.variation_deg.unwrap() - 7.1).abs() < 0.01);
    }

    #[test]
    fn decode_hdg_magnitude_without_direction_errors() {
        let bytes = build(b"HCHDG,98.3,1.0,,7.1,E");
        let raw = parse_raw(&bytes);
        match decode_hdg(&raw) {
            Err(DecodeError::InvalidHemisphere { field_index: 2 }) => {}
            other => panic!("expected InvalidHemisphere 2, got {other:?}"),
        }
    }

    #[test]
    fn decode_hdg_rejects_too_few_fields() {
        let bytes = build(b"HCHDG,98.3,1.0");
        let raw = parse_raw(&bytes);
        match decode_hdg(&raw) {
            Err(DecodeError::NotEnoughFields { expected: 5, got: 2 }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_hdg_rejects_malformed_heading() {
        let bytes = build(b"HCHDG,not-a-number,0.0,E,12.6,W");
        let raw = parse_raw(&bytes);
        match decode_hdg(&raw) {
            Err(DecodeError::InvalidNumber { field_index: 0 }) => {}
            other => panic!("expected InvalidNumber 0, got {other:?}"),
        }
    }
}
