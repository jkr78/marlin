//! Typed-message parser wrapper — ergonomic layer over the envelope.
//!
//! The envelope crate's parsers emit [`RawSentence`] values; this
//! module wraps them so callers get typed [`Nmea0183Message`] values
//! directly, one `feed` / `next_message` loop instead of two.

use marlin_nmea_envelope::{RawSentence, SentenceSource};

use crate::{decode_with, DecodeError, DecodeOptions, Nmea0183Message};

// ---------------------------------------------------------------------------
// Unified error type
// ---------------------------------------------------------------------------

/// Errors surfacing from [`Nmea0183Parser::next_message`].
///
/// The two variants distinguish **where** the failure happened:
///
/// - [`Self::Envelope`] — framing, checksum, TAG block, buffer overflow.
///   The sentence bytes are malformed at the NMEA 0183 envelope layer.
/// - [`Self::Decode`] — the envelope parsed cleanly, but the sentence
///   has fewer fields than its typed decoder's floor. A field's value
///   never fails the sentence; it decodes to a
///   [`FieldState`](crate::FieldState).
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
    /// Typed-decode failure: fewer fields than the sentence's floor.
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
/// To choose the source mode at runtime, wrap
/// [`marlin_nmea_envelope::Parser`] (re-exported as [`crate::Parser`]):
/// it is a [`SentenceSource`] like [`crate::OneShot`] and
/// [`crate::Streaming`], and the whole API of this type is available
/// on it.
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
    ///   but the sentence has fewer fields than its decoder's floor.
    ///   Same recovery semantic.
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
    use marlin_nmea_envelope::{OneShot, Streaming};

    use super::*;
    use crate::testing::build;
    use crate::{FieldState, GgaFixQuality, PrdidData, PrdidDialect, PsxnLayout};
    use alloc::vec::Vec;

    // -----------------------------------------------------------------
    // End-to-end: bytes → typed message
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_decodes_gga_bytes_into_typed_message() {
        let mut parser = Nmea0183Parser::new(OneShot::new());
        parser.feed(b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47");
        let msg = parser.next_message().unwrap().unwrap();
        let gga = match msg {
            Nmea0183Message::Gga(d) => d,
            other => panic!("expected Gga, got {other:?}"),
        };
        assert_eq!(gga.talker, Some(*b"GP"));
        assert_eq!(gga.satellites_used, FieldState::Value(8));
        assert_eq!(gga.fix_quality, FieldState::Value(GgaFixQuality::GpsFix));
    }

    #[test]
    fn streaming_decodes_multiple_messages_from_one_feed() {
        let mut parser = Nmea0183Parser::new(Streaming::new());
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
    // Options propagation: Nmea0183Parser honors DecodeOptions
    // -----------------------------------------------------------------

    #[test]
    fn parser_applies_configured_prdid_dialect() {
        let opts = DecodeOptions::default().with_prdid_dialect(PrdidDialect::PitchRollHeading);
        let mut parser = Nmea0183Parser::with_options(OneShot::new(), opts);
        parser.feed(&build(b"PRDID,1.0,2.0,180.0"));

        let msg = parser.next_message().unwrap().unwrap();
        match msg {
            Nmea0183Message::Prdid(PrdidData::PitchRollHeading(prh)) => {
                assert!((prh.pitch_deg.value().unwrap() - 1.0).abs() < 0.01);
            }
            other => panic!("expected Prdid PRH, got {other:?}"),
        }
    }

    #[test]
    fn parser_default_options_emit_raw_for_prdid() {
        let mut parser = Nmea0183Parser::new(OneShot::new());
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
        let mut parser = Nmea0183Parser::new(OneShot::new());
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
                assert!((rph.roll_deg.value().unwrap() - 3.0).abs() < 0.01);
            }
            other => panic!("expected Prdid RPH after set_options, got {other:?}"),
        }
    }

    #[test]
    fn parser_applies_psxn_layout() {
        let layout: PsxnLayout = "rphx".parse().unwrap();
        let opts = DecodeOptions::default().with_psxn_layout(layout);
        let mut parser = Nmea0183Parser::with_options(OneShot::new(), opts);
        // rphx: roll, pitch, heave, x, x, x
        parser.feed(&build(b"PSXN,10,tok,0.017453,0.034907,0.5,,,"));
        match parser.next_message().unwrap().unwrap() {
            Nmea0183Message::Psxn(d) => {
                assert_eq!(d.id, FieldState::Value(10));
                assert!((d.roll_deg.value().unwrap() - 1.0).abs() < 0.01);
                assert!((d.heave_m.value().unwrap() - 0.5).abs() < 0.01);
            }
            other => panic!("expected Psxn, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Error propagation
    // -----------------------------------------------------------------

    #[test]
    fn parser_propagates_envelope_checksum_error() {
        let mut parser = Nmea0183Parser::new(OneShot::new());
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
        let mut parser = Nmea0183Parser::new(OneShot::new());
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
        let mut parser = Nmea0183Parser::new(Streaming::new());
        parser.feed(b"$GPGGA,1,2,3"); // no '*hh' yet
        assert!(parser.next_message().is_none());
    }

    // -----------------------------------------------------------------
    // Unknown sentence types return Nmea0183Message::Unknown
    // -----------------------------------------------------------------

    #[test]
    fn parser_returns_unknown_for_unrecognised_sentence_type() {
        let mut parser = Nmea0183Parser::new(OneShot::new());
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
                assert!((d.heading_true_deg.value().unwrap() - 123.4).abs() < 0.01);
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
                assert!((d.heading_true_deg.value().unwrap() - 123.4).abs() < 0.01);
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

    #[test]
    fn decode_with_routes_radar_sentences() {
        use crate::{decode, Nmea0183Message};
        let hdg = crate::testing::build(b"HCHDG,98.3,0.0,E,12.6,W");
        assert!(matches!(
            decode(&crate::testing::parse_raw(&hdg)).unwrap(),
            Nmea0183Message::Hdg(_)
        ));
        let ttm = crate::testing::build(b"RATTM,1,1.0,2.0,T,3.0,4.0,T,5.0,6.0,N,x,T,");
        assert!(matches!(
            decode(&crate::testing::parse_raw(&ttm)).unwrap(),
            Nmea0183Message::Ttm(_)
        ));
        let tll = crate::testing::build(b"RATLL,1,5000.00,N,00500.00,E");
        assert!(matches!(
            decode(&crate::testing::parse_raw(&tll)).unwrap(),
            Nmea0183Message::Tll(_)
        ));
    }

    // -----------------------------------------------------------------
    // Source mode chosen at runtime: wrap the envelope `Parser` enum
    // -----------------------------------------------------------------

    #[test]
    fn wraps_envelope_parser_enum_for_runtime_source_mode() {
        let use_streaming = true;
        let source = if use_streaming {
            marlin_nmea_envelope::Parser::streaming()
        } else {
            marlin_nmea_envelope::Parser::one_shot()
        };
        let opts = DecodeOptions::default().with_prdid_dialect(PrdidDialect::PitchRollHeading);
        let mut parser = Nmea0183Parser::with_options(source, opts);
        let mut combined = build(b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&build(b"PRDID,1.0,2.0,180.0"));
        combined.extend_from_slice(b"\r\n");
        parser.feed(&combined);

        assert!(matches!(
            parser.next_message().unwrap().unwrap(),
            Nmea0183Message::Gga(_)
        ));
        assert!(matches!(
            parser.next_message().unwrap().unwrap(),
            Nmea0183Message::Prdid(PrdidData::PitchRollHeading(_))
        ));
        assert!(parser.next_message().is_none());
    }
}
