//! PSXN — Kongsberg-family proprietary motion sentence.
//!
//! # Wire format
//!
//! ```text
//! $PSXN,<id>,<token>,<data0>,<data1>,<data2>,<data3>,<data4>,<data5>*hh
//! ```
//!
//! The six `dataN` slots carry scalar numeric values whose **meaning is
//! determined by install-time sensor configuration**, not by the
//! sentence itself. Each slot may carry roll, pitch, heave, a sine-
//! encoded variant of roll/pitch, or nothing — selected per-install by
//! the motion-sensor operator.
//!
//! Callers configure this via a [`PsxnLayout`] and pass it to
//! [`decode_psxn`]. The output is always the same [`PsxnData`] struct,
//! so caller code doesn't branch on layout.
//!
//! # `id` and `token`
//!
//! - `id` is an integer the sensor emits for application-layer
//!   interpretation (e.g. some systems use specific values to signal
//!   quality presets). This crate stores it as-is; downstream logic
//!   decides what to do with it.
//! - `token` is an opaque pass-through field. Owned bytes are returned
//!   so callers can hold them after the [`RawSentence`] is dropped.
//!
//! # Sine-encoded slots (legacy "Hippy")
//!
//! Some motion sensors don't emit Euler angles; they emit components
//! of the gravity vector projected onto vessel axes. The
//! [`PsxnSlot::RollSineEncoded`] and [`PsxnSlot::PitchSineEncoded`]
//! variants cover these encodings. The decoder recovers angles via
//! inverse trig; see each variant's docs for the exact formula.
//!
//! # Legacy config string compatibility
//!
//! Legacy Python pipelines use a short string like `"rphx"` or
//! `"sqhx1"` to describe the layout. [`PsxnLayout`] implements
//! [`core::str::FromStr`] so those strings parse directly — useful for
//! config files carrying pre-existing operator-set layouts.

use alloc::vec::Vec;
use core::str::FromStr;

use marlin_nmea_envelope::RawSentence;

use crate::util::{non_empty, optional_f32, optional_u16};
use crate::DecodeError;

// ---------------------------------------------------------------------------
// Output struct
// ---------------------------------------------------------------------------

/// Decoded fields of a `$PSXN` sentence.
///
/// Always the same shape regardless of which [`PsxnLayout`] was used to
/// decode. Any slot not carrying a given quantity (or carrying an empty
/// field on the wire) yields `None` for that quantity.
#[derive(Debug, Clone, PartialEq)]
pub struct PsxnData {
    /// `id` field from the wire (application-specific meaning).
    pub id: Option<u16>,
    /// `token` field from the wire — opaque pass-through. Preserved as
    /// owned bytes so callers can keep it after the [`RawSentence`] is
    /// dropped.
    pub token: Option<Vec<u8>>,
    /// Roll in degrees (or radians if [`PsxnLayout::raw_radians`] is set).
    pub roll_deg: Option<f32>,
    /// Pitch in degrees (or radians if [`PsxnLayout::raw_radians`] is set).
    pub pitch_deg: Option<f32>,
    /// Heave displacement in metres (positive = up by convention;
    /// verify against your sensor ICD).
    pub heave_m: Option<f32>,
}

// ---------------------------------------------------------------------------
// Slot / Layout
// ---------------------------------------------------------------------------

/// Meaning of one of the six `dataN` slots in a PSXN sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PsxnSlot {
    /// Slot carries roll directly. Interpreted as radians unless
    /// [`PsxnLayout::raw_radians`] is `true`, in which case the raw
    /// value is passed through unchanged.
    Roll,
    /// Slot carries pitch directly. Same unit convention as [`Self::Roll`].
    Pitch,
    /// Slot carries heave displacement in metres.
    Heave,
    /// Slot carries the sine-encoded roll component
    /// `sin(roll) * cos(pitch)`.
    ///
    /// The decoder recovers roll via `asin(value / cos(pitch))`. This
    /// requires the layout to **also** contain a source of pitch
    /// ([`Self::Pitch`] or [`Self::PitchSineEncoded`]); if no pitch is
    /// available, or if `cos(pitch)` is too close to zero (gimbal
    /// lock), `roll_deg` resolves to `None`.
    ///
    /// Legacy Python called this `rollHippy`. The encoding is common
    /// on TSS-family hydrographic sensors.
    RollSineEncoded,
    /// Slot carries the negated sine of pitch: `-sin(pitch)`.
    ///
    /// The decoder recovers pitch via `asin(-value)`. Legacy Python
    /// called this `pitchHippy`.
    PitchSineEncoded,
    /// Slot is present on the wire but has no defined meaning — read
    /// and discarded.
    Ignored,
}

