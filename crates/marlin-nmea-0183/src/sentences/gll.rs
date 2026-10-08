//! GLL — Geographic Position, Latitude/Longitude.
//!
//! Position-only fix sentence. Carries latitude, longitude, UTC time,
//! and a single-byte validity status. NMEA 2.3+ adds a mode indicator;
//! pre-2.3 sentences omit it.

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::util::{code, latitude, longitude, number, optional};
use crate::DecodeError;

use super::{DataStatus, UtcTime, VtgMode};

/// Decoded fields of a `$__GLL` sentence.
///
/// The talker ID is preserved — `$GPGLL`, `$GNGLL`, `$INGLL` all decode
/// to `GllData` with distinct [`talker`](Self::talker) values.
///
/// An empty NMEA field is not available. Safety-critical consumers
/// should reject [`Self::status`] of `DataStatus::Void` before using the
/// position values in the same sentence; the status does not change
/// their field state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GllData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`).
    pub talker: Option<[u8; 2]>,
    /// Latitude in signed decimal degrees (north positive). A paired
    /// field with the `N`/`S` hemisphere letter.
    pub latitude_deg: FieldState<f64>,
    /// Longitude in signed decimal degrees (east positive). A paired
    /// field with the `E`/`W` hemisphere letter.
    pub longitude_deg: FieldState<f64>,
    /// UTC time-of-day of the position fix.
    pub utc: FieldState<UtcTime>,
    /// Validity status — `A` (active/valid) or `V` (void/invalid).
    pub status: FieldState<DataStatus>,
    /// Mode indicator (NMEA 2.3+). Not available if the sentence
    /// predates 2.3 or the field is present but empty.
    pub mode: FieldState<VtgMode>,
}

/// GLL has at least 6 fields in pre-NMEA-2.3 form:
///
/// ```text
/// 0 : Latitude         ddmm.mmmm
/// 1 : N/S
/// 2 : Longitude        dddmm.mmmm
/// 3 : E/W
/// 4 : UTC time         hhmmss[.ss]
/// 5 : Status           A=valid, V=void
/// 6 : Mode indicator   (NMEA 2.3+)   — optional
/// ```
const GLL_MIN_FIELDS: usize = 6;

