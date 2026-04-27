//! PRDID — proprietary attitude sentence (multiple vendor dialects).
//!
//! PRDID is an address-space squatted on by several motion / attitude
//! vendors with **incompatible field orderings**. The bytes alone are
//! ambiguous — correct decoding requires knowing which vendor emitted
//! the sentence.
//!
//! This module ships:
//!
//! - One typed struct per dialect ([`PrdidPitchRollHeading`],
//!   [`PrdidRollPitchHeading`]).
//! - One public decoder per dialect (`decode_prdid_pitch_roll_heading`,
//!   `decode_prdid_roll_pitch_heading`) — callers who know their
//!   hardware can skip the dispatcher entirely and get a precisely
//!   typed result.
//! - A [`PrdidDialect`] enum for runtime selection, carried on
//!   [`crate::DecodeOptions`].
//! - A [`PrdidData`] enum that wraps any of the dialect-specific
//!   structs, plus a [`PrdidData::Raw`] variant for when the default
//!   `PrdidDialect::Unknown` dialect is in effect.
//!
//! # Policy
//!
//! The default [`PrdidDialect::Unknown`] **refuses to guess**. The
//! top-level [`crate::decode`] emits `Nmea0183Message::Prdid(PrdidData::Raw { fields })`
//! for PRDID sentences until the caller configures a dialect via
//! [`crate::DecodeOptions::with_prdid_dialect`]. This protects against
//! silent field-order bugs when hardware is unknown or heterogeneous.
//!
//! # Dialects in this crate
//!
//! | Variant | Field order | Seen on |
//! | --- | --- | --- |
//! | `PitchRollHeading` | pitch, roll, heading | Teledyne RDI ADCPs (canonical) |
//! | `RollPitchHeading` | roll, pitch, heading | Alternative ordering in some integration guides |
//!
//! Both dialects expect 3 numeric fields (degrees). Verify against a
//! real capture from your hardware before production use — see
//! `TODO.md` pre-release checklist.

use alloc::vec::Vec;

use marlin_nmea_envelope::RawSentence;

use crate::util::optional_f32;
use crate::DecodeError;

// ---------------------------------------------------------------------------
// Dialect-specific structs (one per field ordering)
// ---------------------------------------------------------------------------

/// PRDID decoded with field order `pitch, roll, heading` — the
/// canonical Teledyne RDI ADCP ordering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrdidPitchRollHeading {
    /// Pitch in degrees.
    pub pitch_deg: Option<f32>,
    /// Roll in degrees.
    pub roll_deg: Option<f32>,
    /// True heading in degrees (0..360).
    pub heading_deg: Option<f32>,
}

/// PRDID decoded with field order `roll, pitch, heading` — an
/// alternative ordering seen on some integration guides. **Not**
/// compatible with the Teledyne RDI ordering; swapping them produces
/// wrong data silently.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrdidRollPitchHeading {
    /// Roll in degrees.
    pub roll_deg: Option<f32>,
    /// Pitch in degrees.
    pub pitch_deg: Option<f32>,
    /// True heading in degrees (0..360).
    pub heading_deg: Option<f32>,
}

// ---------------------------------------------------------------------------
// Dialect selector + wrapping enum
// ---------------------------------------------------------------------------

/// Runtime selector for which PRDID field ordering to decode with.
///
/// The **default** is [`Self::Unknown`]: PRDID sentences decode to
/// [`PrdidData::Raw`] and the caller is forced to opt into a specific
/// dialect via [`crate::DecodeOptions::with_prdid_dialect`] before
/// getting typed data. This prevents silent data corruption when the
/// source hardware is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PrdidDialect {
    /// **Default — refuse to guess.** PRDID sentences decode to
    /// [`PrdidData::Raw`] carrying owned copies of the raw field
    /// bytes. Callers can inspect them, log them, or route them to
    /// domain-specific logic.
    #[default]
    Unknown,
    /// Decode as pitch, roll, heading (Teledyne RDI ADCP canonical).
    PitchRollHeading,
    /// Decode as roll, pitch, heading.
    RollPitchHeading,
}

