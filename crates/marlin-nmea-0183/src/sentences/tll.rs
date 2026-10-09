//! TLL — Target Latitude/Longitude (radar/ARPA target position).
//!
//! `$--TLL,xx,llll.ll,a,yyyyy.yy,a,c--c,hhmmss.ss,a,a*hh`. 9 data
//! fields; `name`, `utc_time`, `status`, and `reference_target` are
//! optional (older/partial reports omit trailing fields).

use alloc::string::String;

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::sentences::status::TargetStatus;
use crate::sentences::utc_time::UtcTime;
use crate::util::{code, latitude, longitude, number, optional, reference_target, text};
use crate::Nmea0183DecodeError;

/// Decoded fields of a `$__TLL` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct TllData {
    /// Two-byte talker ID (e.g. `Some(*b"RA")` for radar).
    pub talker: Option<[u8; 2]>,
    /// Target number (00–99 per spec; wider values tolerated).
    pub target_number: FieldState<u16>,
    /// Target latitude in signed decimal degrees (`S` negative). A
    /// paired field with the `N`/`S` hemisphere letter.
    pub latitude_deg: FieldState<f64>,
    /// Target longitude in signed decimal degrees (`W` negative). A
    /// paired field with the `E`/`W` hemisphere letter.
    pub longitude_deg: FieldState<f64>,
    /// Target label. Not available when absent or empty.
    pub name: FieldState<String>,
    /// UTC time of the data. Not available when absent or empty.
    pub utc_time: FieldState<UtcTime>,
    /// Tracking state. Not available when absent or empty.
    pub status: FieldState<TargetStatus>,
    /// `true` when this target is flagged (`R`) as the reference target.
    /// An empty or absent field is `Value(false)`, the standard's
    /// encoding of "not the reference target"; never not available.
    pub reference_target: FieldState<bool>,
}

/// Minimum fields: target number + latitude pair + longitude pair
/// (indices 0–4). Name (5), UTC time (6), status (7), and reference
/// target (8) are optional and read via `get`.
const TLL_MIN_FIELDS: usize = 5;

