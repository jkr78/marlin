//! # marlin-nmea-0183
//!
//! Sans-I/O typed decoders for NMEA 0183 sentences. Built on
//! [`marlin-nmea-envelope`](marlin_nmea_envelope).
//!
//! The envelope crate produces [`RawSentence`] values — verified framing,
//! checksum-checked, fields split as byte slices. This crate turns those
//! into typed Rust structs:
//!
//! ```text
//!  bytes → marlin_nmea_envelope → RawSentence → marlin_nmea_0183 → Nmea0183Message
//! ```
//!
//! # Supported sentence types
//!
//! - **`$__GGA`** — [`GgaData`] — position + fix quality + satellites
//! - **`$__HDT`** — [`HdtData`] — true heading
//! - **`$__VTG`** — [`VtgData`] — course & speed over ground
//! - **`$PSXN`** — [`PsxnData`] — Kongsberg-family proprietary motion; slot
//!   meanings are install-configurable via [`PsxnLayout`] / [`DecodeOptions`]
//! - **`$PRDID`** — [`PrdidData`] — proprietary attitude with multiple
//!   vendor dialects; default refuses to guess (emits
//!   [`PrdidData::Raw`]). Select a dialect via
//!   [`DecodeOptions::with_prdid_dialect`].
//!
//! The talker ID is preserved as metadata on each typed struct rather
//! than dispatched on; `$GPGGA`, `$INGGA`, `$GNGGA` all decode to
//! [`GgaData`] with different `talker` values.
//!
//! # Quickstart
//!
//! ```
//! use marlin_nmea_envelope::{OneShot, SentenceSource};
//! use marlin_nmea_0183::{decode, Nmea0183Message};
//!
//! // Classic NMEA 0183 GGA example (checksum 0x47 = XOR of body bytes).
//! let mut parser = OneShot::new();
//! parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
//!
//! let raw = parser.next_sentence().unwrap().unwrap();
//! match decode(&raw).unwrap() {
//!     Nmea0183Message::Gga(gga) => {
//!         assert_eq!(gga.talker, Some(*b"GP"));
//!         assert_eq!(gga.satellites_used, Some(8));
//!     }
//!     _ => panic!("expected GGA"),
//! }
//! ```
//!
//! # Extension
//!
//! Each sentence type has a **public** per-sentence decoder
//! ([`decode_gga`], [`decode_hdt`], …). Downstream crates that need
//! proprietary sentences this crate doesn't decode can build their own
//! enum and delegate to these decoders. See the crate
//! [README](https://docs.rs/marlin-nmea-0183/latest/marlin_nmea_0183)
//! for a full example.
//!
//! # Policy
//!
//! Unknown sentence types are returned as
//! [`Nmea0183Message::Unknown`] with the raw sentence preserved. They
//! are **not** silently dropped and **not** returned as errors — the
//! caller decides what to do with them.

#![doc(html_root_url = "https://docs.rs/marlin-nmea-0183/0.1.0")]
#![no_std]

extern crate alloc;

mod error;
mod message;
mod parser;
mod sentences;
mod util;

#[cfg(test)]
pub(crate) mod testing;

pub use error::DecodeError;
pub use message::Nmea0183Message;
pub use parser::{Nmea0183Error, Nmea0183Parser, Parser};
pub use sentences::{
    decode_gga, decode_hdt, decode_prdid, decode_prdid_pitch_roll_heading,
    decode_prdid_roll_pitch_heading, decode_psxn, decode_vtg, GgaData, GgaFixQuality, HdtData,
    PrdidData, PrdidDialect, PrdidPitchRollHeading, PrdidRollPitchHeading, PsxnData, PsxnLayout,
    PsxnLayoutParseError, PsxnSlot, UtcTime, VtgData, VtgMode,
};

// Re-export the envelope's `RawSentence` for convenience — most callers
// of this crate will need it to pattern-match `Nmea0183Message::Unknown`.
pub use marlin_nmea_envelope::RawSentence;

use marlin_nmea_envelope::RawSentence as Raw;

