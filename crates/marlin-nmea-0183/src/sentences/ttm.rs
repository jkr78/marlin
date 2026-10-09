//! TTM — Tracked Target Message (radar/ARPA).
//!
//! `$--TTM,xx,x.x,x.x,a,x.x,x.x,a,x.x,x.x,a,c--c,a,a,hhmmss.ss,a*hh`.
//! 15 data fields; the trailing `utc_time` (13) and `acquisition` (14)
//! were added in NMEA 3.0 and are optional. `$RATTM` (radar talker) is
//! this exact layout — the `RA` talker surfaces in [`TtmData::talker`].

use alloc::string::String;

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::sentences::status::TargetStatus;
use crate::sentences::utc_time::UtcTime;
use crate::util::{code, number, optional, reference_target, text};
use crate::Nmea0183DecodeError;

/// Bearing/course reference, shared by TTM fields 3 and 6. An unnamed
/// byte decodes to the invalid field state with the byte as its raw
/// code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AngleReference {
    /// `T` — referenced to true north.
    True,
    /// `R` — relative to own ship's heading.
    Relative,
}

impl AngleReference {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'T' | b't' => Some(Self::True),
            b'R' | b'r' => Some(Self::Relative),
            _ => None,
        }
    }
}

/// Speed & distance units governing TTM fields 1, 4, 7. An unnamed
/// byte decodes to the invalid field state with the byte as its raw
/// code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DistanceUnits {
    /// `N` — nautical miles / knots.
    Nautical,
    /// `K` — kilometres / km·h⁻¹.
    Kilometers,
    /// `S` — statute miles / mph.
    Statute,
}

impl DistanceUnits {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'N' | b'n' => Some(Self::Nautical),
            b'K' | b'k' => Some(Self::Kilometers),
            b'S' | b's' => Some(Self::Statute),
            _ => None,
        }
    }
}

/// How a target was acquired (TTM field 14). An unnamed byte decodes
/// to the invalid field state with the byte as its raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcquisitionType {
    /// `A` — automatic.
    Automatic,
    /// `M` — manual.
    Manual,
    /// `R` — reported (from another source).
    Reported,
}

impl AcquisitionType {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'A' | b'a' => Some(Self::Automatic),
            b'M' | b'm' => Some(Self::Manual),
            b'R' | b'r' => Some(Self::Reported),
            _ => None,
        }
    }
}

/// Decoded fields of a `$__TTM` sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct TtmData {
    /// Two-byte talker ID (e.g. `Some(*b"RA")` for radar).
    pub talker: Option<[u8; 2]>,
    /// Target number (00–99 per spec; wider values tolerated).
    pub target_number: FieldState<u16>,
    /// Distance from own ship, in the unit given by [`units`](Self::units).
    pub distance: FieldState<f32>,
    /// Bearing to the target, degrees.
    pub bearing_deg: FieldState<f32>,
    /// Reference frame of [`bearing_deg`](Self::bearing_deg).
    pub bearing_reference: FieldState<AngleReference>,
    /// Target speed, in the unit given by [`units`](Self::units).
    pub speed: FieldState<f32>,
    /// Target course, degrees.
    pub course_deg: FieldState<f32>,
    /// Reference frame of [`course_deg`](Self::course_deg).
    pub course_reference: FieldState<AngleReference>,
    /// Distance at closest point of approach, in [`units`](Self::units).
    pub cpa: FieldState<f32>,
    /// Time to CPA in minutes; negative means range is increasing
    /// (target receding).
    pub tcpa: FieldState<f32>,
    /// Units governing [`distance`](Self::distance),
    /// [`speed`](Self::speed), and [`cpa`](Self::cpa).
    pub units: FieldState<DistanceUnits>,
    /// Target label.
    pub name: FieldState<String>,
    /// Tracking state.
    pub status: FieldState<TargetStatus>,
    /// `true` when this target is flagged (`R`) as the reference target.
    /// An empty or absent field is `Value(false)`, the standard's
    /// encoding of "not the reference target"; never not available.
    pub reference_target: FieldState<bool>,
    /// UTC time of the data (NMEA 3.0+). Not available if absent or
    /// empty.
    pub utc_time: FieldState<UtcTime>,
    /// How the target was acquired (NMEA 3.0+). Not available if absent
    /// or empty.
    pub acquisition: FieldState<AcquisitionType>,
}

