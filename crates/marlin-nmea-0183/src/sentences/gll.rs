//! GLL — Geographic Position, Latitude/Longitude.
//!
//! Position-only fix sentence. Carries latitude, longitude, UTC time,
//! and a single-byte validity status. NMEA 2.3+ adds a mode indicator;
//! pre-2.3 sentences omit it.

use marlin_nmea_envelope::RawSentence;

use crate::util::{non_empty, optional_coordinate};
use crate::DecodeError;

use super::{DataStatus, UtcTime, VtgMode};

/// Decoded fields of a `$__GLL` sentence.
///
/// The talker ID is preserved — `$GPGLL`, `$GNGLL`, `$INGLL` all decode
/// to `GllData` with distinct [`talker`](Self::talker) values.
///
/// Empty NMEA fields decode to `None`. Safety-critical consumers should
/// reject [`Self::status`] of `DataStatus::Void` before using the
/// position values in the same sentence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GllData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`).
    pub talker: Option<[u8; 2]>,
    /// Latitude in signed decimal degrees (north positive).
    pub latitude_deg: Option<f64>,
    /// Longitude in signed decimal degrees (east positive).
    pub longitude_deg: Option<f64>,
    /// UTC time-of-day of the position fix.
    pub utc: Option<UtcTime>,
    /// Validity status — `A` (active/valid) or `V` (void/invalid).
    pub status: DataStatus,
    /// Mode indicator (NMEA 2.3+). `None` if the sentence predates 2.3
    /// or the field is present but empty.
    pub mode: Option<VtgMode>,
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
/// - [`DecodeError::InvalidUtcTime`] for a malformed UTC time.
/// - [`DecodeError::InvalidNumber`], [`DecodeError::InvalidHemisphere`],
///   [`DecodeError::OutOfRange`] for per-field malformations.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_gll(raw: &RawSentence<'_>) -> Result<GllData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < GLL_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: GLL_MIN_FIELDS,
            got: f.len(),
        });
    }

    let latitude_deg = optional_coordinate(f[0], f[1], 0, 1, false)?;
    let longitude_deg = optional_coordinate(f[2], f[3], 2, 3, true)?;

    let utc = if f[4].is_empty() {
        None
    } else {
        Some(UtcTime::parse(f[4], 4)?)
    };

    let status = match f[5].first() {
        None => DataStatus::Other(0),
        Some(&b) => DataStatus::from_byte(b),
    };

    let mode = f
        .get(6)
        .and_then(|bytes| non_empty(bytes))
        .and_then(|bytes| bytes.first().copied())
        .map(VtgMode::from_byte);

    Ok(GllData {
        talker: raw.talker,
        latitude_deg,
        longitude_deg,
        utc,
        status,
        mode,
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

    // -----------------------------------------------------------------
    // Happy path — NMEA 2.3+ full sentence
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_full_with_mode() {
        let raw = parse_raw(b"$GPGLL,4916.45,N,12311.12,W,225444,A,A*5C");
        let gll = decode_gll(&raw).expect("parse");

        assert_eq!(gll.talker, Some(*b"GP"));
        assert!((gll.latitude_deg.unwrap() - 49.27417).abs() < 0.0001);
        assert!((gll.longitude_deg.unwrap() - (-123.18533)).abs() < 0.0001);
        assert_eq!(
            gll.utc,
            Some(UtcTime {
                hour: 22,
                minute: 54,
                second: 44,
                millisecond: 0
            })
        );
        assert_eq!(gll.status, DataStatus::Active);
        assert_eq!(gll.mode, Some(VtgMode::Autonomous));
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form — no mode indicator field
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_pre_nmea_2_3_returns_none_mode() {
        let bytes = build(b"GPGLL,4916.45,N,12311.12,W,225444,A");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 6);
        let gll = decode_gll(&raw).expect("parse");
        assert_eq!(gll.mode, None);
    }

    // -----------------------------------------------------------------
    // Empty mode field — distinct from pre-2.3
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_empty_mode_field_is_none() {
        let bytes = build(b"GPGLL,4916.45,N,12311.12,W,225444,A,");
        let raw = parse_raw(&bytes);
        let gll = decode_gll(&raw).expect("parse");
        assert_eq!(gll.mode, None);
    }

    // -----------------------------------------------------------------
    // Void status — propagates regardless of other fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_void_status_propagates() {
        let bytes = build(b"GPGLL,,,,,,V,N");
        let raw = parse_raw(&bytes);
        let gll = decode_gll(&raw).expect("parse");
        assert_eq!(gll.status, DataStatus::Void);
        assert_eq!(gll.latitude_deg, None);
        assert_eq!(gll.longitude_deg, None);
        assert_eq!(gll.utc, None);
        assert_eq!(gll.mode, Some(VtgMode::NotValid));
    }

    // -----------------------------------------------------------------
    // Southern + western hemispheres flip sign
    // -----------------------------------------------------------------

    #[test]
    fn decode_gll_southern_western_coordinates_are_negative() {
        let bytes = build(b"GPGLL,4807.038,S,01131.000,W,123519,A,A");
        let raw = parse_raw(&bytes);
        let gll = decode_gll(&raw).expect("parse");
        assert!(gll.latitude_deg.unwrap() < 0.0);
        assert!(gll.longitude_deg.unwrap() < 0.0);
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