/// Configuration for how PSXN `dataN` slots map to motion quantities.
///
/// One layout applies for the lifetime of a sensor install; all PSXN
/// sentences from that sensor are decoded with the same layout. In a
/// multi-sensor setup, callers can use [`decode_psxn`] directly with a
/// per-sensor layout instead of the top-level dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsxnLayout {
    /// Per-slot meaning. `slots[0]` corresponds to wire field `data0`
    /// (overall sentence field index 2).
    pub slots: [PsxnSlot; 6],
    /// `true` — the sensor emits angles already in the final unit
    /// (typically radians); decoder leaves values unchanged.
    /// `false` (default) — angles are in radians on the wire and the
    /// decoder converts them to degrees. Mirrors the legacy `1` flag.
    pub raw_radians: bool,
}

impl Default for PsxnLayout {
    /// `rphx` with degree conversion: roll / pitch / heave / ignored ×
    /// 3. Matches the legacy default.
    fn default() -> Self {
        Self {
            slots: [
                PsxnSlot::Roll,
                PsxnSlot::Pitch,
                PsxnSlot::Heave,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
            ],
            raw_radians: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy layout-string parser
// ---------------------------------------------------------------------------

/// Error from parsing a legacy PSXN layout string like `"rphx1"`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PsxnLayoutParseError {
    /// Unrecognised character in the layout string. Valid characters
    /// are `r`, `p`, `h`, `s`, `q`, `x`, `1` (case-insensitive).
    #[error("unknown PSXN layout character '{0}'")]
    UnknownChar(char),
    /// The string described more than 6 data slots.
    #[error("PSXN layout string has more than 6 data slots")]
    TooManySlots,
}

impl FromStr for PsxnLayout {
    type Err = PsxnLayoutParseError;

    /// Parse a legacy layout string.
    ///
    /// Recognised characters (case-insensitive):
    ///
    /// | Char | Slot meaning |
    /// | --- | --- |
    /// | `r` | [`PsxnSlot::Roll`] |
    /// | `p` | [`PsxnSlot::Pitch`] |
    /// | `h` | [`PsxnSlot::Heave`] |
    /// | `s` | [`PsxnSlot::RollSineEncoded`] (legacy "rollHippy") |
    /// | `q` | [`PsxnSlot::PitchSineEncoded`] (legacy "pitchHippy") |
    /// | `x` | [`PsxnSlot::Ignored`] |
    /// | `1` | Flag: set [`PsxnLayout::raw_radians`] to `true` |
    ///
    /// The `1` flag may appear anywhere in the string and does not
    /// consume a slot. Any slot positions not populated by the string
    /// default to [`PsxnSlot::Ignored`].
    #[allow(clippy::indexing_slicing)] // idx is explicitly bounded by `idx >= 6` check
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut slots = [PsxnSlot::Ignored; 6];
        let mut raw_radians = false;
        let mut idx = 0usize;
        for ch in s.chars() {
            if ch == '1' {
                raw_radians = true;
                continue;
            }
            if idx >= 6 {
                return Err(PsxnLayoutParseError::TooManySlots);
            }
            let slot = match ch {
                'r' | 'R' => PsxnSlot::Roll,
                'p' | 'P' => PsxnSlot::Pitch,
                'h' | 'H' => PsxnSlot::Heave,
                's' | 'S' => PsxnSlot::RollSineEncoded,
                'q' | 'Q' => PsxnSlot::PitchSineEncoded,
                'x' | 'X' => PsxnSlot::Ignored,
                other => return Err(PsxnLayoutParseError::UnknownChar(other)),
            };
            slots[idx] = slot;
            idx = idx.saturating_add(1);
        }
        Ok(Self { slots, raw_radians })
    }
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// Minimum fields the wire format carries: `id, token, data0..data5`.
const PSXN_MIN_FIELDS: usize = 8;

/// Epsilon below which `cos(pitch)` is treated as zero (gimbal lock).
/// At `|pitch| > ~89.99°` we can't recover roll from a sine-encoded
/// slot. Matches the sign convention of [`libm::cosf`].
const COS_PITCH_MIN: f32 = 1e-6;

/// Decode a `$PSXN` sentence using the provided layout.
///
/// The layout tells the decoder which quantity each `dataN` slot
/// carries; see [`PsxnLayout`] for the configuration options and
/// [`PsxnData`] for the output shape.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if fewer than 8 fields are
///   present (the wire format always has `id, token, data0..data5`).
/// - [`DecodeError::InvalidNumber`] / [`DecodeError::InvalidUtf8`] if
///   `id` or any non-empty data slot fails to parse as a number.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_psxn(raw: &RawSentence<'_>, layout: &PsxnLayout) -> Result<PsxnData, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < PSXN_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: PSXN_MIN_FIELDS,
            got: f.len(),
        });
    }

    // Field 0: id (u16, optional).
    let id = optional_u16(f[0], 0)?;

    // Field 1: token — opaque bytes, preserved as an owned Vec so the
    // returned PsxnData is 'static-safe (callers can move it freely).
    let token = non_empty(f[1]).map(<[u8]>::to_vec);

    // Fields 2..8: six data slots, each an optional f32.
    let mut slot_values = [None::<f32>; 6];
    for i in 0..6 {
        slot_values[i] = optional_f32(f[2 + i], 2 + i)?;
    }

    // Collect the raw-value-by-role map from the layout.
    let mut roll_direct: Option<f32> = None;
    let mut pitch_direct: Option<f32> = None;
    let mut heave_m: Option<f32> = None;
    let mut roll_sine: Option<f32> = None;
    let mut pitch_sine: Option<f32> = None;

    for (i, slot) in layout.slots.iter().enumerate() {
        let v = slot_values[i];
        match slot {
            PsxnSlot::Roll => roll_direct = v,
            PsxnSlot::Pitch => pitch_direct = v,
            PsxnSlot::Heave => heave_m = v,
            PsxnSlot::RollSineEncoded => roll_sine = v,
            PsxnSlot::PitchSineEncoded => pitch_sine = v,
            PsxnSlot::Ignored => {}
        }
    }

    // Resolve pitch first — it's needed to de-encode sine-encoded roll.
    // Direct value wins; fall back to sine-encoded recovery.
    let pitch_rad = match (pitch_direct, pitch_sine) {
        (Some(p), _) => Some(p),
        (None, Some(ps)) => {
            let arg = -ps;
            if (-1.0..=1.0).contains(&arg) {
                Some(libm::asinf(arg))
            } else {
                None
            }
        }
        (None, None) => None,
    };

    // Resolve roll. For the sine-encoded path we need cos(pitch).
    let roll_rad = match (roll_direct, roll_sine, pitch_rad) {
        (Some(r), _, _) => Some(r),
        (None, Some(rs), Some(p)) => {
            let cp = libm::cosf(p);
            if cp.abs() <= COS_PITCH_MIN {
                // Gimbal lock — can't recover roll from the sine form.
                None
            } else {
                let ratio = rs / cp;
                if (-1.0..=1.0).contains(&ratio) {
                    Some(libm::asinf(ratio))
                } else {
                    None
                }
            }
        }
        // RollSineEncoded in layout but no pitch source → can't resolve.
        // Also: no roll slot at all → None.
        (None, Some(_), None) | (None, None, _) => None,
    };

    // Apply radians→degrees conversion unless the layout opted out.
    let factor = if layout.raw_radians {
        1.0
    } else {
        180.0 / core::f32::consts::PI
    };
    let roll_deg = roll_rad.map(|r| r * factor);
    let pitch_deg = pitch_rad.map(|p| p * factor);

    Ok(PsxnData {
        id,
        token,
        roll_deg,
        pitch_deg,
        heave_m,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    // Envelope-level contract — proprietary, no standard talker
    // -----------------------------------------------------------------

    #[test]
    fn psxn_envelope_has_no_talker() {
        let bytes = build(b"PSXN,10,tok,0.0,0.0,0.0,0,0,0");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.talker, None, "proprietary → no standardised talker");
        assert_eq!(raw.sentence_type, "PSXN");
    }

    // -----------------------------------------------------------------
    // Default layout (rphx) — the legacy-compatible happy path
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_default_layout_yields_roll_pitch_heave_in_degrees() {
        // 0.017453 rad ≈ 1°, 0.034907 rad ≈ 2°, heave = 0.5 m.
        let bytes = build(b"PSXN,10,mytoken,0.017453,0.034907,0.5,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &PsxnLayout::default()).expect("parse");

        assert_eq!(data.id, Some(10));
        assert_eq!(data.token.as_deref(), Some(b"mytoken".as_slice()));
        assert!((data.roll_deg.unwrap() - 1.0).abs() < 0.01);
        assert!((data.pitch_deg.unwrap() - 2.0).abs() < 0.01);
        assert!((data.heave_m.unwrap() - 0.5).abs() < 0.001);
    }

    // -----------------------------------------------------------------
    // Empty fields → None across the board
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_all_empty_data_fields_decode_to_none() {
        let bytes = build(b"PSXN,,,,,,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &PsxnLayout::default()).expect("parse");
        assert_eq!(data.id, None);
        assert_eq!(data.token, None);
        assert_eq!(data.roll_deg, None);
        assert_eq!(data.pitch_deg, None);
        assert_eq!(data.heave_m, None);
    }

    // -----------------------------------------------------------------
    // raw_radians flag disables degree conversion
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_raw_radians_preserves_wire_values() {
        let layout = PsxnLayout {
            raw_radians: true,
            ..PsxnLayout::default()
        };
        let bytes = build(b"PSXN,10,t,0.1,0.2,0.3,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &layout).expect("parse");
        // Not multiplied by 180/π — raw value preserved.
        assert!((data.roll_deg.unwrap() - 0.1).abs() < 0.001);
        assert!((data.pitch_deg.unwrap() - 0.2).abs() < 0.001);
    }

    // -----------------------------------------------------------------
    // Sine-encoded pitch works standalone
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_pitch_sine_encoded_resolves_to_angle() {
        // pitch_sine = -sin(30°) = -0.5. Decoder: pitch = asin(-(-0.5)) = 30°.
        let layout = PsxnLayout {
            slots: [
                PsxnSlot::PitchSineEncoded,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
            ],
            raw_radians: false,
        };
        let bytes = build(b"PSXN,10,,-0.5,,,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &layout).expect("parse");
        assert!(
            (data.pitch_deg.unwrap() - 30.0).abs() < 0.1,
            "got {:?}",
            data.pitch_deg
        );
    }

    // -----------------------------------------------------------------
    // Sine-encoded roll alone (no pitch) → None
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_roll_sine_without_pitch_source_yields_none() {
        let layout = PsxnLayout {
            slots: [
                PsxnSlot::RollSineEncoded,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
            ],
            raw_radians: false,
        };
        let bytes = build(b"PSXN,10,,0.3,,,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &layout).expect("parse");
        assert_eq!(data.roll_deg, None);
    }

    // -----------------------------------------------------------------
    // Sine-encoded roll + direct pitch
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_roll_sine_with_pitch_resolves() {
        // pitch=0 → cos(pitch)=1. roll_sine = sin(10°) * 1 ≈ 0.17365.
        let layout = PsxnLayout {
            slots: [
                PsxnSlot::Pitch,
                PsxnSlot::RollSineEncoded,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
            ],
            raw_radians: false,
        };
        let bytes = build(b"PSXN,10,,0.0,0.17365,,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &layout).expect("parse");
        assert!((data.roll_deg.unwrap() - 10.0).abs() < 0.1);
        assert_eq!(data.pitch_deg, Some(0.0));
    }

    // -----------------------------------------------------------------
    // Gimbal lock — cos(pitch) ≈ 0 → roll None
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_gimbal_lock_yields_none_for_sine_roll() {
        // pitch = π/2 rad = 90°. cos(90°) = 0 → can't recover roll.
        let layout = PsxnLayout {
            slots: [
                PsxnSlot::Pitch,
                PsxnSlot::RollSineEncoded,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
                PsxnSlot::Ignored,
            ],
            raw_radians: false,
        };
        let bytes = build(b"PSXN,10,,1.5707963,0.5,,,,");
        let raw = parse_raw(&bytes);
        let data = decode_psxn(&raw, &layout).expect("parse");
        assert_eq!(data.roll_deg, None);
        assert!((data.pitch_deg.unwrap() - 90.0).abs() < 0.01);
    }

    // -----------------------------------------------------------------
    // Error paths
    // -----------------------------------------------------------------

    #[test]
    fn decode_psxn_too_few_fields_errors() {
        let bytes = build(b"PSXN,10,tok,1.0,2.0"); // 5 fields, need 8
        let raw = parse_raw(&bytes);
        match decode_psxn(&raw, &PsxnLayout::default()) {
            Err(DecodeError::NotEnoughFields { expected: 8, got }) => {
                assert!(got < 8);
            }
            other => panic!("expected NotEnoughFields, got {other:?}"),
        }
    }

    #[test]
    fn decode_psxn_invalid_id_errors_at_field_0() {
        let bytes = build(b"PSXN,abc,tok,1.0,2.0,3.0,0,0,0");
        let raw = parse_raw(&bytes);
        match decode_psxn(&raw, &PsxnLayout::default()) {
            Err(DecodeError::InvalidNumber { field_index: 0 }) => {}
            other => panic!("expected InvalidNumber field 0, got {other:?}"),
        }
    }

    #[test]
    fn decode_psxn_invalid_data_slot_errors_at_correct_field_index() {
        // data2 (wire field 4) is malformed.
        let bytes = build(b"PSXN,10,tok,1.0,2.0,nan-str,0,0,0");
        let raw = parse_raw(&bytes);
        match decode_psxn(&raw, &PsxnLayout::default()) {
            Err(DecodeError::InvalidNumber { field_index: 4 }) => {}
            other => panic!("expected InvalidNumber field 4, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Legacy layout-string FromStr
    // -----------------------------------------------------------------

    #[test]
    fn layout_from_str_rphx_matches_default() {
        let parsed: PsxnLayout = "rphx".parse().expect("parse");
        assert_eq!(parsed.slots[0], PsxnSlot::Roll);
        assert_eq!(parsed.slots[1], PsxnSlot::Pitch);
        assert_eq!(parsed.slots[2], PsxnSlot::Heave);
        assert_eq!(parsed.slots[3], PsxnSlot::Ignored);
        assert_eq!(parsed.slots[4], PsxnSlot::Ignored);
        assert_eq!(parsed.slots[5], PsxnSlot::Ignored);
        assert!(!parsed.raw_radians);
        assert_eq!(parsed, PsxnLayout::default());
    }

    #[test]
    fn layout_from_str_flag_1_sets_raw_radians_anywhere_in_string() {
        for s in ["rphx1", "1rphx", "rp1hx"] {
            let parsed: PsxnLayout = s.parse().expect("parse");
            assert!(parsed.raw_radians, "for input {s:?}");
        }
    }

    #[test]
    fn layout_from_str_case_insensitive() {
        let parsed: PsxnLayout = "RPHX".parse().expect("parse");
        assert_eq!(parsed.slots[0], PsxnSlot::Roll);
        assert_eq!(parsed.slots[1], PsxnSlot::Pitch);
    }

    #[test]
    fn layout_from_str_hippy_variants() {
        let parsed: PsxnLayout = "sqhx".parse().expect("parse");
        assert_eq!(parsed.slots[0], PsxnSlot::RollSineEncoded);
        assert_eq!(parsed.slots[1], PsxnSlot::PitchSineEncoded);
        assert_eq!(parsed.slots[2], PsxnSlot::Heave);
    }

    #[test]
    fn layout_from_str_rejects_unknown_char() {
        match "rpz".parse::<PsxnLayout>() {
            Err(PsxnLayoutParseError::UnknownChar('z')) => {}
            other => panic!("expected UnknownChar('z'), got {other:?}"),
        }
    }

    #[test]
    fn layout_from_str_rejects_too_many_slots() {
        match "rphxrph".parse::<PsxnLayout>() {
            Err(PsxnLayoutParseError::TooManySlots) => {}
            other => panic!("expected TooManySlots, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Top-level dispatcher
    // -----------------------------------------------------------------

    #[test]
    fn top_level_decode_dispatches_psxn_with_default_options() {
        let bytes = build(b"PSXN,10,tok,0.017453,0.0,0.0,,,");
        let raw = parse_raw(&bytes);
        let msg = crate::decode(&raw).expect("dispatcher");
        match msg {
            crate::Nmea0183Message::Psxn(d) => {
                assert_eq!(d.id, Some(10));
                assert!((d.roll_deg.unwrap() - 1.0).abs() < 0.01);
            }
            other => panic!("expected Psxn, got {other:?}"),
        }
    }

    #[test]
    fn decode_with_custom_layout_applies_configured_slots() {
        // Layout `qpxxxx` with raw_radians flag — only PitchSineEncoded in slot 0.
        let layout: PsxnLayout = "q1".parse().expect("layout parse");
        let opts = crate::DecodeOptions::default().with_psxn_layout(layout);
        // pitch_sine = -sin(π/6) = -0.5 → pitch = π/6 rad = 0.5236.
        let bytes = build(b"PSXN,10,tok,-0.5,,,,,");
        let raw = parse_raw(&bytes);
        let msg = crate::decode_with(&raw, &opts).expect("dispatcher");
        match msg {
            crate::Nmea0183Message::Psxn(d) => {
                // raw_radians on → value kept in radians.
                assert!((d.pitch_deg.unwrap() - core::f32::consts::FRAC_PI_6).abs() < 0.001);
            }
            other => panic!("expected Psxn, got {other:?}"),
        }
    }
}
