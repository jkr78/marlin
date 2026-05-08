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
//! optional and decode to `None` when absent or empty.

use marlin_nmea_envelope::RawSentence;

use crate::util::{non_empty, optional_coordinate, optional_f32};
use crate::DecodeError;

use super::{DataStatus, UtcTime, VtgMode};

/// Decoded fields of an `$__RMC` sentence.
///
/// The talker ID is preserved — `$GPRMC`, `$GNRMC`, `$INRMC` all decode
/// to `RmcData` with distinct [`talker`](Self::talker) values.
///
/// Empty NMEA fields decode to `None`. The [`status`](Self::status)
/// field is the receiver's own validity assertion; safety-critical
/// consumers should reject `DataStatus::Void` regardless of how
/// well-formed the other fields look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmcData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` is not expected
    /// here — RMC is not proprietary — but the field shape matches
    /// [`RawSentence::talker`].
    pub talker: Option<[u8; 2]>,
    /// UTC time-of-day of the position fix.
    pub utc: Option<UtcTime>,
    /// Validity status — `A` (active/valid) or `V` (void/invalid).
    pub status: DataStatus,
    /// Latitude in signed decimal degrees (north positive).
    pub latitude_deg: Option<f64>,
    /// Longitude in signed decimal degrees (east positive).
    pub longitude_deg: Option<f64>,
    /// Speed over ground in knots.
    pub speed_knots: Option<f32>,
    /// Course over ground, true (degrees).
    pub course_true_deg: Option<f32>,
    /// UTC date (`ddmmyy`). The 2-digit year is preserved as raw —
    /// callers apply their own century-resolution rule.
    pub date: Option<UtcDate>,
    /// Magnetic variation in signed decimal degrees (east positive,
    /// west negative). `None` for empty fields.
    pub magnetic_variation_deg: Option<f32>,
    /// Mode indicator (NMEA 2.3+). `None` if the sentence predates 2.3
    /// or the field is present but empty.
    pub mode: Option<VtgMode>,
    /// Navigational status (NMEA 4.10+). `None` for sentences that
    /// predate 4.10 or where the field is empty.
    pub nav_status: Option<RmcNavStatus>,
}

/// UTC calendar date as carried by RMC's `ddmmyy` field.
///
/// The 2-digit year is preserved verbatim — the spec does not pin a
/// pivot year for century resolution, and different vendors use
/// different rules. Consumers convert to a full year by their own
/// policy (typically `+ 2000` if `year_yy < 80`, else `+ 1900`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDate {
    /// Day of month (1..=31).
    pub day: u8,
    /// Month of year (1..=12).
    pub month: u8,
    /// Year, last two digits (0..=99).
    pub year_yy: u8,
}

impl UtcDate {
    #[allow(clippy::indexing_slicing)] // length validated above each slice
    pub(crate) fn parse(bytes: &[u8], field_index: usize) -> Result<Self, DecodeError> {
        if bytes.len() != 6 || !bytes.iter().all(u8::is_ascii_digit) {
            return Err(DecodeError::InvalidUtcTime { field_index });
        }
        let s =
            core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtcTime { field_index })?;
        let parse_pair = |src: &str| -> Result<u8, DecodeError> {
            src.parse::<u8>()
                .map_err(|_| DecodeError::InvalidUtcTime { field_index })
        };
        let day = parse_pair(&s[0..2])?;
        let month = parse_pair(&s[2..4])?;
        let year_yy = parse_pair(&s[4..6])?;
        if !(1..=31).contains(&day) || !(1..=12).contains(&month) {
            return Err(DecodeError::InvalidUtcTime { field_index });
        }
        Ok(Self {
            day,
            month,
            year_yy,
        })
    }
}

/// NMEA 4.10+ navigational-status byte — 13th field of RMC.
///
/// Used by ECDIS-aware receivers to signal a higher-level safety
/// assessment than the receiver-internal `Status` byte.
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
    /// Any letter not covered above; raw byte preserved.
    Other(u8),
}

