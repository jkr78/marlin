//! Single-sentence parser for datagram transports.

use alloc::vec::Vec;

use crate::{parser, Error, RawSentence, SentenceSource};

/// One-sentence-per-datagram NMEA envelope parser.
///
/// Each logical `feed` delivery is expected to contain exactly one complete
/// NMEA 0183 sentence, with or without a terminator. This fits UDP
/// datagram transports where the frame itself delimits the sentence.
/// [`feed`](Self::feed) may still be called multiple times before
/// [`next_sentence`](SentenceSource::next_sentence) — fragments are
/// concatenated — to accommodate rare cases where a single datagram arrives
/// across more than one read.
///
/// Once `next_sentence` returns `Some(_)`, the internal buffer is marked as
/// consumed; the next `feed` clears it and begins accumulating a fresh
/// sentence. This matches PRD §E5.
///
/// See [`Streaming`](crate::Streaming) for the byte-stream (TCP / serial)
/// counterpart, and [`Parser`](crate::Parser) for a runtime-dispatch
/// wrapper over both modes.
#[derive(Debug)]
pub struct OneShot {
    buf: Vec<u8>,
    /// `true` once `next_sentence` has delivered a definitive result for
    /// the current buffer contents; the subsequent `feed` clears the
    /// buffer before appending.
    yielded: bool,
}

impl OneShot {
    /// Create a parser with a small default initial capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Create a parser with a specified initial buffer capacity. Useful
    /// when the caller knows the typical sentence size and wants to avoid
    /// early reallocations.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
            yielded: false,
        }
    }
}

impl Default for OneShot {
    fn default() -> Self {
        Self::new()
    }
}

impl SentenceSource for OneShot {
    type Item<'a>
        = RawSentence<'a>
    where
        Self: 'a;

    fn feed(&mut self, bytes: &[u8]) {
        if self.yielded {
            self.buf.clear();
            self.yielded = false;
        }
        self.buf.extend_from_slice(bytes);
    }

    fn next_sentence(&mut self) -> Option<Result<Self::Item<'_>, Error>> {
        if self.yielded {
            // Already answered for the current buffer; waiting for the next
            // feed to reset. Returning `None` here (rather than re-parsing)
            // matches the "one shot per feed" semantic of this impl.
            return None;
        }

