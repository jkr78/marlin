//! TTM — Tracked Target Message (radar/ARPA).
//!
//! `$--TTM,xx,x.x,x.x,a,x.x,x.x,a,x.x,x.x,a,c--c,a,a,hhmmss.ss,a*hh`.
//! 15 data fields; the trailing `utc_time` (13) and `acquisition` (14)
//! were added in NMEA 3.0 and are optional. `$RATTM` (radar talker) is
//! this exact layout — the `RA` talker surfaces in [`TtmData::talker`].

use alloc::string::String;

use marlin_nmea_envelope::RawSentence;

use crate::sentences::status::TargetStatus;
use crate::sentences::utc_time::UtcTime;
use crate::util::{optional_f32, optional_string, optional_u16};
use crate::DecodeError;

/// Bearing/course reference, shared by TTM fields 3 and 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AngleReference {
    /// `T` — referenced to true north.
    True,
    /// `R` — relative to own ship's heading.
    Relative,
    /// Any byte not covered above; raw byte preserved.
    Other(u8),
}

impl AngleReference {
    pub(crate) fn from_byte(b: u8) -> Self {
        match b {
            b'T' | b't' => Self::True,
            b'R' | b'r' => Self::Relative,
            other => Self::Other(other),
        }
    }
}

/// Speed & distance units governing TTM fields 1, 4, 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DistanceUnits {
    /// `N` — nautical miles / knots.
    Nautical,
    /// `K` — kilometres / km·h⁻¹.
    Kilometers,
    /// `S` — statute miles / mph.
    Statute,
    /// Any byte not covered above; raw byte preserved.
    Other(u8),
}

impl DistanceUnits {
    pub(crate) fn from_byte(b: u8) -> Self {
        match b {
            b'N' | b'n' => Self::Nautical,
            b'K' | b'k' => Self::Kilometers,
            b'S' | b's' => Self::Statute,
            other => Self::Other(other),
        }
    }
}

/// How a target was acquired (TTM field 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcquisitionType {
    /// `A` — automatic.
    Automatic,
    /// `M` — manual.
    Manual,
    /// `R` — reported (from another source).
    Reported,
    /// Any byte not covered above; raw byte preserved.
    Other(u8),
}

impl AcquisitionType {
    pub(crate) fn from_byte(b: u8) -> Self {
        match b {
            b'A' | b'a' => Self::Automatic,
            b'M' | b'm' => Self::Manual,
            b'R' | b'r' => Self::Reported,
            other => Self::Other(other),
        }
    }
}

/// Decoded fields of a `$__TTM` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct TtmData {
    /// Two-byte talker ID (e.g. `Some(*b"RA")` for radar).
    pub talker: Option<[u8; 2]>,
    /// Target number (00–99 per spec; wider values tolerated).
    pub target_number: Option<u16>,
    /// Distance from own ship, in the unit given by [`units`](Self::units).
    pub distance: Option<f32>,
    /// Bearing to the target, degrees.
    pub bearing_deg: Option<f32>,
    /// Reference frame of [`bearing_deg`](Self::bearing_deg).
    pub bearing_reference: Option<AngleReference>,
    /// Target speed, in the unit given by [`units`](Self::units).
    pub speed: Option<f32>,
    /// Target course, degrees.
    pub course_deg: Option<f32>,
    /// Reference frame of [`course_deg`](Self::course_deg).
    pub course_reference: Option<AngleReference>,
    /// Distance at closest point of approach, in [`units`](Self::units).
    pub cpa: Option<f32>,
    /// Time to CPA in minutes; negative means range is increasing
    /// (target receding).
    pub tcpa: Option<f32>,
    /// Units governing [`distance`](Self::distance),
    /// [`speed`](Self::speed), and [`cpa`](Self::cpa).
    pub units: Option<DistanceUnits>,
    /// Target label.
    pub name: Option<String>,
    /// Tracking state.
    pub status: Option<TargetStatus>,
    /// `true` when this target is flagged (`R`) as the reference target.
    pub reference_target: bool,
    /// UTC time of the data (NMEA 3.0+). `None` if absent or empty.
    pub utc_time: Option<UtcTime>,
    /// How the target was acquired (NMEA 3.0+). `None` if absent/empty.
    pub acquisition: Option<AcquisitionType>,
}

/// Minimum fields: target number through target status (indices 0–11).
/// Reference target (12), UTC time (13), and acquisition (14) are
/// optional and read via `get`.
const TTM_MIN_FIELDS: usize = 12;

