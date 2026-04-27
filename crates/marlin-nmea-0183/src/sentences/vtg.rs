//! VTG — Course Over Ground and Ground Speed.
//!
//! The sentence has a pre-NMEA-2.3 form with 8 fields (course/unit
//! pairs for true + magnetic, speed/unit pairs for knots + km/h) and a
//! NMEA-2.3+ form that adds a 9th field — the mode indicator. This
//! decoder accepts both: the mode is an [`Option<VtgMode>`] that's
//! `None` for pre-2.3 sentences *and* for sentences where the mode
//! field is present but empty.

use marlin_nmea_envelope::RawSentence;

use crate::util::optional_f32;
use crate::DecodeError;

/// Decoded fields of a `$__VTG` sentence.
///
/// Per PRD §D6, the talker ID is preserved rather than dispatched on —
/// `$GPVTG`, `$GNVTG`, `$INVTG` all decode to `VtgData` with different
/// [`talker`](Self::talker) values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VtgData {
    /// Two-byte talker ID (e.g. `Some(*b"GP")`). `None` would only
    /// appear for proprietary sentences — VTG is never proprietary —
    /// but the field is `Option` to match
    /// [`RawSentence::talker`]'s shape.
    pub talker: Option<[u8; 2]>,
    /// Course over ground, true (degrees). `None` for empty field.
    pub course_true_deg: Option<f32>,
    /// Course over ground, magnetic (degrees). `None` for empty field
    /// or for receivers without a compass sensor.
    pub course_magnetic_deg: Option<f32>,
    /// Speed over ground in knots. `None` for empty field.
    pub speed_knots: Option<f32>,
    /// Speed over ground in kilometres per hour. `None` for empty field.
    pub speed_kmh: Option<f32>,
    /// Mode indicator (NMEA 2.3+). `None` if the sentence predates 2.3
    /// (fewer than 9 fields) or if the field is present but empty.
    pub mode: Option<VtgMode>,
}

/// NMEA 2.3+ mode indicator — 9th field of VTG.
///
/// Describes how the reported fix was obtained. Safety-critical
/// consumers often check this field and reject `NotValid` or
/// `Estimated` modes before acting on the speed/course values.
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
    /// Any letter not covered above; the raw byte is preserved so
    /// downstream consumers can decide whether to warn or pass.
    Other(u8),
}