impl RmcNavStatus {
    fn from_byte(b: u8) -> Self {
        match b {
            b'S' | b's' => Self::Safe,
            b'C' | b'c' => Self::Caution,
            b'U' | b'u' => Self::Unsafe,
            b'V' | b'v' => Self::NotValid,
            other => Self::Other(other),
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
/// - [`DecodeError::NotEnoughFields`] if the payload has fewer than 11
///   fields.
/// - [`DecodeError::InvalidUtcTime`] for malformed time or date.
/// - [`DecodeError::InvalidHemisphere`] for an empty value paired with
///   a non-empty direction byte (or vice versa) in the variation field.
/// - [`DecodeError::InvalidNumber`], [`DecodeError::InvalidUtf8`],
///   [`DecodeError::OutOfRange`] for per-field malformations.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_rmc(raw: &RawSentence<'_>) -> Result<RmcData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < RMC_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: RMC_MIN_FIELDS,
            got: f.len(),
        });
    }

    let utc = if f[0].is_empty() {
        None
    } else {
        Some(UtcTime::parse(f[0], 0)?)
    };

    let status = match f[1].first() {
        None => DataStatus::Other(0),
        Some(&b) => DataStatus::from_byte(b),
    };

    let latitude_deg = optional_coordinate(f[2], f[3], 2, 3, false)?;
    let longitude_deg = optional_coordinate(f[4], f[5], 4, 5, true)?;

    let speed_knots = optional_f32(f[6], 6)?;
    let course_true_deg = optional_f32(f[7], 7)?;

    let date = if f[8].is_empty() {
        None
    } else {
        Some(UtcDate::parse(f[8], 8)?)
    };

    // Magnetic variation: magnitude in field 9, direction (E/W) in
    // field 10. Both empty → None. One empty / one not → InvalidHemisphere.
    let magnetic_variation_deg = match (f[9].is_empty(), f[10].is_empty()) {
        (true, true) => None,
        (true, false) | (false, true) => {
            return Err(DecodeError::InvalidHemisphere { field_index: 10 });
        }
        (false, false) => {
            let mag =
                optional_f32(f[9], 9)?.ok_or(DecodeError::InvalidNumber { field_index: 9 })?;
            let dir = *f[10].first().unwrap_or(&0);
            let signed = match dir {
                b'E' | b'e' => mag,
                b'W' | b'w' => -mag,
                _ => {
                    return Err(DecodeError::InvalidHemisphere { field_index: 10 });
                }
            };
            Some(signed)
        }
    };

    let mode = f
        .get(11)
        .and_then(|bytes| non_empty(bytes))
        .and_then(|bytes| bytes.first().copied())
        .map(VtgMode::from_byte);

    let nav_status = f
        .get(12)
        .and_then(|bytes| non_empty(bytes))
        .and_then(|bytes| bytes.first().copied())
        .map(RmcNavStatus::from_byte);

    Ok(RmcData {
        talker: raw.talker,
        utc,
        status,
        latitude_deg,
        longitude_deg,
        speed_knots,
        course_true_deg,
        date,
        magnetic_variation_deg,
        mode,
        nav_status,
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
    fn decode_rmc_full_with_mode() {
        // Build with auto-computed checksum to avoid hand-typing one.
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A");
        let raw = parse_raw(&bytes);
        let rmc = decode_rmc(&raw).expect("parse");

        assert_eq!(rmc.talker, Some(*b"GP"));
        assert_eq!(
            rmc.utc,
            Some(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert_eq!(rmc.status, DataStatus::Active);
        assert!((rmc.latitude_deg.unwrap() - 48.1173).abs() < 0.0001);
        assert!((rmc.longitude_deg.unwrap() - 11.51667).abs() < 0.0001);
        assert!((rmc.speed_knots.unwrap() - 22.4).abs() < 0.01);
        assert!((rmc.course_true_deg.unwrap() - 84.4).abs() < 0.01);
        assert_eq!(
            rmc.date,
            Some(UtcDate {
                day: 23,
                month: 3,
                year_yy: 94
            })
        );
        assert!((rmc.magnetic_variation_deg.unwrap() - (-3.1)).abs() < 0.01);
        assert_eq!(rmc.mode, Some(VtgMode::Autonomous));
        assert_eq!(rmc.nav_status, None);
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form — no mode indicator field at all
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_pre_nmea_2_3_returns_none_mode() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 11);
        let rmc = decode_rmc(&raw).expect("parse");
        assert_eq!(rmc.mode, None);
        assert_eq!(rmc.nav_status, None);
    }

    // -----------------------------------------------------------------
    // NMEA 4.10+ form — mode + nav status
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_with_nav_status() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,S");
        let raw = parse_raw(&bytes);
        let rmc = decode_rmc(&raw).expect("parse");
        assert_eq!(rmc.mode, Some(VtgMode::Autonomous));
        assert_eq!(rmc.nav_status, Some(RmcNavStatus::Safe));
    }

    // -----------------------------------------------------------------
    // Void status — receiver flagged the fix as unreliable
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_void_status_propagates() {
        let bytes = build(b"GPRMC,,V,,,,,,,,,,N");
        let raw = parse_raw(&bytes);
        let rmc = decode_rmc(&raw).expect("parse");
        assert_eq!(rmc.status, DataStatus::Void);
        assert_eq!(rmc.utc, None);
        assert_eq!(rmc.latitude_deg, None);
        assert_eq!(rmc.mode, Some(VtgMode::NotValid));
    }

    // -----------------------------------------------------------------
    // Eastern variation reads positive
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_eastern_variation_is_positive() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,E,A");
        let raw = parse_raw(&bytes);
        let rmc = decode_rmc(&raw).expect("parse");
        assert!((rmc.magnetic_variation_deg.unwrap() - 5.0).abs() < 0.01);
    }

    // -----------------------------------------------------------------
    // Date validation
    // -----------------------------------------------------------------

    #[test]
    fn utc_date_parses_ddmmyy() {
        let d = UtcDate::parse(b"230394", 8).unwrap();
        assert_eq!(d.day, 23);
        assert_eq!(d.month, 3);
        assert_eq!(d.year_yy, 94);
    }

    #[test]
    fn utc_date_rejects_wrong_length() {
        match UtcDate::parse(b"23039", 8) {
            Err(DecodeError::InvalidUtcTime { field_index: 8 }) => {}
            other => panic!("expected InvalidUtcTime, got {other:?}"),
        }
    }

    #[test]
    fn utc_date_rejects_invalid_month() {
        match UtcDate::parse(b"231394", 8) {
            Err(DecodeError::InvalidUtcTime { field_index: 8 }) => {}
            other => panic!("expected InvalidUtcTime, got {other:?}"),
        }
    }

    #[test]
    fn utc_date_rejects_zero_day() {
        match UtcDate::parse(b"000394", 8) {
            Err(DecodeError::InvalidUtcTime { field_index: 8 }) => {}
            other => panic!("expected InvalidUtcTime, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Magnetic variation — partial fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_variation_magnitude_without_direction_errors() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,,A");
        let raw = parse_raw(&bytes);
        match decode_rmc(&raw) {
            Err(DecodeError::InvalidHemisphere { field_index: 10 }) => {}
            other => panic!("expected InvalidHemisphere, got {other:?}"),
        }
    }

    #[test]
    fn decode_rmc_variation_direction_without_magnitude_errors() {
        let bytes = build(b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,,W,A");
        let raw = parse_raw(&bytes);
        match decode_rmc(&raw) {
            Err(DecodeError::InvalidHemisphere { field_index: 10 }) => {}
            other => panic!("expected InvalidHemisphere, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Field count gate
    // -----------------------------------------------------------------

    #[test]
    fn decode_rmc_rejects_too_few_fields() {
        let bytes = build(b"GPRMC,123519,A,4807.038");
        let raw = parse_raw(&bytes);
        match decode_rmc(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 11,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Nav status — every recognized variant
    // -----------------------------------------------------------------

    #[test]
    fn rmc_nav_status_covers_all_recognized_letters() {
        for (byte, expected) in [
            (b'S', RmcNavStatus::Safe),
            (b'C', RmcNavStatus::Caution),
            (b'U', RmcNavStatus::Unsafe),
            (b'V', RmcNavStatus::NotValid),
            (b'X', RmcNavStatus::Other(b'X')),
        ] {
            assert_eq!(RmcNavStatus::from_byte(byte), expected);
        }
    }
}
