//! Typed-message parser wrapper — ergonomic layer over the envelope.
//!
//! The envelope crate's parsers emit [`RawSentence`] values; this
//! module wraps them so callers get typed [`Nmea0183Message`] values
//! directly, one `feed` / `next_message` loop instead of two.

use marlin_nmea_envelope::{OneShot, RawSentence, SentenceSource, Streaming};

use crate::{decode_with, DecodeError, DecodeOptions, Nmea0183Message};

// ---------------------------------------------------------------------------
// Unified error type
// ---------------------------------------------------------------------------

/// Errors surfacing from [`Nmea0183Parser::next_message`] (and from the
/// [`Parser`] enum's delegating method).
///
/// The two variants distinguish **where** the failure happened:
///
/// - [`Self::Envelope`] — framing, checksum, TAG block, buffer overflow.
///   The sentence bytes are malformed at the NMEA 0183 envelope layer.
/// - [`Self::Decode`] — the envelope parsed cleanly, but the typed
///   decoder couldn't interpret the fields (wrong field count,
///   invalid number, out-of-range coordinate, etc.).
///
/// Both categories are recoverable — a parser that hits either error
/// has advanced past the offending sentence and can continue to the
/// next one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Nmea0183Error {
    /// Envelope-level failure (framing, checksum, TAG block, ...).
    #[error("envelope error: {0}")]
    Envelope(#[from] marlin_nmea_envelope::Error),
    /// Typed-decode failure (malformed field, wrong field count, ...).
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),
}

// ---------------------------------------------------------------------------
// Generic parser wrapper
// ---------------------------------------------------------------------------

/// Typed-message parser built on top of any envelope-level
/// [`SentenceSource`] that produces [`RawSentence`] values.
///
/// This wrapper:
///
/// - Owns an envelope parser (`OneShot` or `Streaming`).
/// - Carries [`DecodeOptions`] for ambiguous sentences (PSXN, PRDID).
/// - Exposes the same `feed` / `next_message` shape at every layer.
///
/// # Example
///
/// ```
/// use marlin_nmea_envelope::OneShot;
/// use marlin_nmea_0183::{Nmea0183Message, Nmea0183Parser};
///
/// let mut parser = Nmea0183Parser::new(OneShot::new());
/// parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
/// match parser.next_message().unwrap().unwrap() {
///     Nmea0183Message::Gga(gga) => assert_eq!(gga.talker, Some(*b"GP")),
///     _ => panic!("expected GGA"),
/// }
/// ```
///
/// For runtime mode selection (one-shot vs streaming) without
/// generics infecting the call site, use the [`Parser`] enum instead.
#[derive(Debug)]
pub struct Nmea0183Parser<P> {
    inner: P,
    options: DecodeOptions,
}

impl<P> Nmea0183Parser<P> {
    /// Construct a parser that wraps `inner` with default
    /// [`DecodeOptions`].
    #[must_use]
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            options: DecodeOptions::default(),
        }
    }

    /// Construct a parser that wraps `inner` with caller-provided
    /// [`DecodeOptions`]. Use this to configure PSXN layout, PRDID
    /// dialect, etc. at construction time.
    #[must_use]
    pub fn with_options(inner: P, options: DecodeOptions) -> Self {
        Self { inner, options }
    }

    /// Borrow the current [`DecodeOptions`].
    #[must_use]
    pub fn options(&self) -> &DecodeOptions {
        &self.options
    }

    /// Replace the [`DecodeOptions`] on an already-constructed parser.
    /// Subsequent calls to [`next_message`](Self::next_message) use
    /// the new options immediately.
    pub fn set_options(&mut self, options: DecodeOptions) {
        self.options = options;
    }

    /// Borrow the underlying envelope parser. Useful for diagnostics
    /// or if a caller wants to temporarily drop to the envelope layer.
    #[must_use]
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Mutably borrow the underlying envelope parser.
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Unwrap back to the underlying envelope parser. Drops the
    /// decode options along with the wrapper.
    #[must_use]
    pub fn into_inner(self) -> P {
        self.inner
    }
}

