//! VTG — Course Over Ground and Ground Speed.
//!
//! The sentence has a pre-NMEA-2.3 form with 8 fields (course/unit
//! pairs for true + magnetic, speed/unit pairs for knots + km/h) and a
//! NMEA-2.3+ form that adds a 9th field — the mode indicator. This
//! decoder accepts both: the mode is not available for pre-2.3
//! sentences *and* for sentences where the mode field is present but
//! empty.

use marlin_field::FieldState;
use marlin_nmea_envelope::RawSentence;

use crate::util::{code, number, optional};
use crate::Nmea0183DecodeError;

/// Decoded fields of a `$__VTG` sentence.
///
/// The talker is preserved rather than dispatched on —
/// `$GPVTG`, `$GNVTG`, `$INVTG` all decode to `VtgData` with different
/// [`talker`](Self::talker) values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VtgData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` would only
    /// appear for proprietary sentences — VTG is never proprietary —
    /// but the field is `Option` to match
    /// [`RawSentence::talker`]'s shape.
    pub talker: Option<[u8; 2]>,
    /// Course over ground, true (degrees).
    pub course_true_deg: FieldState<f32>,
    /// Course over ground, magnetic (degrees). Not available for
    /// receivers without a compass sensor.
    pub course_magnetic_deg: FieldState<f32>,
    /// Speed over ground in knots.
    pub speed_knots: FieldState<f32>,
    /// Speed over ground in kilometres per hour.
    pub speed_kmh: FieldState<f32>,
    /// Mode indicator (NMEA 2.3+). Not available if the sentence
    /// predates 2.3 (fewer than 9 fields) or if the field is present but
    /// empty.
    pub mode: FieldState<VtgMode>,
}

/// NMEA 2.3+ mode indicator — 9th field of VTG.
///
/// Describes how the reported fix was obtained. Safety-critical
/// consumers often check this field and reject `NotValid` or
/// `Estimated` modes before acting on the speed/course values. An
/// unnamed byte decodes to the invalid field state with the byte as its
/// raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VtgMode {
    /// `A` — Autonomous fix.
    Autonomous,
    /// `D` — Differential fix (DGPS).
    Differential,
    /// `E` — Estimated (dead-reckoning).
    Estimated,
    /// `N` — Data not valid.
    NotValid,
    /// `M` — Manual input.
    Manual,
    /// `S` — Simulator mode.
    Simulator,
}

impl VtgMode {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'A' | b'a' => Some(Self::Autonomous),
            b'D' | b'd' => Some(Self::Differential),
            b'E' | b'e' => Some(Self::Estimated),
            b'N' | b'n' => Some(Self::NotValid),
            b'M' | b'm' => Some(Self::Manual),
            b'S' | b's' => Some(Self::Simulator),
            _ => None,
        }
    }
}

/// VTG has at least 8 fields in pre-NMEA-2.3 form:
///
/// ```text
/// 0 : Course over ground, true     (degrees)
/// 1 : Unit indicator               (always `T`)
/// 2 : Course over ground, magnetic (degrees)
/// 3 : Unit indicator               (always `M`)
/// 4 : Speed over ground            (knots)
/// 5 : Unit indicator               (always `N`)
/// 6 : Speed over ground            (km/h)
/// 7 : Unit indicator               (always `K`)
/// 8 : Mode indicator (NMEA 2.3+)   (A/D/E/N/M/S)   — optional
/// ```
///
/// We don't validate the unit letters — they're redundant with the
/// struct field names. Only the numeric fields and the mode letter
/// are decoded.
const VTG_MIN_FIELDS: usize = 8;