/// Decode a TLL sentence.
///
/// # Errors
///
/// - [`Nmea0183DecodeError::NotEnoughFields`] if fewer than 5 fields.
#[allow(clippy::indexing_slicing)] // indices 0..5 validated; 5..9 via get
pub fn decode_tll(raw: &RawSentence<'_>) -> Result<TllData, Nmea0183DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < TLL_MIN_FIELDS {
        return Err(Nmea0183DecodeError::NotEnoughFields {
            expected: TLL_MIN_FIELDS,
            got: f.len(),
        });
    }
    Ok(TllData {
        talker: raw.talker,
        target_number: number(f[0]),
        latitude_deg: latitude(f[1], f[2]),
        longitude_deg: longitude(f[3], f[4]),
        name: text(optional(f, 5)),
        utc_time: number(optional(f, 6)),
        status: code(optional(f, 7), TargetStatus::from_byte),
        reference_target: reference_target(optional(f, 8)),
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
    use alloc::string::ToString;

    use marlin_field::{Invalid, RawCode};

    use super::*;
    use crate::testing::{build, parse_raw, unparsable};

    fn decode(body: &[u8]) -> TllData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_tll(&raw).expect("parse")
    }

    #[test]
    fn decode_tll_full() {
        let tll = decode(b"RATLL,7,4807.038,N,01131.000,E,TGT7,123519,T,R");
        assert_eq!(tll.talker, Some(*b"RA"));
        assert_eq!(tll.target_number, FieldState::Value(7));
        assert!((tll.latitude_deg.value().unwrap() - 48.1173).abs() < 0.0001);
        assert!((tll.longitude_deg.value().unwrap() - 11.51667).abs() < 0.0001);
        assert_eq!(tll.name, FieldState::Value("TGT7".to_string()));
        assert_eq!(
            tll.utc_time,
            FieldState::Value(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert_eq!(tll.status, FieldState::Value(TargetStatus::Tracking));
        assert_eq!(tll.reference_target, FieldState::Value(true));
    }

    #[test]
    fn decode_tll_southern_western_negative() {
        let tll = decode(b"RATLL,1,4807.038,S,01131.000,W,,,L,");
        assert!(tll.latitude_deg.value().unwrap() < 0.0);
        assert!(tll.longitude_deg.value().unwrap() < 0.0);
        assert_eq!(tll.name, FieldState::NotAvailable);
        assert_eq!(tll.utc_time, FieldState::NotAvailable);
        assert_eq!(tll.status, FieldState::Value(TargetStatus::Lost));
        assert_eq!(tll.reference_target, FieldState::Value(false));
    }

    #[test]
    fn decode_tll_position_only_five_fields() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E");
        assert_eq!(tll.target_number, FieldState::Value(2));
        assert!(tll.latitude_deg.value().is_some());
        assert_eq!(tll.name, FieldState::NotAvailable);
        assert_eq!(tll.utc_time, FieldState::NotAvailable);
        assert_eq!(tll.status, FieldState::NotAvailable);
        assert_eq!(tll.reference_target, FieldState::Value(false));
    }

    #[test]
    fn decode_tll_every_empty_field_is_not_available() {
        let tll = decode(b"RATLL,,,,,,,,,");
        assert_eq!(tll.target_number, FieldState::NotAvailable);
        assert_eq!(tll.latitude_deg, FieldState::NotAvailable);
        assert_eq!(tll.longitude_deg, FieldState::NotAvailable);
        assert_eq!(tll.name, FieldState::NotAvailable);
        assert_eq!(tll.utc_time, FieldState::NotAvailable);
        assert_eq!(tll.status, FieldState::NotAvailable);
        assert_eq!(tll.reference_target, FieldState::Value(false));
    }

    #[test]
    fn decode_tll_every_unreadable_field_is_invalid() {
        let tll = decode(b"RATLL,x,x,N,x,E,\xFF,x,LQ,RR");
        assert_eq!(tll.target_number, unparsable());
        assert_eq!(tll.latitude_deg, unparsable());
        assert_eq!(tll.longitude_deg, unparsable());
        assert_eq!(tll.name, unparsable());
        assert_eq!(tll.utc_time, unparsable());
        assert_eq!(tll.status, unparsable());
        assert_eq!(tll.reference_target, unparsable());
    }

    #[test]
    fn decode_tll_rejects_too_few_fields() {
        let bytes = build(b"RATLL,2,5000.00,N");
        let raw = parse_raw(&bytes);
        match decode_tll(&raw) {
            Err(Nmea0183DecodeError::NotEnoughFields {
                expected: 5,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_tll_unnamed_hemisphere_is_invalid_with_the_byte() {
        let tll = decode(b"RATLL,2,5000.00,X,00500.00,E");
        assert_eq!(
            tll.latitude_deg,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
        assert!(tll.longitude_deg.value().is_some());
    }

    #[test]
    fn decode_tll_unparsable_target_number_is_invalid() {
        let tll = decode(b"RATLL,two,5000.00,N,00500.00,E");
        assert_eq!(tll.target_number, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_tll_non_utf8_name_is_invalid() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,\xFF\xFE,,T,");
        assert_eq!(tll.name, FieldState::Invalid(Invalid::Unparsable));
        assert_eq!(tll.status, FieldState::Value(TargetStatus::Tracking));
    }

    #[test]
    fn decode_tll_unparsable_utc_is_invalid() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,TGT,noon,T,");
        assert_eq!(tll.utc_time, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_tll_unnamed_status_is_invalid_with_the_byte() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,TGT,,Z,");
        assert_eq!(
            tll.status,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'Z'))))
        );
    }

    #[test]
    fn decode_tll_lowercase_reference_flag_is_true() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,TGT,,T,r");
        assert_eq!(tll.reference_target, FieldState::Value(true));
    }

    #[test]
    fn decode_tll_unnamed_reference_flag_is_invalid_with_the_byte() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,TGT,,T,X");
        assert_eq!(
            tll.reference_target,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_tll_two_byte_reference_flag_is_invalid_without_a_code() {
        let tll = decode(b"RATLL,2,5000.00,N,00500.00,E,TGT,,T,RR");
        assert_eq!(
            tll.reference_target,
            FieldState::Invalid(Invalid::Unparsable)
        );
    }
}