/// Decode a GLL sentence into typed fields.
///
/// The caller asserts the type by calling this — the top-level
/// [`decode`](crate::decode) dispatcher does so.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if the payload has fewer than 6
///   fields.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_gll(raw: &RawSentence<'_>) -> Result<GllData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < GLL_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: GLL_MIN_FIELDS,
            got: f.len(),
        });
    }

    Ok(GllData {
        talker: raw.talker,
        latitude_deg: latitude(f[0], f[1]),
        longitude_deg: longitude(f[2], f[3]),
        utc: number(f[4]),
        status: code(f[5], DataStatus::from_byte),
        mode: code(optional(f, 6), VtgMode::from_byte),
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

    fn decode(body: &[u8]) -> GllData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_gll(&raw).expect("parse")
    }

    // -----------------------------------------------------------------
    // Happy path — NMEA 2.3+ full sentence
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_full_with_mode() {
        let raw = parse_raw(b"$GPGLL,4916.45,N,12311.12,W,225444,A,A*5C");
        let gll = decode_gll(&raw).expect("parse");

        assert_eq!(gll.talker, Some(*b"GP"));
        assert!((gll.latitude_deg.value().unwrap() - 49.27417).abs() < 0.0001);
        assert!((gll.longitude_deg.value().unwrap() - (-123.18533)).abs() < 0.0001);
        assert_eq!(
            gll.utc,
            FieldState::Value(UtcTime {
                hour: 22,
                minute: 54,
                second: 44,
                millisecond: 0
            })
        );
        assert_eq!(gll.status, FieldState::Value(DataStatus::Active));
        assert_eq!(gll.mode, FieldState::Value(VtgMode::Autonomous));
    }

    #[test]
    fn decode_gll_every_empty_field_is_not_available() {
        let gll = decode(b"GPGLL,,,,,,,");
        assert_eq!(gll.latitude_deg, FieldState::NotAvailable);
        assert_eq!(gll.longitude_deg, FieldState::NotAvailable);
        assert_eq!(gll.utc, FieldState::NotAvailable);
        assert_eq!(gll.status, FieldState::NotAvailable);
        assert_eq!(gll.mode, FieldState::NotAvailable);
    }

    #[test]
    fn decode_gll_every_unreadable_field_is_invalid() {
        let gll = decode(b"GPGLL,x,N,x,W,x,AV,AD");
        assert_eq!(gll.latitude_deg, unparsable());
        assert_eq!(gll.longitude_deg, unparsable());
        assert_eq!(gll.utc, unparsable());
        assert_eq!(gll.status, unparsable());
        assert_eq!(gll.mode, unparsable());
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form — no mode indicator field
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_pre_nmea_2_3_mode_is_not_available() {
        let bytes = build(b"GPGLL,4916.45,N,12311.12,W,225444,A");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 6);
        let gll = decode_gll(&raw).expect("parse");
        assert_eq!(gll.mode, FieldState::NotAvailable);
    }

    #[test]
    fn decode_gll_empty_mode_field_is_not_available() {
        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,A,");
        assert_eq!(gll.mode, FieldState::NotAvailable);
    }

    // -----------------------------------------------------------------
    // Status field: void qualifies, never stamps the other fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_void_status_leaves_sibling_states_alone() {
        let gll = decode(b"GPGLL,,,,,,V,N");
        assert_eq!(gll.status, FieldState::Value(DataStatus::Void));
        assert_eq!(gll.latitude_deg, FieldState::NotAvailable);
        assert_eq!(gll.longitude_deg, FieldState::NotAvailable);
        assert_eq!(gll.utc, FieldState::NotAvailable);
        assert_eq!(gll.mode, FieldState::Value(VtgMode::NotValid));

        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,V,N");
        assert_eq!(gll.status, FieldState::Value(DataStatus::Void));
        assert!((gll.latitude_deg.value().unwrap() - 49.27417).abs() < 0.0001);
    }

    #[test]
    fn decode_gll_empty_status_is_not_available() {
        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,,A");
        assert_eq!(gll.status, FieldState::NotAvailable);
    }

    #[test]
    fn decode_gll_lowercase_status_is_accepted() {
        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,a,A");
        assert_eq!(gll.status, FieldState::Value(DataStatus::Active));
    }

    #[test]
    fn decode_gll_unnamed_status_byte_is_invalid_with_the_byte() {
        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,X,A");
        assert_eq!(
            gll.status,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_gll_two_byte_status_is_invalid_without_a_code() {
        let gll = decode(b"GPGLL,4916.45,N,12311.12,W,225444,AV,A");
        assert_eq!(gll.status, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Southern + western hemispheres flip sign
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_southern_western_coordinates_are_negative() {
        let gll = decode(b"GPGLL,4807.038,S,01131.000,W,123519,A,A");
        assert!(gll.latitude_deg.value().unwrap() < 0.0);
        assert!(gll.longitude_deg.value().unwrap() < 0.0);
    }

    #[test]
    fn decode_gll_half_filled_longitude_pair_is_invalid() {
        let gll = decode(b"GPGLL,4807.038,S,,W,123519,A,A");
        assert!(gll.latitude_deg.value().is_some());
        assert_eq!(gll.longitude_deg, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_gll_unparsable_utc_is_invalid() {
        let gll = decode(b"GPGLL,4807.038,S,01131.000,W,12:35:19,A,A");
        assert_eq!(gll.utc, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Field count gate
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_rejects_too_few_fields() {
        let bytes = build(b"GPGLL,4916.45,N");
        let raw = parse_raw(&bytes);
        match decode_gll(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 6,
                got: 2,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }
}