/// Decode a TTM sentence.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if fewer than 12 fields.
/// - [`DecodeError::InvalidNumber`] on a malformed numeric field.
/// - [`DecodeError::InvalidUtf8`] on a non-UTF-8 target name.
/// - [`DecodeError::InvalidUtcTime`] on a malformed non-empty UTC field.
#[allow(clippy::indexing_slicing)] // indices 0..12 validated; 12..15 via get
pub fn decode_ttm(raw: &RawSentence<'_>) -> Result<TtmData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < TTM_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: TTM_MIN_FIELDS,
            got: f.len(),
        });
    }

    let target_number = optional_u16(f[0], 0)?;
    let distance = optional_f32(f[1], 1)?;
    let bearing_deg = optional_f32(f[2], 2)?;
    let bearing_reference = f[3].first().copied().map(AngleReference::from_byte);
    let speed = optional_f32(f[4], 4)?;
    let course_deg = optional_f32(f[5], 5)?;
    let course_reference = f[6].first().copied().map(AngleReference::from_byte);
    let cpa = optional_f32(f[7], 7)?;
    let tcpa = optional_f32(f[8], 8)?;
    let units = f[9].first().copied().map(DistanceUnits::from_byte);
    let name = optional_string(f[10], 10)?;
    let status = f[11].first().copied().map(TargetStatus::from_byte);
    let reference_target = matches!(
        f.get(12).and_then(|b| b.first().copied()),
        Some(b'R' | b'r')
    );
    let utc_time = match f.get(13) {
        Some(bytes) => UtcTime::parse_optional(bytes, 13)?,
        None => None,
    };
    let acquisition = f
        .get(14)
        .and_then(|b| b.first().copied())
        .map(AcquisitionType::from_byte);

    Ok(TtmData {
        talker: raw.talker,
        target_number,
        distance,
        bearing_deg,
        bearing_reference,
        speed,
        course_deg,
        course_reference,
        cpa,
        tcpa,
        units,
        name,
        status,
        reference_target,
        utc_time,
        acquisition,
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

    // Full 15-field RATTM (radar talker), statute units, reported acquisition.
    #[test]
    fn decode_rattm_full() {
        let bytes = build(b"RATTM,12,1.23,45.6,T,7.8,90.1,R,2.5,-11.0,S,TGT1,T,R,123519.00,R");
        let raw = parse_raw(&bytes);
        let ttm = decode_ttm(&raw).expect("parse");
        assert_eq!(ttm.talker, Some(*b"RA"));
        assert_eq!(ttm.target_number, Some(12));
        assert!((ttm.distance.unwrap() - 1.23).abs() < 0.001);
        assert_eq!(ttm.bearing_reference, Some(AngleReference::True));
        assert_eq!(ttm.course_reference, Some(AngleReference::Relative));
        assert!((ttm.tcpa.unwrap() - -11.0).abs() < 0.001, "negative TCPA");
        assert_eq!(ttm.units, Some(DistanceUnits::Statute));
        assert_eq!(ttm.name.as_deref(), Some("TGT1"));
        assert_eq!(ttm.status, Some(TargetStatus::Tracking));
        assert!(ttm.reference_target);
        assert_eq!(
            ttm.utc_time,
            Some(UtcTime { hour: 12, minute: 35, second: 19, millisecond: 0 })
        );
        assert_eq!(ttm.acquisition, Some(AcquisitionType::Reported));
    }

    // Base 13-field TTM: no utc_time / acquisition.
    #[test]
    fn decode_ttm_base_13_fields_optional_trailing_none() {
        let bytes = build(b"RATTM,3,5.0,180.0,T,10.0,270.0,T,1.0,5.0,N,,Q,");
        let raw = parse_raw(&bytes);
        let ttm = decode_ttm(&raw).expect("parse");
        assert_eq!(ttm.target_number, Some(3));
        assert_eq!(ttm.units, Some(DistanceUnits::Nautical));
        assert_eq!(ttm.name, None);
        assert_eq!(ttm.status, Some(TargetStatus::Query));
        assert!(!ttm.reference_target);
        assert_eq!(ttm.utc_time, None);
        assert_eq!(ttm.acquisition, None);
    }

    #[test]
    fn decode_ttm_unknown_codes_preserved() {
        let bytes = build(b"RATTM,1,1.0,2.0,X,3.0,4.0,Y,5.0,6.0,Z,N1,W,,,B");
        let raw = parse_raw(&bytes);
        let ttm = decode_ttm(&raw).expect("parse");
        assert_eq!(ttm.bearing_reference, Some(AngleReference::Other(b'X')));
        assert_eq!(ttm.course_reference, Some(AngleReference::Other(b'Y')));
        assert_eq!(ttm.units, Some(DistanceUnits::Other(b'Z')));
        assert_eq!(ttm.status, Some(TargetStatus::Other(b'W')));
        assert_eq!(ttm.acquisition, Some(AcquisitionType::Other(b'B')));
    }

    #[test]
    fn decode_ttm_rejects_too_few_fields() {
        let bytes = build(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,name");
        let raw = parse_raw(&bytes);
        match decode_ttm(&raw) {
            Err(DecodeError::NotEnoughFields { expected: 12, got: 11 }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_ttm_rejects_malformed_number() {
        let bytes = build(b"RATTM,1,bad,2.0,T,3.0,4.0,T,5.0,6.0,N,name,T,");
        let raw = parse_raw(&bytes);
        match decode_ttm(&raw) {
            Err(DecodeError::InvalidNumber { field_index: 1 }) => {}
            other => panic!("expected InvalidNumber 1, got {other:?}"),
        }
    }
}