        match buffer_status(&self.buf) {
            BufferStatus::Empty | BufferStatus::Partial => None,
            BufferStatus::NoStart => {
                self.yielded = true;
                Some(Err(Error::MissingStartDelimiter))
            }
            BufferStatus::MalformedTag => {
                self.yielded = true;
                Some(Err(Error::MalformedTagBlock))
            }
            BufferStatus::Complete { body_end } => {
                self.yielded = true;
                // Reborrow buf immutably to carry the lifetime into the
                // returned RawSentence. The `yielded` flag is set above, so
                // subsequent calls return `None` until the caller feeds.
                let trimmed = parser::strip_terminator(&self.buf);
                let slice = trimmed.get(..body_end).unwrap_or(trimmed);
                Some(parser::parse_sentence(slice))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferStatus {
    /// Nothing in the buffer yet.
    Empty,
    /// First non-empty byte is none of `$`, `!`, `\`.
    NoStart,
    /// Has a plausible start but not yet enough bytes for the sentence's
    /// `*hh` checksum (or for the TAG block's closing `\`).
    Partial,
    /// Buffer began with a TAG block (`\...*hh\`) that was structurally
    /// valid but not immediately followed by `$`/`!`. In single-datagram
    /// semantics this is surfaced as an error.
    MalformedTag,
    /// Buffer holds at least one complete sentence (with any TAG prefix);
    /// `body_end` is the index (into the terminator-stripped view) one
    /// past the sentence's last hex digit.
    Complete { body_end: usize },
}

fn buffer_status(buf: &[u8]) -> BufferStatus {
    let trimmed = parser::strip_terminator(buf);
    if trimmed.is_empty() {
        return BufferStatus::Empty;
    }
    match trimmed.first() {
        Some(&b'$' | &b'!' | &b'\\') => {}
        _ => return BufferStatus::NoStart,
    }

    // If a TAG block is present, compute where the sentence begins.
    let sentence_start = if trimmed.first() == Some(&b'\\') {
        let after_open = trimmed.get(1..).unwrap_or(&[]);
        let Some(close_off) = after_open.iter().position(|&b| b == b'\\') else {
            return BufferStatus::Partial;
        };
        close_off.saturating_add(2)
    } else {
        0
    };

    match trimmed.get(sentence_start) {
        Some(&b'$' | &b'!') => {}
        None => return BufferStatus::Partial,
        Some(_) => return BufferStatus::MalformedTag,
    }

    // Find the sentence's '*'.
    let after = trimmed
        .get(sentence_start.saturating_add(1)..)
        .unwrap_or(&[]);
    let Some(star_off) = after.iter().position(|&b| b == b'*') else {
        return BufferStatus::Partial;
    };
    let star_abs = sentence_start.saturating_add(1).saturating_add(star_off);
    let need = star_abs.saturating_add(3);
    if trimmed.len() >= need {
        BufferStatus::Complete { body_end: need }
    } else {
        BufferStatus::Partial
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::testing::{
        build_sentence, build_with_bad_tag_checksum, build_with_delim_and_term,
        build_with_lowercase_checksum, build_with_tag, build_with_tag_and_terminator,
        build_with_terminator, build_with_wrong_checksum,
    };

    // -----------------------------------------------------------------
    // Happy path + buffer lifecycle
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_parses_gga_happy_path_no_terminator() {
        let body = b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,";
        let bytes = build_sentence(body);

        let mut parser = OneShot::new();
        parser.feed(&bytes);

        let sentence = parser
            .next_sentence()
            .expect("parser yielded a result")
            .expect("sentence parsed cleanly");

        assert_eq!(sentence.start_delimiter, b'$');
        assert_eq!(sentence.talker, Some(*b"GP"));
        assert_eq!(sentence.sentence_type, "GGA");
        assert!(sentence.checksum_ok);
        assert!(sentence.tag_block.is_none());
        assert_eq!(sentence.raw, bytes.as_slice());

        assert_eq!(sentence.fields.len(), 14);
        assert_eq!(sentence.fields[0], b"123519");
        assert_eq!(sentence.fields[1], b"4807.038");
        assert_eq!(sentence.fields[2], b"N");
        assert_eq!(sentence.fields[12], b"");
        assert_eq!(sentence.fields[13], b"");
    }

    #[test]
    fn empty_buffer_yields_none() {
        let mut parser = OneShot::new();
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn after_yielded_sentence_returns_none_until_feed() {
        let bytes = build_sentence(b"GPGGA,1,2,3");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        assert!(parser.next_sentence().is_some());
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn partial_buffer_yields_none_awaiting_more_bytes() {
        let mut parser = OneShot::new();
        parser.feed(b"$GPGGA,1,2,3");
        assert!(parser.next_sentence().is_none());
        assert!(parser.next_sentence().is_none());
    }

    // -----------------------------------------------------------------
    // Terminator variants (PRD T3: CRLF, LF, CR, none)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_parses_sentence_with_crlf_terminator() {
        let bytes = build_with_terminator(b"INHDT,123.4,T", b"\r\n");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.talker, Some(*b"IN"));
        assert_eq!(s.sentence_type, "HDT");
        // raw must NOT include the terminator.
        assert!(!s.raw.ends_with(b"\r\n"));
        assert_eq!(s.raw.len(), bytes.len() - 2);
    }

    #[test]
    fn one_shot_parses_sentence_with_lf_only_terminator() {
        let bytes = build_with_terminator(b"GPVTG,0.0,T,0.0,M,0.0,N,0.0,K,A", b"\n");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "VTG");
        assert_eq!(s.raw.len(), bytes.len() - 1);
    }

    #[test]
    fn one_shot_parses_sentence_with_cr_only_terminator() {
        let bytes = build_with_terminator(b"GPGGA,1,2,3", b"\r");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
        assert_eq!(s.raw.len(), bytes.len() - 1);
    }

    // -----------------------------------------------------------------
    // Checksum handling (PRD E2: case-insensitive hex; bad checksum is Err)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_accepts_lowercase_hex_checksum() {
        let bytes = build_with_lowercase_checksum(b"GPGGA,1,2,3");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert!(s.checksum_ok);
        assert_eq!(s.sentence_type, "GGA");
    }

    #[test]
    fn one_shot_rejects_sentence_with_bad_checksum() {
        let bytes = build_with_wrong_checksum(b"GPGGA,1,2,3");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        match parser.next_sentence().unwrap() {
            Err(Error::ChecksumMismatch { expected, found }) => {
                assert_ne!(expected, found);
            }
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }
    }

    #[test]
    fn one_shot_yields_error_on_non_hex_checksum_digits() {
        // Manually craft: valid body + '*' + non-hex digits
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"$GPGGA,1,2,3*ZZ");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        // buffer_status requires 3 bytes after '*' to consider the sentence
        // "complete", so 'ZZ' triggers a completeness positive -> hand off
        // to parse_sentence, which rejects the non-hex digits.
        match parser.next_sentence().unwrap() {
            Err(Error::InvalidChecksumDigits) => {}
            other => panic!("expected InvalidChecksumDigits, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Start-delimiter handling ($ vs !, and garbage)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_accepts_bang_start_delimiter_for_encapsulation() {
        let bytes =
            build_with_delim_and_term(b'!', b"AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0", b"");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.start_delimiter, b'!');
        assert_eq!(s.talker, Some(*b"AI"));
        assert_eq!(s.sentence_type, "VDM");
    }

    #[test]
    fn one_shot_yields_error_when_buffer_does_not_start_with_delimiter() {
        let mut parser = OneShot::new();
        parser.feed(b"garbage bytes that do not start with $ or !");
        match parser.next_sentence().unwrap() {
            Err(Error::MissingStartDelimiter) => {}
            other => panic!("expected MissingStartDelimiter, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Field structure (empty fields preserved; no-field sentences)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_preserves_consecutive_empty_fields() {
        // Four explicit empty fields between values.
        let bytes = build_sentence(b"GPGGA,A,,,,,B");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.fields.len(), 6);
        assert_eq!(s.fields[0], b"A");
        assert_eq!(s.fields[1], b"");
        assert_eq!(s.fields[2], b"");
        assert_eq!(s.fields[3], b"");
        assert_eq!(s.fields[4], b"");
        assert_eq!(s.fields[5], b"B");
    }

    #[test]
    fn one_shot_parses_sentence_with_no_fields() {
        // $GPZDA*XX is legal though unusual: no comma, no payload.
        let bytes = build_sentence(b"GPZDA");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.talker, Some(*b"GP"));
        assert_eq!(s.sentence_type, "ZDA");
        assert!(s.fields.is_empty());
    }

    // -----------------------------------------------------------------
    // Multi-feed accumulation (PRD E5)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_accumulates_across_multiple_feed_calls() {
        let full = build_sentence(b"GPGGA,123519,4807.038,N");
        let split_at = full.len() / 2;
        let (head, tail) = full.split_at(split_at);

        let mut parser = OneShot::new();
        parser.feed(head);
        assert!(
            parser.next_sentence().is_none(),
            "partial feed should yield None"
        );
        parser.feed(tail);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
        assert_eq!(s.fields.len(), 3);
    }

    // -----------------------------------------------------------------
    // Proprietary ($P...) sentence handling
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_parses_psxn_proprietary_with_no_talker() {
        let bytes = build_sentence(b"PSXN,23,1.2,3.4,5.6,7.8");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        // Proprietary: no standardized talker/type split.
        assert_eq!(s.talker, None);
        assert_eq!(s.sentence_type, "PSXN");
        assert_eq!(s.fields.len(), 5);
        assert_eq!(s.fields[0], b"23"); // PSXN subtype
    }

    #[test]
    fn one_shot_parses_prdid_proprietary_with_no_talker() {
        let bytes = build_sentence(b"PRDID,-1.5,2.5,123.4");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.talker, None);
        assert_eq!(s.sentence_type, "PRDID");
        assert_eq!(s.fields.len(), 3);
        assert_eq!(s.fields[0], b"-1.5");
    }