/// Typed payload of a `$PRDID` sentence.
///
/// The variant depends on which [`PrdidDialect`] was configured. With
/// the default [`PrdidDialect::Unknown`], PRDID always decodes to
/// [`Self::Raw`] regardless of field content.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PrdidData {
    /// Decoded under the `PitchRollHeading` dialect.
    PitchRollHeading(PrdidPitchRollHeading),
    /// Decoded under the `RollPitchHeading` dialect.
    RollPitchHeading(PrdidRollPitchHeading),
    /// No dialect configured (or the caller explicitly chose
    /// [`PrdidDialect::Unknown`]). The PRDID field bytes are preserved
    /// as owned `Vec<u8>` so callers can handle them with their own
    /// logic without re-parsing the envelope.
    Raw {
        /// Owned copies of the sentence's fields (`data0..dataN`).
        /// Empty fields are preserved as empty `Vec<u8>`, matching the
        /// envelope's empty-field semantics.
        fields: Vec<Vec<u8>>,
    },
}

// ---------------------------------------------------------------------------
// Decoders
// ---------------------------------------------------------------------------

/// Minimum fields every typed PRDID dialect expects (3 angles).
const PRDID_MIN_FIELDS: usize = 3;

/// Decode a PRDID sentence using the given dialect.
///
/// Dispatches to the appropriate per-dialect decoder, or returns a
/// [`PrdidData::Raw`] for [`PrdidDialect::Unknown`].
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if a typed dialect was selected
///   and the sentence has fewer than 3 fields.
/// - [`DecodeError::InvalidNumber`] / [`DecodeError::InvalidUtf8`] on
///   per-field decode failures.
///
/// Never errors for [`PrdidDialect::Unknown`] — preserves whatever
/// fields the envelope parsed.
pub fn decode_prdid(
    raw: &RawSentence<'_>,
    dialect: PrdidDialect,
) -> Result<PrdidData, DecodeError> {
    match dialect {
        PrdidDialect::Unknown => Ok(PrdidData::Raw {
            fields: raw.fields.iter().map(|f| f.to_vec()).collect(),
        }),
        PrdidDialect::PitchRollHeading => {
            decode_prdid_pitch_roll_heading(raw).map(PrdidData::PitchRollHeading)
        }
        PrdidDialect::RollPitchHeading => {
            decode_prdid_roll_pitch_heading(raw).map(PrdidData::RollPitchHeading)
        }
    }
}

/// Decode a PRDID sentence as `pitch, roll, heading` (Teledyne RDI
/// canonical ordering).
///
/// The caller asserts the dialect by choosing this function; the
/// decoder does not verify that the bytes came from Teledyne hardware.
///
/// # Errors
///
/// - [`DecodeError::NotEnoughFields`] if the sentence has fewer than
///   3 fields.
/// - [`DecodeError::InvalidNumber`] / [`DecodeError::InvalidUtf8`] if
///   any non-empty field is not a valid number.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_prdid_pitch_roll_heading(
    raw: &RawSentence<'_>,
) -> Result<PrdidPitchRollHeading, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < PRDID_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: PRDID_MIN_FIELDS,
            got: f.len(),
        });
    }
    Ok(PrdidPitchRollHeading {
        pitch_deg: optional_f32(f[0], 0)?,
        roll_deg: optional_f32(f[1], 1)?,
        heading_deg: optional_f32(f[2], 2)?,
    })
}

