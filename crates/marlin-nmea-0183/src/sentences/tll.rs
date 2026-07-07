//! TLL — Target Latitude/Longitude (radar/ARPA target position).
//!
//! `$--TLL,xx,llll.ll,a,yyyyy.yy,a,c--c,hhmmss.ss,a,a*hh`. 9 data
//! fields; `name`, `utc_time`, `status`, and `reference_target` are
//! optional (older/partial reports omit trailing fields).

use alloc::string::String;

use marlin_nmea_envelope::RawSentence;

use crate::sentences::status::TargetStatus;
use crate::sentences::utc_time::UtcTime;
use crate::util::{optional_coordinate, optional_string, optional_u16};
use crate::DecodeError;

/// Decoded fields of a `$__TLL` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct TllData {
    /// Two-byte talker ID (e.g. `Some(*b"RA")` for radar).
    pub talker: Option<[u8; 2]>,
    /// Target number (00–99 per spec; wider values tolerated).
    pub target_number: Option<u16>,
    /// Target latitude in signed decimal degrees (`S` negative).
    pub latitude_deg: Option<f64>,
    /// Target longitude in signed decimal degrees (`W` negative).
    pub longitude_deg: Option<f64>,
    /// Target label.
    pub name: Option<String>,
    /// UTC time of the data. `None` if absent or empty.
    pub utc_time: Option<UtcTime>,
    /// Tracking state.
    pub status: Option<TargetStatus>,
    /// `true` when this target is flagged (`R`) as the reference target.
    pub reference_target: bool,
}

/// Minimum fields: target number + latitude pair + longitude pair
/// (indices 0–4). Name (5), UTC time (6), status (7), and reference
/// target (8) are optional and read via `get`.
const TLL_MIN_FIELDS: usize = 5;

/// Decode a TLL sentence.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if fewer than 5 fields.
/// - [`DecodeError::InvalidNumber`] / [`DecodeError::OutOfRange`] /
///   [`DecodeError::InvalidHemisphere`] on a malformed coordinate.
/// - [`DecodeError::InvalidUtf8`] on a non-UTF-8 target name.
/// - [`DecodeError::InvalidUtcTime`] on a malformed non-empty UTC field.
#[allow(clippy::indexing_slicing)] // indices 0..5 validated; 5..9 via get
pub fn decode_tll(raw: &RawSentence<'_>) -> Result<TllData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < TLL_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: TLL_MIN_FIELDS,
            got: f.len(),
        });
    }

    let target_number = optional_u16(f[0], 0)?;
    let latitude_deg = optional_coordinate(f[1], f[2], 1, 2, false)?;
    let longitude_deg = optional_coordinate(f[3], f[4], 3, 4, true)?;
    let name = match f.get(5) {
        Some(bytes) => optional_string(bytes, 5)?,
        None => None,
    };
    let utc_time = match f.get(6) {
        Some(bytes) => UtcTime::parse_optional(bytes, 6)?,
        None => None,
    };
    let status = f
        .get(7)
        .and_then(|b| b.first().copied())
        .map(TargetStatus::from_byte);
    let reference_target = matches!(f.get(8).and_then(|b| b.first().copied()), Some(b'R' | b'r'));

    Ok(TllData {
        talker: raw.talker,
        target_number,
        latitude_deg,
        longitude_deg,
        name,
        utc_time,
        status,
        reference_target,
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
    fn decode_tll_full() {
        let bytes = build(b"RATLL,7,4807.038,N,01131.000,E,TGT7,123519,T,R");
        let raw = parse_raw(&bytes);
        let tll = decode_tll(&raw).expect("parse");
        assert_eq!(tll.talker, Some(*b"RA"));
        assert_eq!(tll.target_number, Some(7));
        assert!((tll.latitude_deg.unwrap() - 48.1173).abs() < 0.0001);
        assert!((tll.longitude_deg.unwrap() - 11.51667).abs() < 0.0001);
        assert_eq!(tll.name.as_deref(), Some("TGT7"));
        assert_eq!(
            tll.utc_time,
            Some(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert_eq!(tll.status, Some(TargetStatus::Tracking));
        assert!(tll.reference_target);
    }

    #[test]
    fn decode_tll_southern_western_negative() {
        let bytes = build(b"RATLL,1,4807.038,S,01131.000,W,,,L,");
        let raw = parse_raw(&bytes);
        let tll = decode_tll(&raw).expect("parse");
        assert!(tll.latitude_deg.unwrap() < 0.0);
        assert!(tll.longitude_deg.unwrap() < 0.0);
        assert_eq!(tll.name, None);
        assert_eq!(tll.utc_time, None);
        assert_eq!(tll.status, Some(TargetStatus::Lost));
        assert!(!tll.reference_target);
    }

    #[test]
    fn decode_tll_position_only_five_fields() {
        let bytes = build(b"RATLL,2,5000.00,N,00500.00,E");
        let raw = parse_raw(&bytes);
        let tll = decode_tll(&raw).expect("parse");
        assert_eq!(tll.target_number, Some(2));
        assert!(tll.latitude_deg.is_some());
        assert_eq!(tll.name, None);
        assert_eq!(tll.status, None);
        assert!(!tll.reference_target);
    }

    #[test]
    fn decode_tll_rejects_too_few_fields() {
        let bytes = build(b"RATLL,2,5000.00,N");
        let raw = parse_raw(&bytes);
        match decode_tll(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 5,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_tll_rejects_invalid_hemisphere() {
        let bytes = build(b"RATLL,2,5000.00,X,00500.00,E");
        let raw = parse_raw(&bytes);
        match decode_tll(&raw) {
            Err(DecodeError::InvalidHemisphere { field_index: 2 }) => {}
            other => panic!("expected InvalidHemisphere 2, got {other:?}"),
        }
    }
}
