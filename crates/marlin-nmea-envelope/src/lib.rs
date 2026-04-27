//! # marlin-nmea-envelope
//!
//! Sans-I/O NMEA 0183 envelope parser.
//!
//! This crate handles the transport-independent framing of NMEA 0183
//! sentences: locating sentence boundaries, verifying the XOR checksum,
//! splitting fields, and recognizing optional NMEA 4.10 TAG blocks. It
//! performs **no I/O** — bytes are pushed in by the caller via
//! [`SentenceSource::feed`], and parsed [`RawSentence`] values are pulled
//! out via [`SentenceSource::next_sentence`].
//!
//! Higher-level crates (`marlin-nmea-0183`, `marlin-ais`) build typed
//! decoders on top of this envelope.
//!
//! # Modes
//!
//! Two sentence sources ship with this crate, both satisfying the same
//! [`SentenceSource`] trait:
//!
//! - [`OneShot`] — assumes each `feed` call (or a small number of them)
//!   delivers exactly one complete sentence. Designed for datagram
//!   transports (UDP) where framing is provided by the transport itself,
//!   and terminators are frequently absent.
//!
//! - [`Streaming`] — maintains an internal buffer, scans for sentence
//!   boundaries, and yields sentences as they complete. Designed for
//!   byte-stream transports (TCP, serial).
//!
//! Both go through the *same* nom-based parser core; the difference is
//! buffer management, not parsing logic.
//!
//! # Examples
//!
//! **One-shot / UDP-style — no terminator required:**
//!
//! ```
//! use marlin_nmea_envelope::{OneShot, SentenceSource};
//!
//! // `$GPGGA,123519*77` — checksum 0x77 is XOR of the bytes between `$` and `*`.
//! let mut parser = OneShot::new();
//! parser.feed(b"$GPGGA,123519*77");
//!
//! let sentence = parser.next_sentence().unwrap().unwrap();
//! assert_eq!(sentence.talker, Some(*b"GP"));
//! assert_eq!(sentence.sentence_type, "GGA");
//! assert!(sentence.checksum_ok);
//! ```
//!
//! **Streaming / TCP-style — multiple sentences per feed, terminators consumed:**
//!
//! ```
//! use marlin_nmea_envelope::{Streaming, SentenceSource};
//!
//! let mut parser = Streaming::new();
//! parser.feed(b"$GPGGA,123519*77\r\n$GPGGA,123519*77\r\n");
//!
//! // Can only hold one borrowed sentence at a time — that's the GAT's
//! // contract. Inspect each, then loop for the next.
//! assert_eq!(parser.next_sentence().unwrap().unwrap().sentence_type, "GGA");
//! assert_eq!(parser.next_sentence().unwrap().unwrap().sentence_type, "GGA");
//! assert!(parser.next_sentence().is_none());
//! ```
//!
//! # Runtime dispatch without `dyn`
//!
//! The [`SentenceSource`] trait uses a GAT for zero-copy borrows (see the
//! trait's documentation) and therefore is not object-safe. Callers who
//! need to choose the mode at runtime (e.g. from a config file) use the
//! [`Parser`] enum, which provides the same `feed` / `next_sentence`
//! shape with zero-cost static dispatch:
//!
//! ```
//! use marlin_nmea_envelope::Parser;
//!
//! // Mode chosen at runtime (e.g. from config).
//! let use_streaming = true;
//! let mut parser = if use_streaming {
//!     Parser::streaming()
//! } else {
//!     Parser::one_shot()
//! };
//!
//! parser.feed(b"$GPGGA,123519*77\r\n");
//! let sentence = parser.next_sentence().unwrap().unwrap();
//! assert_eq!(sentence.sentence_type, "GGA");
//! ```
//!
//! # Feature flags
//!
//! - `tracing` *(off by default)* — emits `tracing` events for discarded
//!   garbage, buffer overflows, and TAG block checksum mismatches. Add a
//!   subscriber in the host application to see them.
//!
//! # Specification
//!
//! The normative requirements are in `.docs/prd.txt` §5.1. Key
//! architectural decisions (§9) relevant to this crate:
//!
//! - Sans-I/O: no sockets, no runtime, no file handles.
//! - One parser core shared by both modes.
//! - `complete` nom parsers only — no `Err::Incomplete` propagation.
//! - Zero-copy borrows; no allocation on the sentence hot path.
//! - TAG block checksum mismatches are advisory, not fatal (decision 7).

#![doc(html_root_url = "https://docs.rs/marlin-nmea-envelope/0.1.0")]
#![no_std]

extern crate alloc;

mod error;
mod one_shot;
mod parser;
mod sentence;
mod source;
mod streaming;

#[cfg(test)]
pub(crate) mod testing;

pub use error::Error;
pub use one_shot::OneShot;
pub use sentence::RawSentence;
pub use source::SentenceSource;
pub use streaming::{Streaming, DEFAULT_MAX_BUFFER_SIZE};

