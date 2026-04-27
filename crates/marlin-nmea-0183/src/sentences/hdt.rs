//! HDT — True Heading.

use marlin_nmea_envelope::RawSentence;

use crate::util::optional_f32;
use crate::DecodeError;

/// Decoded fields of a `$__HDT` sentence.
///
/// HDT carries a single data field: the true heading in degrees. A
/// second trailing field `T` (unit indicator) is always present and
/// checked structurally.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdtData {
    /// Two-byte talker ID (e.g. `Some(*b"IN")` for an INS,
    /// `Some(*b"HE")` for a heading sensor).
    pub talker: Option<[u8; 2]>,
    /// True heading in degrees (0..360). `None` for an empty field.
    pub heading_true_deg: Option<f32>,
}

/// Minimum fields for a HDT payload: heading + `T` unit indicator.
const HDT_MIN_FIELDS: usize = 2;

/// Decode an HDT sentence.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if fewer than 2 fields.
/// - [`DecodeError::InvalidNumber`] if the heading field is non-empty
///   but not a valid float.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_hdt(raw: &RawSentence<'_>) -> Result<HdtData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < HDT_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: HDT_MIN_FIELDS,
            got: f.len(),
        });
    }

    let heading_true_deg = optional_f32(f[0], 0)?;

    Ok(HdtData {
        talker: raw.talker,
        heading_true_deg,
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
    fn decode_hdt_basic() {
        let bytes = build(b"INHDT,123.456,T");
        let raw = parse_raw(&bytes);
        let hdt = decode_hdt(&raw).expect("parse");
        assert_eq!(hdt.talker, Some(*b"IN"));
        assert!((hdt.heading_true_deg.unwrap() - 123.456).abs() < 0.001);
    }

    #[test]
    fn decode_hdt_empty_heading_is_none() {
        let bytes = build(b"HEHDT,,T");
        let raw = parse_raw(&bytes);
        let hdt = decode_hdt(&raw).expect("parse");
        assert_eq!(hdt.heading_true_deg, None);
    }

    #[test]
    fn decode_hdt_rejects_too_few_fields() {
        let bytes = build(b"GPHDT,123.4");
        let raw = parse_raw(&bytes);
        match decode_hdt(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 2,
                got: 1,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }
}