// The core feed / next_message impl. Constrained to sources that emit
// RawSentence values — that's `OneShot` and `Streaming` from the
// envelope crate.
impl<P> Nmea0183Parser<P>
where
    P: for<'a> SentenceSource<Item<'a> = RawSentence<'a>>,
{
    /// Push raw bytes into the underlying envelope parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.inner.feed(bytes);
    }

    /// Pull the next complete typed message.
    ///
    /// Returns:
    ///
    /// - `Some(Ok(message))` — envelope parsed and typed decode
    ///   succeeded.
    /// - `Some(Err(Nmea0183Error::Envelope(_)))` — envelope parsing
    ///   failed (bad checksum, malformed framing, etc.). The parser
    ///   has advanced past the offending bytes and is ready to
    ///   continue.
    /// - `Some(Err(Nmea0183Error::Decode(_)))` — envelope succeeded
    ///   but the typed decoder rejected the fields. Same recovery
    ///   semantic.
    /// - `None` — no complete sentence is available yet; call
    ///   [`feed`](Self::feed) with more bytes.
    pub fn next_message(&mut self) -> Option<Result<Nmea0183Message<'_>, Nmea0183Error>> {
        match self.inner.next_sentence()? {
            Ok(raw) => Some(decode_with(&raw, &self.options).map_err(Nmea0183Error::from)),
            Err(e) => Some(Err(Nmea0183Error::from(e))),
        }
    }
}

impl<P: Default> Default for Nmea0183Parser<P> {
    fn default() -> Self {
        Self::new(P::default())
    }
}

// ---------------------------------------------------------------------------
// Runtime-dispatch enum
// ---------------------------------------------------------------------------

/// Runtime selector between one-shot and streaming typed parsers.
///
/// This is the typed-layer analogue of
/// [`marlin_nmea_envelope::Parser`]. Use it when the mode is a
/// configuration choice rather than a compile-time decision:
///
/// ```
/// use marlin_nmea_0183::Parser;
///
/// let use_streaming = true;
/// let mut parser = if use_streaming {
///     Parser::streaming()
/// } else {
///     Parser::one_shot()
/// };
/// parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n");
/// let _ = parser.next_message().unwrap().unwrap();
/// ```
///
/// Dispatch is a match (static, inlined); no heap allocation or
/// vtable lookup.
#[derive(Debug)]
pub enum Parser {
    /// Single-sentence-per-feed mode; wraps
    /// [`Nmea0183Parser`]`<`[`OneShot`]`>`.
    OneShot(Nmea0183Parser<OneShot>),
    /// Buffered streaming mode; wraps
    /// [`Nmea0183Parser`]`<`[`Streaming`]`>`.
    Streaming(Nmea0183Parser<Streaming>),
}

impl Parser {
    /// Construct a one-shot parser with default [`DecodeOptions`].
    #[must_use]
    pub fn one_shot() -> Self {
        Self::OneShot(Nmea0183Parser::new(OneShot::new()))
    }

    /// Construct a one-shot parser with caller-provided options.
    #[must_use]
    pub fn one_shot_with_options(options: DecodeOptions) -> Self {
        Self::OneShot(Nmea0183Parser::with_options(OneShot::new(), options))
    }

    /// Construct a streaming parser with the envelope's default
    /// maximum buffer size.
    #[must_use]
    pub fn streaming() -> Self {
        Self::Streaming(Nmea0183Parser::new(Streaming::new()))
    }

    /// Construct a streaming parser with a specified maximum buffer
    /// size. See [`Streaming::with_capacity`] for the envelope-level
    /// semantics.
    #[must_use]
    pub fn streaming_with_capacity(max_size: usize) -> Self {
        Self::Streaming(Nmea0183Parser::new(Streaming::with_capacity(max_size)))
    }

    /// Construct a streaming parser with caller-provided options.
    #[must_use]
    pub fn streaming_with_options(options: DecodeOptions) -> Self {
        Self::Streaming(Nmea0183Parser::with_options(Streaming::new(), options))
    }

    /// Borrow the current [`DecodeOptions`] regardless of variant.
    #[must_use]
    pub fn options(&self) -> &DecodeOptions {
        match self {
            Self::OneShot(p) => p.options(),
            Self::Streaming(p) => p.options(),
        }
    }

    /// Replace the [`DecodeOptions`] on the active variant.
    pub fn set_options(&mut self, options: DecodeOptions) {
        match self {
            Self::OneShot(p) => p.set_options(options),
            Self::Streaming(p) => p.set_options(options),
        }
    }