/// Decode a PRDID sentence as `roll, pitch, heading` (alternative
/// ordering).
///
/// # Errors
///
/// Same as [`decode_prdid_pitch_roll_heading`].
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn decode_prdid_roll_pitch_heading(
    raw: &RawSentence<'_>,
) -> Result<PrdidRollPitchHeading, DecodeError> {
    let f = raw.fields.as_slice();
    if f.len() < PRDID_MIN_FIELDS {
        return Err(DecodeError::NotEnoughFields {
            expected: PRDID_MIN_FIELDS,
            got: f.len(),
        });
    }
    Ok(PrdidRollPitchHeading {
        roll_deg: optional_f32(f[0], 0)?,
        pitch_deg: optional_f32(f[1], 1)?,
        heading_deg: optional_f32(f[2], 2)?,
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
    // Envelope-level contract — proprietary, no talker
    // -----------------------------------------------------------------

    #[test]
    fn prdid_envelope_has_no_talker() {
        let bytes = build(b"PRDID,1.0,2.0,180.0");
        let raw = parse_raw(&bytes);
        assert_eq!(raw.talker, None, "proprietary → no standardised talker");
        assert_eq!(raw.sentence_type, "PRDID");
    }

    // -----------------------------------------------------------------
    // Default dialect = Unknown → always Raw
    // -----------------------------------------------------------------

    #[test]
    fn decode_prdid_with_unknown_dialect_emits_raw_fields() {
        let bytes = build(b"PRDID,1.5,-0.5,92.3");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::Unknown).expect("decode");
        let fields = match data {
            PrdidData::Raw { fields } => fields,
            other => panic!("expected Raw, got {other:?}"),
        };
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], b"1.5");
        assert_eq!(fields[1], b"-0.5");
        assert_eq!(fields[2], b"92.3");
    }

    #[test]
    fn decode_prdid_unknown_preserves_empty_fields() {
        let bytes = build(b"PRDID,1.5,,92.3");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::Unknown).expect("decode");
        let fields = match data {
            PrdidData::Raw { fields } => fields,
            other => panic!("expected Raw, got {other:?}"),
        };
        assert_eq!(fields.len(), 3);
        assert!(fields[1].is_empty());
    }

    #[test]
    fn decode_prdid_unknown_never_errors_on_field_count() {
        // Raw mode preserves *any* field count without erroring.
        let bytes = build(b"PRDID,only-one-field");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::Unknown).expect("decode");
        let fields = match data {
            PrdidData::Raw { fields } => fields,
            other => panic!("expected Raw, got {other:?}"),
        };
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0], b"only-one-field");
    }

    // -----------------------------------------------------------------
    // PitchRollHeading dialect (Teledyne RDI canonical)
    // -----------------------------------------------------------------

    #[test]
    fn decode_prdid_pitch_roll_heading_dispatches_correctly() {
        // Bytes: 1.0, 2.0, 180.0
        // Under PRH dialect: pitch=1.0, roll=2.0, heading=180.0
        let bytes = build(b"PRDID,1.0,2.0,180.0");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::PitchRollHeading).expect("decode");
        let prh = match data {
            PrdidData::PitchRollHeading(d) => d,
            other => panic!("expected PitchRollHeading, got {other:?}"),
        };
        assert!((prh.pitch_deg.unwrap() - 1.0).abs() < 0.001);
        assert!((prh.roll_deg.unwrap() - 2.0).abs() < 0.001);
        assert!((prh.heading_deg.unwrap() - 180.0).abs() < 0.001);
    }

    #[test]
    fn decode_prdid_pitch_roll_heading_direct_decoder_matches_dispatcher() {
        // Calling the per-dialect decoder directly should give the same
        // struct as calling through decode_prdid with the dialect set.
        let bytes = build(b"PRDID,-1.5,0.5,45.0");
        let raw = parse_raw(&bytes);
        let direct = decode_prdid_pitch_roll_heading(&raw).expect("direct");
        let via_dispatch = decode_prdid(&raw, PrdidDialect::PitchRollHeading).expect("via");
        assert_eq!(PrdidData::PitchRollHeading(direct), via_dispatch);
    }

    // -----------------------------------------------------------------
    // RollPitchHeading dialect (alternative ordering)
    // -----------------------------------------------------------------

    #[test]
    fn decode_prdid_roll_pitch_heading_swaps_first_two_fields() {
        // Same bytes as above, different dialect → different interpretation.
        // Bytes: 1.0, 2.0, 180.0
        // Under RPH dialect: roll=1.0, pitch=2.0, heading=180.0
        let bytes = build(b"PRDID,1.0,2.0,180.0");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::RollPitchHeading).expect("decode");
        let rph = match data {
            PrdidData::RollPitchHeading(d) => d,
            other => panic!("expected RollPitchHeading, got {other:?}"),
        };
        // First field is roll under RPH, pitch under PRH — that's the
        // whole reason these are different dialects.
        assert!((rph.roll_deg.unwrap() - 1.0).abs() < 0.001);
        assert!((rph.pitch_deg.unwrap() - 2.0).abs() < 0.001);
        assert!((rph.heading_deg.unwrap() - 180.0).abs() < 0.001);
    }

    #[test]
    fn prh_and_rph_produce_different_interpretations_of_same_bytes() {
        // The central reason this library has two dialects: the same
        // wire bytes give different decoded quantities.
        let bytes = build(b"PRDID,10.0,20.0,90.0");
        let raw = parse_raw(&bytes);
        let prh = decode_prdid_pitch_roll_heading(&raw).expect("prh");
        let rph = decode_prdid_roll_pitch_heading(&raw).expect("rph");
        // Under PRH, field 0 is pitch. Under RPH, field 0 is roll. So:
        assert_eq!(prh.pitch_deg, rph.roll_deg, "first field role differs");
        assert_eq!(prh.roll_deg, rph.pitch_deg, "second field role differs");
        assert_eq!(prh.heading_deg, rph.heading_deg, "heading is 3rd for both");
    }

    // -----------------------------------------------------------------
    // Empty fields in typed dialects → None
    // -----------------------------------------------------------------

    #[test]
    fn decode_prdid_typed_preserves_none_for_empty_fields() {
        let bytes = build(b"PRDID,,,");
        let raw = parse_raw(&bytes);
        let data = decode_prdid(&raw, PrdidDialect::PitchRollHeading).expect("decode");
        let prh = match data {
            PrdidData::PitchRollHeading(d) => d,
            other => panic!("expected PitchRollHeading, got {other:?}"),
        };
        assert_eq!(prh.pitch_deg, None);
        assert_eq!(prh.roll_deg, None);
        assert_eq!(prh.heading_deg, None);
    }

    // -----------------------------------------------------------------
    // Error paths
    // -----------------------------------------------------------------

    #[test]
    fn decode_prdid_typed_errors_on_too_few_fields() {
        let bytes = build(b"PRDID,1.0,2.0"); // only 2 fields
        let raw = parse_raw(&bytes);
        match decode_prdid(&raw, PrdidDialect::PitchRollHeading) {
            Err(DecodeError::NotEnoughFields {
                expected: 3,
                got: 2,
            }) => {}
            other => panic!("expected NotEnoughFields 3/2, got {other:?}"),
        }
    }

    #[test]
    fn decode_prdid_typed_errors_on_invalid_number() {
        let bytes = build(b"PRDID,not-a-number,2.0,90.0");
        let raw = parse_raw(&bytes);
        match decode_prdid(&raw, PrdidDialect::PitchRollHeading) {
            Err(DecodeError::InvalidNumber { field_index: 0 }) => {}
            other => panic!("expected InvalidNumber field 0, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Default PrdidDialect is Unknown
    // -----------------------------------------------------------------

    #[test]
    fn prdid_dialect_default_is_unknown() {
        assert_eq!(PrdidDialect::default(), PrdidDialect::Unknown);
    }

    // -----------------------------------------------------------------
    // Top-level dispatcher routes PRDID through DecodeOptions
    // -----------------------------------------------------------------

    #[test]
    fn top_level_decode_uses_default_unknown_dialect() {
        let bytes = build(b"PRDID,1.0,2.0,180.0");
        let raw = parse_raw(&bytes);
        let msg = crate::decode(&raw).expect("dispatcher");
        match msg {
            crate::Nmea0183Message::Prdid(PrdidData::Raw { fields }) => {
                assert_eq!(fields.len(), 3);
            }
            other => panic!("expected Prdid(Raw), got {other:?}"),
        }
    }

    #[test]
    fn decode_with_configures_prdid_dialect() {
        let opts =
            crate::DecodeOptions::default().with_prdid_dialect(PrdidDialect::PitchRollHeading);
        let bytes = build(b"PRDID,1.0,2.0,180.0");
        let raw = parse_raw(&bytes);
        let msg = crate::decode_with(&raw, &opts).expect("dispatcher");
        match msg {
            crate::Nmea0183Message::Prdid(PrdidData::PitchRollHeading(prh)) => {
                assert!((prh.pitch_deg.unwrap() - 1.0).abs() < 0.001);
            }
            other => panic!("expected Prdid(PitchRollHeading), got {other:?}"),
        }
    }

    #[test]
    fn decode_with_switches_between_dialects() {
        let bytes = build(b"PRDID,10.0,20.0,30.0");
        let raw = parse_raw(&bytes);

        let prh_opts =
            crate::DecodeOptions::default().with_prdid_dialect(PrdidDialect::PitchRollHeading);
        let rph_opts =
            crate::DecodeOptions::default().with_prdid_dialect(PrdidDialect::RollPitchHeading);

        let prh_msg = crate::decode_with(&raw, &prh_opts).expect("prh");
        let rph_msg = crate::decode_with(&raw, &rph_opts).expect("rph");

        match (prh_msg, rph_msg) {
            (
                crate::Nmea0183Message::Prdid(PrdidData::PitchRollHeading(prh)),
                crate::Nmea0183Message::Prdid(PrdidData::RollPitchHeading(rph)),
            ) => {
                // Same bytes, opposite interpretation of first two fields.
                assert_eq!(prh.pitch_deg, rph.roll_deg);
                assert_eq!(prh.roll_deg, rph.pitch_deg);
            }
            other => panic!("unexpected pair: {other:?}"),
        }
    }
}