/// Decode a VTG sentence into typed fields.
///
/// The caller is responsible for verifying `raw.sentence_type == "VTG"`
/// before calling this — the top-level [`decode`](crate::decode)
/// dispatcher does so.
///
/// # Errors
///
/// - [`Nmea0183DecodeError::NotEnoughFields`] if the payload has fewer than 8
///   fields.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_vtg(raw: &RawSentence<'_>) -> Result<VtgData, Nmea0183DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < VTG_MIN_FIELDS {
        return Err(Nmea0183DecodeError::NotEnoughFields {
            expected: VTG_MIN_FIELDS,
            got: f.len(),
        });
    }

    Ok(VtgData {
        talker: raw.talker,
        course_true_deg: number(f[0]),
        course_magnetic_deg: number(f[2]),
        speed_knots: number(f[4]),
        speed_kmh: number(f[6]),
        // Mode indicator (NMEA 2.3+) — may be missing entirely or empty.
        mode: code(optional(f, 8), VtgMode::from_byte),
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

    fn decode(body: &[u8]) -> VtgData {
        let bytes = build(body);
        let raw = parse_raw(&bytes);
        decode_vtg(&raw).expect("parse")
    }

    // -----------------------------------------------------------------
    // Happy path — NMEA 2.3+ full sentence
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_full_with_mode() {
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,A");

        assert_eq!(vtg.talker, Some(*b"GP"));
        assert!((vtg.course_true_deg.value().unwrap() - 54.7).abs() < 0.01);
        assert!((vtg.course_magnetic_deg.value().unwrap() - 34.4).abs() < 0.01);
        assert!((vtg.speed_knots.value().unwrap() - 5.5).abs() < 0.01);
        assert!((vtg.speed_kmh.value().unwrap() - 10.2).abs() < 0.01);
        assert_eq!(vtg.mode, FieldState::Value(VtgMode::Autonomous));
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form (no mode indicator field)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_pre_nmea_2_3_mode_is_not_available() {
        // Exactly 8 fields — no 9th mode indicator at all.
        let bytes = build(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 8, "fixture sanity: pre-2.3 has 8 fields");

        let vtg = decode_vtg(&raw).expect("parse");
        assert_eq!(vtg.mode, FieldState::NotAvailable);
        assert!((vtg.speed_knots.value().unwrap() - 5.5).abs() < 0.01);
    }

    #[test]
    fn decode_vtg_empty_mode_field_is_not_available() {
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,");
        assert_eq!(vtg.mode, FieldState::NotAvailable);
    }

    // -----------------------------------------------------------------
    // All numeric fields empty (no fix / no compass)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_all_empty_numeric_fields_are_not_available() {
        let vtg = decode(b"GPVTG,,T,,M,,N,,K,N");
        assert_eq!(vtg.course_true_deg, FieldState::NotAvailable);
        assert_eq!(vtg.course_magnetic_deg, FieldState::NotAvailable);
        assert_eq!(vtg.speed_knots, FieldState::NotAvailable);
        assert_eq!(vtg.speed_kmh, FieldState::NotAvailable);
        assert_eq!(vtg.mode, FieldState::Value(VtgMode::NotValid));
    }

    #[test]
    fn decode_vtg_every_unreadable_field_is_invalid() {
        let vtg = decode(b"GPVTG,x,T,x,M,x,N,x,K,AD");
        assert_eq!(vtg.course_true_deg, unparsable());
        assert_eq!(vtg.course_magnetic_deg, unparsable());
        assert_eq!(vtg.speed_knots, unparsable());
        assert_eq!(vtg.speed_kmh, unparsable());
        assert_eq!(vtg.mode, unparsable());
    }

    #[test]
    fn decode_vtg_receiver_without_compass_has_no_magnetic_course() {
        let vtg = decode(b"GPVTG,054.7,T,,M,005.5,N,010.2,K,A");
        assert!(vtg.course_true_deg.value().is_some());
        assert_eq!(vtg.course_magnetic_deg, FieldState::NotAvailable);
        assert!(vtg.speed_knots.value().is_some());
    }

    // -----------------------------------------------------------------
    // Mode indicator — every recognized variant, through the decoder
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_covers_every_recognized_mode() {
        for (letter, expected) in [
            (b"A", VtgMode::Autonomous),
            (b"D", VtgMode::Differential),
            (b"E", VtgMode::Estimated),
            (b"N", VtgMode::NotValid),
            (b"M", VtgMode::Manual),
            (b"S", VtgMode::Simulator),
        ] {
            let mut body = b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,".to_vec();
            body.extend_from_slice(letter);
            assert_eq!(decode(&body).mode, FieldState::Value(expected));
        }
    }

    #[test]
    fn decode_vtg_mode_is_case_insensitive() {
        // Most NMEA sentences use uppercase, but case-insensitivity is
        // a sane default — match what the envelope does for hex digits.
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,d");
        assert_eq!(vtg.mode, FieldState::Value(VtgMode::Differential));
    }

    #[test]
    fn decode_vtg_unnamed_mode_letter_is_invalid_with_the_byte() {
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,X");
        assert_eq!(
            vtg.mode,
            FieldState::Invalid(Invalid::Undefined(RawCode(i64::from(b'X'))))
        );
    }

    #[test]
    fn decode_vtg_two_byte_mode_is_invalid_without_a_code() {
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,AD");
        assert_eq!(vtg.mode, FieldState::Invalid(Invalid::Unparsable));
    }

    // -----------------------------------------------------------------
    // Error: too few fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_rejects_too_few_fields() {
        let bytes = build(b"GPVTG,054.7,T,034.4"); // only 3 fields
        let raw = parse_raw(&bytes);
        match decode_vtg(&raw) {
            Err(Nmea0183DecodeError::NotEnoughFields {
                expected: 8,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // An unparsable number is an invalid field, not a sentence failure
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_unparsable_speed_is_invalid_and_the_rest_decodes() {
        let vtg = decode(b"GPVTG,054.7,T,034.4,M,not-a-number,N,010.2,K,A");
        assert_eq!(vtg.speed_knots, FieldState::Invalid(Invalid::Unparsable));
        assert!((vtg.speed_kmh.value().unwrap() - 10.2).abs() < 0.01);
        assert_eq!(vtg.mode, FieldState::Value(VtgMode::Autonomous));
    }
}