/// Decode a [`RawSentence`] into a typed [`Nmea0183Message`] using the
/// default [`DecodeOptions`].
///
/// Dispatch is on [`sentence_type`](marlin_nmea_envelope::RawSentence::sentence_type)
/// alone — the talker ID is preserved on each typed struct but not used
/// for routing. `$GPGGA`, `$INGGA`, `$GNGGA` all land in
/// [`Nmea0183Message::Gga`] with distinct `talker` values.
///
/// Equivalent to `decode_with(raw, &DecodeOptions::default())`. Use
/// [`decode_with`] when you need to configure ambiguous decodings such
/// as the PSXN data-slot layout.
///
/// Sentence types this crate doesn't recognize return
/// [`Nmea0183Message::Unknown`] wrapping a clone of the input, so the
/// caller can decode them with their own logic or discard them.
///
/// # Errors
///
/// Returns [`DecodeError`] when a sentence of a **recognized** type has
/// malformed fields (wrong field count, invalid number, invalid
/// coordinate, etc.). Unknown types are **not** errors — see above.
pub fn decode<'a>(raw: &Raw<'a>) -> Result<Nmea0183Message<'a>, DecodeError> {
    decode_with(raw, &DecodeOptions::default())
}

/// Decode a [`RawSentence`] into a typed [`Nmea0183Message`] using the
/// supplied [`DecodeOptions`].
///
/// The options carry per-sentence knobs for decodings that can't be
/// inferred from the bytes alone — currently the [`PsxnLayout`].
///
/// # Errors
///
/// Propagates the same [`DecodeError`] variants as each per-sentence
/// decoder. Unknown sentence types are **not** errors — they return
/// [`Nmea0183Message::Unknown`] wrapping the input.
pub fn decode_with<'a>(
    raw: &Raw<'a>,
    options: &DecodeOptions,
) -> Result<Nmea0183Message<'a>, DecodeError> {
    match raw.sentence_type {
        "GGA" => Ok(Nmea0183Message::Gga(decode_gga(raw)?)),
        "HDT" => Ok(Nmea0183Message::Hdt(decode_hdt(raw)?)),
        "VTG" => Ok(Nmea0183Message::Vtg(decode_vtg(raw)?)),
        "PSXN" => Ok(Nmea0183Message::Psxn(decode_psxn(
            raw,
            &options.psxn_layout,
        )?)),
        "PRDID" => Ok(Nmea0183Message::Prdid(decode_prdid(
            raw,
            options.prdid_dialect,
        )?)),
        _ => Ok(Nmea0183Message::Unknown(raw.clone())),
    }
}

/// Runtime configuration for ambiguous sentence decodings.
///
/// Several NMEA sentence types cannot be interpreted correctly from
/// bytes alone — their schema depends on vendor or install-time
/// configuration. `DecodeOptions` carries the knobs needed to resolve
/// that ambiguity.
///
/// Construct with [`Default`] and chain builder methods:
///
/// ```
/// use marlin_nmea_0183::{DecodeOptions, PsxnLayout};
///
/// let opts = DecodeOptions::default()
///     .with_psxn_layout("rphx1".parse::<PsxnLayout>().unwrap());
/// ```
///
/// `#[non_exhaustive]` so future knobs (PRDID dialect, checksum
/// policy, etc.) can land in minor versions without a breaking change.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DecodeOptions {
    /// Layout used when decoding `$PSXN` sentences. See [`PsxnLayout`]
    /// for the full configuration surface.
    pub psxn_layout: PsxnLayout,
    /// Dialect used when decoding `$PRDID` sentences. Default
    /// [`PrdidDialect::Unknown`] refuses to guess and emits
    /// [`PrdidData::Raw`].
    pub prdid_dialect: PrdidDialect,
}

impl DecodeOptions {
    /// Set the PSXN layout.
    #[must_use]
    pub fn with_psxn_layout(mut self, layout: PsxnLayout) -> Self {
        self.psxn_layout = layout;
        self
    }

    /// Set the PRDID dialect.
    #[must_use]
    pub fn with_prdid_dialect(mut self, dialect: PrdidDialect) -> Self {
        self.prdid_dialect = dialect;
        self
    }
}