/// Parse a single complete NMEA 0183 sentence from a byte slice.
///
/// This is a lightweight alternative to [`OneShot`] when the caller has
/// a pre-framed byte slice in hand (e.g. one UDP datagram, or a slice
/// extracted from a log file). It accepts:
///
/// - Optional TAG block prefix (`\...*hh\`).
/// - Either `$` or `!` as the sentence start.
/// - An optional trailing terminator (`\r\n`, `\n`, or `\r`).
///
/// The returned [`RawSentence`] borrows directly from `bytes` — no
/// copy, no parser state. This is useful for typed decoders in higher
/// crates that have already obtained bytes via their own means and
/// just want the envelope's framing guarantees.
///
/// # Errors
///
/// Any envelope-level failure — missing start delimiter, missing `*`,
/// bad hex digits, checksum mismatch, malformed TAG block — surfaces
/// as [`Error`].
pub fn parse(bytes: &[u8]) -> Result<RawSentence<'_>, Error> {
    let stripped = parser::strip_terminator(bytes);
    parser::parse_sentence(stripped)
}

/// Runtime-dispatch wrapper over the two sentence-source implementations.
///
/// Use this when the parser mode is a configuration choice — e.g. "if
/// the transport is UDP, use one-shot; if TCP, use streaming" — and the
/// caller does not want generics infecting their signatures.
///
/// The enum delegates [`feed`](Self::feed) and
/// [`next_sentence`](Self::next_sentence) to the active variant. Dispatch
/// is a match (static, inlined); there is no heap allocation and no
/// vtable lookup. This is the recommended pattern for callers that need
/// runtime flexibility but not trait-object genericity.
///
/// # Example
///
/// ```
/// use marlin_nmea_envelope::Parser;
///
/// let mut parser = Parser::streaming();
/// parser.feed(b"$GPGGA,123519*77\r\n");
/// let sentence = parser.next_sentence().unwrap().unwrap();
/// assert_eq!(sentence.sentence_type, "GGA");
/// ```
#[derive(Debug)]
pub enum Parser {
    /// Single-sentence parser. See [`OneShot`].
    OneShot(OneShot),
    /// Buffered stream parser. See [`Streaming`].
    Streaming(Streaming),
}

impl Parser {
    /// Construct a [`Parser::OneShot`] with default initial capacity.
    #[must_use]
    pub fn one_shot() -> Self {
        Self::OneShot(OneShot::new())
    }

    /// Construct a [`Parser::Streaming`] with the default maximum buffer
    /// size ([`DEFAULT_MAX_BUFFER_SIZE`]).
    #[must_use]
    pub fn streaming() -> Self {
        Self::Streaming(Streaming::new())
    }

    /// Construct a [`Parser::Streaming`] with a caller-specified maximum
    /// buffer size.
    #[must_use]
    pub fn streaming_with_capacity(max_size: usize) -> Self {
        Self::Streaming(Streaming::with_capacity(max_size))
    }

    /// Push raw bytes into the active parser. See
    /// [`SentenceSource::feed`] for the per-mode semantics.
    pub fn feed(&mut self, bytes: &[u8]) {
        match self {
            Self::OneShot(p) => p.feed(bytes),
            Self::Streaming(p) => p.feed(bytes),
        }
    }

    /// Pull the next complete sentence out of the active parser. See
    /// [`SentenceSource::next_sentence`] for the per-mode semantics.
    pub fn next_sentence(&mut self) -> Option<Result<RawSentence<'_>, Error>> {
        match self {
            Self::OneShot(p) => p.next_sentence(),
            Self::Streaming(p) => p.next_sentence(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod parser_enum_tests {
    use super::Parser;
    use crate::testing::build_sentence;

    #[test]
    fn parser_enum_one_shot_constructs_and_parses() {
        let bytes = build_sentence(b"GPGGA,1,2,3");
        let mut parser = Parser::one_shot();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
    }

    #[test]
    fn parser_enum_streaming_constructs_and_parses_multiple() {
        let s1 = build_sentence(b"GPGGA,a");
        let s2 = build_sentence(b"INHDT,b,T");
        let mut combined = s1.clone();
        combined.extend_from_slice(&s2);

        let mut parser = Parser::streaming();
        parser.feed(&combined);
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "GGA"
        );
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "HDT"
        );
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn parser_enum_streaming_with_capacity_honors_max_size() {
        let mut parser = Parser::streaming_with_capacity(64);
        parser.feed(&[b'?'; 200]);
        match parser.next_sentence().unwrap() {
            Err(crate::Error::BufferOverflow) => {}
            other => panic!("expected BufferOverflow, got {other:?}"),
        }
    }

    #[test]
    fn parser_enum_same_call_shape_for_both_modes() {
        // The same caller code drives both modes through the enum with no
        // branching at the call site — this is the whole point of the
        // runtime-dispatch wrapper.
        fn drive(parser: &mut Parser, bytes: &[u8]) -> Option<&'static str> {
            parser.feed(bytes);
            parser
                .next_sentence()
                .and_then(Result::ok)
                .map(|s| match s.sentence_type {
                    "GGA" => "gga",
                    "HDT" => "hdt",
                    _ => "other",
                })
        }

        let bytes = build_sentence(b"GPGGA,1");
        assert_eq!(drive(&mut Parser::one_shot(), &bytes), Some("gga"));
        assert_eq!(drive(&mut Parser::streaming(), &bytes), Some("gga"));
    }
}