impl VtgMode {
    fn from_byte(b: u8) -> Self {
        match b {
            b'A' | b'a' => Self::Autonomous,
            b'D' | b'd' => Self::Differential,
            b'E' | b'e' => Self::Estimated,
            b'N' | b'n' => Self::NotValid,
            b'M' | b'm' => Self::Manual,
            b'S' | b's' => Self::Simulator,
            other => Self::Other(other),
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
/// - [`DecodeError::NotEnoughFields`] if the payload has fewer than 8
///   fields.
/// - [`DecodeError::InvalidNumber`] or [`DecodeError::InvalidUtf8`] if
///   a numeric field is non-empty and malformed.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_vtg(raw: &RawSentence<'_>) -> Result<VtgData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < VTG_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: VTG_MIN_FIELDS,
            got: f.len(),
        });
    }

    let course_true_deg = optional_f32(f[0], 0)?;
    let course_magnetic_deg = optional_f32(f[2], 2)?;
    let speed_knots = optional_f32(f[4], 4)?;
    let speed_kmh = optional_f32(f[6], 6)?;

    // Mode indicator (NMEA 2.3+) — may be missing entirely or empty.
    let mode = f
        .get(8)
        .filter(|bytes| !bytes.is_empty())
        .and_then(|bytes| bytes.first().copied())
        .map(VtgMode::from_byte);

    Ok(VtgData {
        talker: raw.talker,
        course_true_deg,
        course_magnetic_deg,
        speed_knots,
        speed_kmh,
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
    fn decode_vtg_full_with_mode() {
        let bytes = build(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,A");
        let raw = parse_raw(&bytes);
        let vtg = decode_vtg(&raw).expect("parse");

        assert_eq!(vtg.talker, Some(*b"GP"));
        assert!((vtg.course_true_deg.unwrap() - 54.7).abs() < 0.01);
        assert!((vtg.course_magnetic_deg.unwrap() - 34.4).abs() < 0.01);
        assert!((vtg.speed_knots.unwrap() - 5.5).abs() < 0.01);
        assert!((vtg.speed_kmh.unwrap() - 10.2).abs() < 0.01);
        assert_eq!(vtg.mode, Some(VtgMode::Autonomous));
    }

    // -----------------------------------------------------------------
    // Pre-NMEA-2.3 form (no mode indicator field)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_pre_nmea_2_3_returns_none_mode() {
        // Exactly 8 fields — no 9th mode indicator at all.
        let bytes = build(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.fields.len(), 8, "fixture sanity: pre-2.3 has 8 fields");

        let vtg = decode_vtg(&raw).expect("parse");
        assert_eq!(vtg.mode, None);
        assert!((vtg.speed_knots.unwrap() - 5.5).abs() < 0.01);
    }

    // -----------------------------------------------------------------
    // Empty mode field → mode: None (distinct from pre-2.3)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_empty_mode_field_is_none() {
        let bytes = build(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,");
        let raw = parse_raw(&bytes);
        let vtg = decode_vtg(&raw).expect("parse");
        assert_eq!(vtg.mode, None);
    }

    // -----------------------------------------------------------------
    // All numeric fields empty (no fix / no compass)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_all_empty_numeric_fields_decode_to_none() {
        let bytes = build(b"GPVTG,,T,,M,,N,,K,N");
        let raw = parse_raw(&bytes);
        let vtg = decode_vtg(&raw).expect("parse");
        assert_eq!(vtg.course_true_deg, None);
        assert_eq!(vtg.course_magnetic_deg, None);
        assert_eq!(vtg.speed_knots, None);
        assert_eq!(vtg.speed_kmh, None);
        assert_eq!(vtg.mode, Some(VtgMode::NotValid));
    }

    // -----------------------------------------------------------------
    // Missing compass — course_magnetic empty, others present
    // (common on GPS-only receivers)
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_receiver_without_compass_has_no_magnetic_course() {
        let bytes = build(b"GPVTG,054.7,T,,M,005.5,N,010.2,K,A");
        let raw = parse_raw(&bytes);
        let vtg = decode_vtg(&raw).expect("parse");
        assert!(vtg.course_true_deg.is_some());
        assert_eq!(vtg.course_magnetic_deg, None);
        assert!(vtg.speed_knots.is_some());
    }

    // -----------------------------------------------------------------
    // Mode indicator — every recognized variant
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_covers_every_recognized_mode() {
        for (byte, expected) in [
            (b'A', VtgMode::Autonomous),
            (b'D', VtgMode::Differential),
            (b'E', VtgMode::Estimated),
            (b'N', VtgMode::NotValid),
            (b'M', VtgMode::Manual),
            (b'S', VtgMode::Simulator),
            (b'X', VtgMode::Other(b'X')),
        ] {
            assert_eq!(VtgMode::from_byte(byte), expected);
        }
    }

    #[test]
    fn decode_vtg_mode_is_case_insensitive() {
        // Most NMEA sentences use uppercase, but case-insensitivity is
        // a sane default — match what the envelope does for hex digits.
        assert_eq!(VtgMode::from_byte(b'a'), VtgMode::Autonomous);
        assert_eq!(VtgMode::from_byte(b'd'), VtgMode::Differential);
    }

    // -----------------------------------------------------------------
    // Error: too few fields
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_rejects_too_few_fields() {
        let bytes = build(b"GPVTG,054.7,T,034.4"); // only 3 fields
        let raw = parse_raw(&bytes);
        match decode_vtg(&raw) {
            Err(DecodeError::NotEnoughFields {
                expected: 8,
                got: 3,
            }) => {}
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Error: invalid number in a numeric field
    // -----------------------------------------------------------------

    #[test]
    fn decode_vtg_rejects_malformed_speed() {
        let bytes = build(b"GPVTG,054.7,T,034.4,M,not-a-number,N,010.2,K,A");
        let raw = parse_raw(&bytes);
        match decode_vtg(&raw) {
            Err(DecodeError::InvalidNumber { field_index: 4 }) => {}
            other => panic!("expected InvalidNumber field 4, got {other:?}"),
        }
    }
}