    /// Push raw bytes into the active parser. See the per-mode docs
    /// ([`OneShot`] vs [`Streaming`]) for semantics.
    pub fn feed(&mut self, bytes: &[u8]) {
        match self {
            Self::OneShot(p) => p.feed(bytes),
            Self::Streaming(p) => p.feed(bytes),
        }
    }

    /// Pull the next typed message. See
    /// [`Nmea0183Parser::next_message`] for semantics.
    pub fn next_message(&mut self) -> Option<Result<Nmea0183Message<'_>, Nmea0183Error>> {
        match self {
            Self::OneShot(p) => p.next_message(),
            Self::Streaming(p) => p.next_message(),
        }
    }
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
    use crate::testing::build;
    use crate::{GgaFixQuality, PrdidData, PrdidDialect, PsxnLayout};
    use alloc::vec::Vec;

    // -----------------------------------------------------------------
    // End-to-end: bytes → typed message
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_decodes_gga_bytes_into_typed_message() {
        let mut parser = Parser::one_shot();
        parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
        let msg = parser.next_message().unwrap().unwrap();
        let gga = match msg {
            Nmea0183Message::Gga(d) => d,
            other => panic!("expected Gga, got {other:?}"),
        };
        assert_eq!(gga.talker, Some(*b"GP"));
        assert_eq!(gga.satellites_used, Some(8));
        assert_eq!(gga.fix_quality, GgaFixQuality::GpsFix);
    }

    #[test]
    fn streaming_decodes_multiple_messages_from_one_feed() {
        let mut parser = Parser::streaming();
        // Build three well-formed sentences back-to-back.
        let gga = build(b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        let hdt = build(b"INHDT,100.0,T");
        let vtg = build(b"GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,A");
        let mut combined = Vec::new();
        combined.extend_from_slice(&gga);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&hdt);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&vtg);
        combined.extend_from_slice(b"\r\n");
        parser.feed(&combined);

        let mut seen: Vec<&'static str> = Vec::new();
        while let Some(result) = parser.next_message() {
            let msg = result.unwrap();
            seen.push(match msg {
                Nmea0183Message::Gga(_) => "gga",
                Nmea0183Message::Hdt(_) => "hdt",
                Nmea0183Message::Vtg(_) => "vtg",
                _ => "other",
            });
        }
        assert_eq!(seen, ["gga", "hdt", "vtg"]);
    }

    // -----------------------------------------------------------------
    // Options propagation: Parser honors DecodeOptions
    // -----------------------------------------------------------------

    #[test]
    fn parser_applies_configured_prdid_dialect() {
        let opts = DecodeOptions::default().with_prdid_dialect(PrdidDialect::PitchRollHeading);
        let mut parser = Parser::one_shot_with_options(opts);
        parser.feed(&build(b"PRDID,1.0,2.0,180.0"));

        let msg = parser.next_message().unwrap().unwrap();
        match msg {
            Nmea0183Message::Prdid(PrdidData::PitchRollHeading(prh)) => {
                assert!((prh.pitch_deg.unwrap() - 1.0).abs() < 0.01);
            }
            other => panic!("expected Prdid PRH, got {other:?}"),
        }
    }

    #[test]
    fn parser_default_options_emit_raw_for_prdid() {
        let mut parser = Parser::one_shot();
        parser.feed(&build(b"PRDID,1.0,2.0,180.0"));
        let msg = parser.next_message().unwrap().unwrap();
        match msg {
            Nmea0183Message::Prdid(PrdidData::Raw { fields }) => {
                assert_eq!(fields.len(), 3);
            }
            other => panic!("expected Prdid(Raw) with default options, got {other:?}"),
        }
    }

    #[test]
    fn parser_set_options_changes_subsequent_decodes() {
        let mut parser = Parser::one_shot();
        // First: default options — PRDID is Raw.
        parser.feed(&build(b"PRDID,1.0,2.0,180.0"));
        assert!(matches!(
            parser.next_message().unwrap().unwrap(),
            Nmea0183Message::Prdid(PrdidData::Raw { .. })
        ));

        // Reconfigure and feed another PRDID.
        parser.set_options(
            DecodeOptions::default().with_prdid_dialect(PrdidDialect::RollPitchHeading),
        );
        parser.feed(&build(b"PRDID,3.0,4.0,90.0"));
        match parser.next_message().unwrap().unwrap() {
            Nmea0183Message::Prdid(PrdidData::RollPitchHeading(rph)) => {
                assert!((rph.roll_deg.unwrap() - 3.0).abs() < 0.01);
            }
            other => panic!("expected Prdid RPH after set_options, got {other:?}"),
        }
    }

    #[test]
    fn parser_applies_psxn_layout() {
        let layout: PsxnLayout = "rphx".parse().unwrap();
        let opts = DecodeOptions::default().with_psxn_layout(layout);
        let mut parser = Parser::one_shot_with_options(opts);
        // rphx: roll, pitch, heave, x, x, x
        parser.feed(&build(b"PSXN,10,tok,0.017453,0.034907,0.5,,,"));
        match parser.next_message().unwrap().unwrap() {
            Nmea0183Message::Psxn(d) => {
                assert_eq!(d.id, Some(10));
                assert!((d.roll_deg.unwrap() - 1.0).abs() < 0.01);
                assert!((d.heave_m.unwrap() - 0.5).abs() < 0.01);
            }
            other => panic!("expected Psxn, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Error propagation
    // -----------------------------------------------------------------

    #[test]
    fn parser_propagates_envelope_checksum_error() {
        let mut parser = Parser::one_shot();
        // Intentionally wrong checksum (bytes match GPGGA,1,2 body but cksum is bogus).
        parser.feed(b"$GPGGA,1,2,3*FF");
        let err = parser.next_message().unwrap().unwrap_err();
        assert!(
            matches!(err, Nmea0183Error::Envelope(_)),
            "expected Envelope, got {err:?}"
        );
    }

    #[test]
    fn parser_propagates_decode_error_for_short_gga() {
        let mut parser = Parser::one_shot();
        // Envelope-valid $GPGGA but only 3 fields (GGA needs 14).
        parser.feed(&build(b"GPGGA,1,2,3"));
        let err = parser.next_message().unwrap().unwrap_err();
        assert!(
            matches!(
                err,
                Nmea0183Error::Decode(DecodeError::NotEnoughFields {
                    expected: 14,
                    got: 3
                })
            ),
            "got {err:?}"
        );
    }

    // -----------------------------------------------------------------
    // None when no complete sentence yet available
    // -----------------------------------------------------------------

    #[test]
    fn streaming_returns_none_on_partial_buffer() {
        let mut parser = Parser::streaming();
        parser.feed(b"$GPGGA,1,2,3"); // no '*hh' yet
        assert!(parser.next_message().is_none());
    }

    // -----------------------------------------------------------------
    // Unknown sentence types return Nmea0183Message::Unknown
    // -----------------------------------------------------------------

    #[test]
    fn parser_returns_unknown_for_unrecognised_sentence_type() {
        let mut parser = Parser::one_shot();
        parser.feed(&build(b"GPABC,1,2,3")); // ABC is not a supported type
        let msg = parser.next_message().unwrap().unwrap();
        match msg {
            Nmea0183Message::Unknown(raw) => {
                assert_eq!(raw.sentence_type, "ABC");
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // The generic wrapper works directly (not only through the enum)
    // -----------------------------------------------------------------

    #[test]
    fn generic_wrapper_works_with_one_shot() {
        let mut parser: Nmea0183Parser<OneShot> = Nmea0183Parser::new(OneShot::new());
        parser.feed(&build(b"INHDT,123.4,T"));
        match parser.next_message().unwrap().unwrap() {
            Nmea0183Message::Hdt(d) => {
                assert!((d.heading_true_deg.unwrap() - 123.4).abs() < 0.01);
            }
            other => panic!("expected Hdt, got {other:?}"),
        }
    }

    #[test]
    fn generic_wrapper_works_with_streaming() {
        let mut parser: Nmea0183Parser<Streaming> = Nmea0183Parser::default();
        parser.feed(&build(b"INHDT,123.4,T"));
        match parser.next_message().unwrap().unwrap() {
            Nmea0183Message::Hdt(d) => {
                assert!((d.heading_true_deg.unwrap() - 123.4).abs() < 0.01);
            }
            other => panic!("expected Hdt, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Inner / into_inner accessors
    // -----------------------------------------------------------------

    #[test]
    fn into_inner_returns_underlying_envelope_parser() {
        let parser: Nmea0183Parser<OneShot> = Nmea0183Parser::new(OneShot::new());
        let _one_shot: OneShot = parser.into_inner();
    }
}
