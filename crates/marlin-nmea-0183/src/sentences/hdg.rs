//! HDG — Heading, Deviation & Variation.
//!
//! `$--HDG,x.x,x.x,a,x.x,a*hh`: magnetic sensor heading, then magnetic
//! deviation and variation each as a paired field, a magnitude and its
//! `E`/`W` direction. We expose the corrections as signed degrees (`E`
//! positive, `W` negative), the convention for correcting a magnetic
//! reading toward true.

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::util::{number, signed_ew};
use crate::DecodeError;

/// Decoded fields of a `$__HDG` sentence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdgData {
    /// Two-byte talker ID (e.g. `Some(*b"HC")` for a magnetic compass).
    pub talker: Option<[u8; 2]>,
    /// Magnetic sensor heading in degrees.
    pub heading_magnetic_deg: FieldState<f32>,
    /// Magnetic deviation in degrees, signed (`E` positive, `W`
    /// negative). A paired field: not available when both magnitude and
    /// direction are empty, invalid when only one of them is.
    pub deviation_deg: FieldState<f32>,
    /// Magnetic variation in degrees, signed (`E` positive, `W`
    /// negative). A paired field like [`deviation_deg`](Self::deviation_deg).
    pub variation_deg: FieldState<f32>,
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
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_hdg(raw: &RawSentence<'_>) -> Result<HdgData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < HDG_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: HDG_MIN_FIELDS,
            got: f.len(),
        });
    }

    Ok(HdgData {
        talker: raw.talker,
        heading_magnetic_deg: number(f[0]),
        deviation_deg: signed_ew(f[1], f[2]),
        variation_deg: signed_ew(f[3], f[4]),
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

    fn decode(body: &[u8]) -> HdgData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_hdg(&raw).expect("parse")
    }

    #[test]
    fn decode_hdg_full_signed() {
        let hdg = decode(b"HCHDG,98.3,0.0,E,12.6,W");
        assert_eq!(hdg.talker, Some(*b"HC"));
        assert!((hdg.heading_magnetic_deg.value().unwrap() - 98.3).abs() < 0.01);
        assert!((hdg.deviation_deg.value().unwrap() - 0.0).abs() < 0.01);
        assert!(
            (hdg.variation_deg.value().unwrap() - -12.6).abs() < 0.01,
            "W is negative"
        );
    }

    #[test]
    fn decode_hdg_heading_only_has_not_available_corrections() {
        let hdg = decode(b"HCHDG,310.1,,,,");
        assert!((hdg.heading_magnetic_deg.value().unwrap() - 310.1).abs() < 0.01);
        assert_eq!(hdg.deviation_deg, FieldState::NotAvailable);
        assert_eq!(hdg.variation_deg, FieldState::NotAvailable);
    }

    #[test]
    fn decode_hdg_every_empty_field_is_not_available() {
        let hdg = decode(b"HCHDG,,,,,");
        assert_eq!(hdg.heading_magnetic_deg, FieldState::NotAvailable);
        assert_eq!(hdg.deviation_deg, FieldState::NotAvailable);
        assert_eq!(hdg.variation_deg, FieldState::NotAvailable);
    }

    #[test]
    fn decode_hdg_every_unreadable_field_is_invalid() {
        let hdg = decode(b"HCHDG,x,x,E,x,W");
        assert_eq!(hdg.heading_magnetic_deg, unparsable());
        assert_eq!(hdg.deviation_deg, unparsable());
        assert_eq!(hdg.variation_deg, unparsable());
    }

    #[test]
    fn decode_hdg_east_variation_is_positive() {
        let hdg = decode(b"HCHDG,98.3,1.0,W,7.1,E");
        assert!((hdg.deviation_deg.value().unwrap() - -1.0).abs() < 0.01);
        assert!((hdg.variation_deg.value().unwrap() - 7.1).abs() < 0.01);
    }

    #[test]
    fn decode_hdg_lowercase_direction_is_accepted() {
        let hdg = decode(b"HCHDG,98.3,1.0,w,7.1,e");
        assert!((hdg.deviation_deg.value().unwrap() - -1.0).abs() < 0.01);
        assert!((hdg.variation_deg.value().unwrap() - 7.1).abs() < 0.01);
    }

    #[test]
    fn decode_hdg_magnitude_without_direction_is_invalid() {
        let hdg = decode(b"HCHDG,98.3,1.0,,7.1,E");
        assert_eq!(hdg.deviation_deg, FieldState::Invalid(Invalid::Unparsable));
        assert!((hdg.variation_deg.value().unwrap() - 7.1).abs() < 0.01);
    }

    #[test]
    fn decode_hdg_direction_without_magnitude_is_invalid() {
        let hdg = decode(b"HCHDG,98.3,,E,7.1,E");
        assert_eq!(hdg.deviation_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_hdg_unparsable_magnitude_is_invalid() {
        let hdg = decode(b"HCHDG,98.3,one,E,7.1,E");
        assert_eq!(hdg.deviation_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_hdg_non_finite_magnitude_is_invalid() {
        let hdg = decode(b"HCHDG,98.3,inf,E,nan,W");
        assert_eq!(hdg.deviation_deg, FieldState::Invalid(Invalid::Unparsable));
        assert_eq!(hdg.variation_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_hdg_direction_outside_the_pair_is_invalid_with_the_byte() {
        let hdg = decode(b"HCHDG,98.3,1.0,N,7.1,E");
        assert_eq!(
            hdg.deviation_deg,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'N'))))
        );
    }

    #[test]
    fn decode_hdg_two_byte_direction_is_invalid_without_a_code() {
        let hdg = decode(b"HCHDG,98.3,1.0,EW,7.1,E");
        assert_eq!(hdg.deviation_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_hdg_rejects_too_few_fields() {
        let bytes = build(b"HCHDG,98.3,1.0");
        let raw = parse_raw(&bytes);
        match decode_hdg(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 5,
                got: 2,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_hdg_unparsable_heading_is_invalid_not_a_sentence_failure() {
        let hdg = decode(b"HCHDG,not-a-number,0.0,E,12.6,W");
        assert_eq!(
            hdg.heading_magnetic_deg,
            FieldState::Invalid(Invalid::Unparsable)
        );
        assert!((hdg.variation_deg.value().unwrap() - -12.6).abs() < 0.01);
    }
}