    #[test]
    fn one_shot_encapsulation_bang_never_treated_as_proprietary() {
        // `!P...` is not a thing in NMEA but verify our rule is scoped
        // to `$` specifically. We construct `!PAIVX,...` (nonsense but
        // syntactically valid) and expect normal talker/type split.
        let bytes = build_with_delim_and_term(b'!', b"PAIVX,1,2", b"");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.start_delimiter, b'!');
        assert_eq!(s.talker, Some(*b"PA")); // standard split, NOT proprietary
        assert_eq!(s.sentence_type, "IVX");
    }

    // -----------------------------------------------------------------
    // TAG block handling (PRD E4, decision 7)
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_parses_sentence_with_tag_block_valid_checksum() {
        let bytes = build_with_tag(b"c:1577836800", b"GPGGA,1,2,3");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.tag_block, Some(b"c:1577836800".as_slice()));
        assert_eq!(s.sentence_type, "GGA");
        // raw must NOT include the TAG block.
        assert!(s.raw.starts_with(b"$GPGGA"));
        assert!(s.checksum_ok);
    }

    #[test]
    fn one_shot_accepts_tag_block_with_invalid_tag_checksum() {
        // PRD decision 7: bad TAG checksum is advisory, not fatal.
        let bytes = build_with_bad_tag_checksum(b"c:12345", b"GPGGA,1,2,3");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.tag_block, Some(b"c:12345".as_slice()));
        assert_eq!(s.sentence_type, "GGA");
    }

    #[test]
    fn one_shot_parses_tag_block_with_crlf_terminator() {
        let bytes = build_with_tag_and_terminator(b"c:9999", b"INHDT,100.0,T", b"\r\n");
        let mut parser = OneShot::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.tag_block, Some(b"c:9999".as_slice()));
        assert_eq!(s.sentence_type, "HDT");
    }

    #[test]
    fn one_shot_returns_none_when_tag_block_is_not_yet_closed() {
        // No closing '\' yet — still accumulating. next_sentence returns
        // None (partial), not an error.
        let mut parser = OneShot::new();
        parser.feed(b"\\tag*ABnever_closed");
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn one_shot_yields_malformed_tag_when_no_sentence_follows() {
        // Structurally valid TAG, but the byte after the closing '\' is
        // not '$' or '!'. The OneShot contract says: give the caller a
        // definitive answer (MalformedTagBlock) rather than wait forever.
        let mut parser = OneShot::new();
        let mut bytes = Vec::new();
        bytes.push(b'\\');
        bytes.extend_from_slice(b"c:1");
        bytes.push(b'*');
        let cksum = b"c:1".iter().fold(0u8, |acc, &x| acc ^ x);
        let nibble = |v: u8| if v < 10 { b'0' + v } else { b'A' + (v - 10) };
        bytes.push(nibble(cksum >> 4));
        bytes.push(nibble(cksum & 0x0F));
        bytes.push(b'\\');
        bytes.extend_from_slice(b"not-a-sentence");
        parser.feed(&bytes);
        match parser.next_sentence().unwrap() {
            Err(Error::MalformedTagBlock) => {}
            other => panic!("expected MalformedTagBlock, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Reset/recovery
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_resets_after_yielding_error() {
        // First: a bad-checksum sentence. Parser yields ChecksumMismatch and
        // marks itself yielded. Next feed should start cleanly.
        let mut parser = OneShot::new();
        parser.feed(&build_with_wrong_checksum(b"GPGGA,1,2,3"));
        match parser.next_sentence().unwrap() {
            Err(Error::ChecksumMismatch { .. }) => {}
            other => panic!("expected ChecksumMismatch first, got {other:?}"),
        }

        // Second: a clean sentence via a new feed.
        parser.feed(&build_sentence(b"INHDT,100.0,T"));
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "HDT");
        assert!(s.checksum_ok);
    }
}