/// Minimum fields: target number through target status (indices 0–11).
/// Reference target (12), UTC time (13), and acquisition (14) are
/// optional and read via `get`.
const TTM_MIN_FIELDS: usize = 12;

/// Decode a TTM sentence.
///
/// # Errors
///
/// - [`Nmea0183DecodeError::NotEnoughFields`] if fewer than 12 fields.
#[allow(clippy::indexing_slicing)] // indices 0..12 validated; 12..15 via get
pub fn decode_ttm(raw: &RawSentence<'_>) -> Result<TtmData, Nmea0183DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < TTM_MIN_FIELDS {
        return Err(Nmea0183DecodeError::NotEnoughFields {
            expected: TTM_MIN_FIELDS,
            got: f.len(),
        });
    }
    Ok(TtmData {
        talker: raw.talker,
        target_number: number(f[0]),
        distance: number(f[1]),
        bearing_deg: number(f[2]),
        bearing_reference: code(f[3], AngleReference::from_byte),
        speed: number(f[4]),
        course_deg: number(f[5]),
        course_reference: code(f[6], AngleReference::from_byte),
        cpa: number(f[7]),
        tcpa: number(f[8]),
        units: code(f[9], DistanceUnits::from_byte),
        name: text(f[10]),
        status: code(f[11], TargetStatus::from_byte),
        reference_target: reference_target(optional(f, 12)),
        utc_time: number(optional(f, 13)),
        acquisition: code(optional(f, 14), AcquisitionType::from_byte),
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

    fn decode(body: &[u8]) -> TtmData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_ttm(&raw).expect("parse")
    }

    // Full 15-field RATTM (radar talker), statute units, reported acquisition.
    #[test]
    fn decode_rattm_full() {
        let ttm = decode(b"RATTM,12,1.23,45.6,T,7.8,90.1,R,2.5,-11.0,S,TGT1,T,R,123519.00,R");
        assert_eq!(ttm.talker, Some(*b"RA"));
        assert_eq!(ttm.target_number, FieldState::Value(12));
        assert!((ttm.distance.value().unwrap() - 1.23).abs() < 0.001);
        assert!((ttm.bearing_deg.value().unwrap() - 45.6).abs() < 0.01);
        assert!((ttm.speed.value().unwrap() - 7.8).abs() < 0.01);
        assert!((ttm.course_deg.value().unwrap() - 90.1).abs() < 0.01);
        assert!((ttm.cpa.value().unwrap() - 2.5).abs() < 0.01);
        assert_eq!(
            ttm.bearing_reference,
            FieldState::Value(AngleReference::True)
        );
        assert_eq!(
            ttm.course_reference,
            FieldState::Value(AngleReference::Relative)
        );
        assert!(
            (ttm.tcpa.value().unwrap() - -11.0).abs() < 0.001,
            "negative TCPA"
        );
        assert_eq!(ttm.units, FieldState::Value(DistanceUnits::Statute));
        assert_eq!(ttm.name, FieldState::Value("TGT1".to_string()));
        assert_eq!(ttm.status, FieldState::Value(TargetStatus::Tracking));
        assert_eq!(ttm.reference_target, FieldState::Value(true));
        assert_eq!(
            ttm.utc_time,
            FieldState::Value(UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            })
        );
        assert_eq!(
            ttm.acquisition,
            FieldState::Value(AcquisitionType::Reported)
        );
    }

    // Base 13-field TTM: no utc_time / acquisition.
    #[test]
    fn decode_ttm_base_13_fields_optional_trailing_not_available() {
        let ttm = decode(b"RATTM,3,5.0,180.0,T,10.0,270.0,T,1.0,5.0,N,,Q,");
        assert_eq!(ttm.target_number, FieldState::Value(3));
        assert_eq!(ttm.units, FieldState::Value(DistanceUnits::Nautical));
        assert_eq!(ttm.name, FieldState::NotAvailable);
        assert_eq!(ttm.status, FieldState::Value(TargetStatus::Query));
        assert_eq!(ttm.reference_target, FieldState::Value(false));
        assert_eq!(ttm.utc_time, FieldState::NotAvailable);
        assert_eq!(ttm.acquisition, FieldState::NotAvailable);
    }

    #[test]
    fn decode_ttm_every_empty_field_is_not_available() {
        let ttm = decode(b"RATTM,,,,,,,,,,,,,,,");
        assert_eq!(ttm.target_number, FieldState::NotAvailable);
        assert_eq!(ttm.distance, FieldState::NotAvailable);
        assert_eq!(ttm.bearing_deg, FieldState::NotAvailable);
        assert_eq!(ttm.bearing_reference, FieldState::NotAvailable);
        assert_eq!(ttm.speed, FieldState::NotAvailable);
        assert_eq!(ttm.course_deg, FieldState::NotAvailable);
        assert_eq!(ttm.course_reference, FieldState::NotAvailable);
        assert_eq!(ttm.cpa, FieldState::NotAvailable);
        assert_eq!(ttm.tcpa, FieldState::NotAvailable);
        assert_eq!(ttm.units, FieldState::NotAvailable);
        assert_eq!(ttm.name, FieldState::NotAvailable);
        assert_eq!(ttm.status, FieldState::NotAvailable);
        assert_eq!(ttm.reference_target, FieldState::Value(false));
        assert_eq!(ttm.utc_time, FieldState::NotAvailable);
        assert_eq!(ttm.acquisition, FieldState::NotAvailable);
    }

    #[test]
    fn decode_ttm_every_unreadable_field_is_invalid() {
        let ttm = decode(b"RATTM,x,x,x,TR,x,x,TR,x,x,NK,\xFF,LQ,RR,x,AM");
        assert_eq!(ttm.target_number, unparsable());
        assert_eq!(ttm.distance, unparsable());
        assert_eq!(ttm.bearing_deg, unparsable());
        assert_eq!(ttm.bearing_reference, unparsable());
        assert_eq!(ttm.speed, unparsable());
        assert_eq!(ttm.course_deg, unparsable());
        assert_eq!(ttm.course_reference, unparsable());
        assert_eq!(ttm.cpa, unparsable());
        assert_eq!(ttm.tcpa, unparsable());
        assert_eq!(ttm.units, unparsable());
        assert_eq!(ttm.name, unparsable());
        assert_eq!(ttm.status, unparsable());
        assert_eq!(ttm.reference_target, unparsable());
        assert_eq!(ttm.utc_time, unparsable());
        assert_eq!(ttm.acquisition, unparsable());
    }

    #[test]
    fn decode_ttm_every_unit_and_acquisition_letter() {
        for (letter, expected) in [
            (b"N", DistanceUnits::Nautical),
            (b"K", DistanceUnits::Kilometers),
            (b"S", DistanceUnits::Statute),
            (b"k", DistanceUnits::Kilometers),
        ] {
            let mut body = b"RATTM,3,5.0,180.0,T,10.0,270.0,T,1.0,5.0,".to_vec();
            body.extend_from_slice(letter);
            body.extend_from_slice(b",,Q,");
            assert_eq!(decode(&body).units, FieldState::Value(expected));
        }
        for (letter, expected) in [
            (b"A", AcquisitionType::Automatic),
            (b"M", AcquisitionType::Manual),
            (b"R", AcquisitionType::Reported),
            (b"m", AcquisitionType::Manual),
        ] {
            let mut body = b"RATTM,3,5.0,180.0,T,10.0,270.0,T,1.0,5.0,N,,Q,,,".to_vec();
            body.extend_from_slice(letter);
            assert_eq!(decode(&body).acquisition, FieldState::Value(expected));
        }
    }

    #[test]
    fn decode_ttm_unnamed_codes_are_invalid_with_the_byte() {
        fn undefined<T>(byte: u8) -> FieldState<T> {
            FieldState::Invalid(Invalid::Undefined(RawCode(byte.into())))
        }
        let ttm = decode(b"RATTM,1,1.0,2.0,X,3.0,4.0,Y,5.0,6.0,Z,N1,W,,,B");
        assert_eq!(ttm.bearing_reference, undefined(b'X'));
        assert_eq!(ttm.course_reference, undefined(b'Y'));
        assert_eq!(ttm.units, undefined(b'Z'));
        assert_eq!(ttm.status, undefined(b'W'));
        assert_eq!(ttm.acquisition, undefined(b'B'));
    }

    #[test]
    fn decode_ttm_two_byte_codes_are_invalid_without_a_code() {
        let ttm = decode(b"RATTM,1,1.0,2.0,TR,3.0,4.0,T,5.0,6.0,NK,N1,LQ,RR,,AM");
        assert_eq!(
            ttm.bearing_reference,
            FieldState::Invalid(Invalid::Unparsable)
        );
        assert_eq!(ttm.units, FieldState::Invalid(Invalid::Unparsable));
        assert_eq!(ttm.status, FieldState::Invalid(Invalid::Unparsable));
        assert_eq!(
            ttm.reference_target,
            FieldState::Invalid(Invalid::Unparsable)
        );
        assert_eq!(ttm.acquisition, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_ttm_empty_codes_are_not_available() {
        let ttm = decode(b"RATTM,1,1.0,2.0,,3.0,4.0,,5.0,6.0,,,,,,");
        assert_eq!(ttm.bearing_reference, FieldState::NotAvailable);
        assert_eq!(ttm.course_reference, FieldState::NotAvailable);
        assert_eq!(ttm.units, FieldState::NotAvailable);
        assert_eq!(ttm.status, FieldState::NotAvailable);
        assert_eq!(ttm.acquisition, FieldState::NotAvailable);
        assert_eq!(ttm.reference_target, FieldState::Value(false));
    }

    #[test]
    fn decode_ttm_non_utf8_name_is_invalid() {
        let ttm = decode(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,\xFF,T,");
        assert_eq!(ttm.name, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_ttm_unnamed_reference_flag_is_invalid_with_the_byte() {
        let ttm = decode(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,n,T,X");
        assert_eq!(
            ttm.reference_target,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_ttm_unparsable_utc_is_invalid() {
        let ttm = decode(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,n,T,,12,A");
        assert_eq!(ttm.utc_time, FieldState::Invalid(Invalid::Unparsable));
    }

    #[test]
    fn decode_ttm_rejects_too_few_fields() {
        let bytes = build(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,name");
        let raw = parse_raw(&bytes);
        match decode_ttm(&raw) {
            Err(Nmea0183DecodeError::NotEnoughFields {
                expected: 12,
                got: 11,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_ttm_unparsable_number_is_invalid_and_the_rest_decodes() {
        let ttm = decode(b"RATTM,1,bad,2.0,T,3.0,4.0,T,5.0,6.0,N,name,T,");
        assert_eq!(ttm.distance, FieldState::Invalid(Invalid::Unparsable));
        assert!((ttm.bearing_deg.value().unwrap() - 2.0).abs() < 0.001);
        assert_eq!(ttm.status, FieldState::Value(TargetStatus::Tracking));
    }
}
