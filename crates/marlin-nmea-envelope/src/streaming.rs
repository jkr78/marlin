//! Buffered parser for byte-stream transports (TCP, serial).

use alloc::vec::Vec;

use crate::{parser, Error, RawSentence, SentenceSource};

/// Default maximum buffer size (64 KiB), per PRD §N1.
pub const DEFAULT_MAX_BUFFER_SIZE: usize = 64 * 1024;

/// Compaction trigger: when `cursor` passes `max_size / COMPACT_DIVISOR`,
/// the next [`feed`](SentenceSource::feed) drains consumed bytes. Setting
/// this to `2` means "compact when the cursor is past half of max size";
/// lower values (e.g. `4`) compact sooner at the cost of more memcpy work.
const COMPACT_DIVISOR: usize = 2;

/// Initial capacity used when `max_size` exceeds what most sentences need.
/// Starting at 4 KiB avoids reserving the full 64 KiB up front when most
/// feeds are tiny.
const INITIAL_CAPACITY_HINT: usize = 4096;

/// Streaming NMEA envelope parser for byte-stream transports.
///
/// Maintains an internal buffer, scans it for sentence boundaries (start
/// delimiter + `*hh` ± optional terminator), and yields sentences as they
/// complete. Garbage bytes before a sentence start (or a buffer with no
/// start delimiter at all) are discarded. When the read cursor passes
/// roughly half of the configured maximum buffer size, the buffer is
/// compacted on the next `feed` — this keeps memory bounded without
/// per-call quadratic rescanning (PRD §N2).
///
/// If a [`feed`](SentenceSource::feed) call would push the buffer past
/// the configured maximum size, the oldest bytes are discarded to make
/// room for the new bytes and an [`Error::BufferOverflow`] is queued for
/// the next [`next_sentence`](SentenceSource::next_sentence) call (PRD
/// §N1). Parsing then continues normally.
///
/// The shared nom parser core is reused from the same implementation
/// used by [`OneShot`](crate::OneShot) — there is one parser in this
/// crate, not two.
///
/// Uses a plain `Vec<u8>` for the internal buffer. `bytes::BytesMut` would
/// be a reasonable alternative with slightly cheaper `split_to`, but for
/// typical feed sizes the savings are in the single-digit-percent range
/// and `Vec<u8>`'s simpler API is worth it here.
#[derive(Debug)]
pub struct Streaming {
    buf: Vec<u8>,
    cursor: usize,
    max_size: usize,
    /// Set when a feed call dropped bytes to stay under `max_size`. The
    /// next `next_sentence` consumes it and yields [`Error::BufferOverflow`]
    /// before resuming normal scanning.
    pending_overflow: bool,
}

impl Streaming {
    /// Create a parser with the default 64 KiB maximum buffer size.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_BUFFER_SIZE)
    }

    /// Create a parser with a specified maximum buffer size.
    ///
    /// The initial allocation is smaller than `max_size`; the buffer grows
    /// on demand and will not exceed `max_size`. If a feed would push the
    /// buffer over `max_size`, the oldest bytes are dropped and an
    /// [`Error::BufferOverflow`] is emitted on the next
    /// [`next_sentence`](SentenceSource::next_sentence).
    #[must_use]
    pub fn with_capacity(max_size: usize) -> Self {
        let initial = core::cmp::min(max_size, INITIAL_CAPACITY_HINT);
        Self {
            buf: Vec::with_capacity(initial),
            cursor: 0,
            max_size,
            pending_overflow: false,
        }
    }

    fn compact(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // drain is O(n) in the tail length; with the half-max threshold,
        // amortised cost per byte of throughput stays O(1).
        self.buf.drain(..self.cursor);
        self.cursor = 0;
    }
}

impl Default for Streaming {
    fn default() -> Self {
        Self::new()
    }
}

impl SentenceSource for Streaming {
    type Item<'a>
        = RawSentence<'a>
    where
        Self: 'a;

    fn feed(&mut self, bytes: &[u8]) {
        if self.cursor > self.max_size.saturating_div(COMPACT_DIVISOR) {
            self.compact();
        }
        let combined = self.buf.len().saturating_add(bytes.len());
        if combined <= self.max_size {
            self.buf.extend_from_slice(bytes);
            return;
        }

        // Overflow path: drop the oldest bytes (buf first, then the front
        // of `bytes`) so the post-feed size equals max_size exactly.
        self.pending_overflow = true;
        let mut to_drop = combined.saturating_sub(self.max_size);

        let from_buf = core::cmp::min(to_drop, self.buf.len());
        if from_buf > 0 {
            self.buf.drain(..from_buf);
            self.cursor = self.cursor.saturating_sub(from_buf);
        }
        to_drop = to_drop.saturating_sub(from_buf);

        let bytes_tail_start = core::cmp::min(to_drop, bytes.len());
        let tail = bytes.get(bytes_tail_start..).unwrap_or(&[]);
        self.buf.extend_from_slice(tail);

        #[cfg(feature = "tracing")]
        tracing::warn!(
            dropped_from_buf = from_buf,
            dropped_from_feed = bytes_tail_start,
            max_size = self.max_size,
            "streaming buffer overflowed; oldest bytes discarded"
        );
    }

    fn next_sentence(&mut self) -> Option<Result<Self::Item<'_>, Error>> {
        if self.pending_overflow {
            self.pending_overflow = false;
            return Some(Err(Error::BufferOverflow));
        }

        // Loop to absorb orphaned TAG blocks (TAG prefix not followed by
        // a sentence) as if they were plain garbage.
        loop {
            match scan(&self.buf, self.cursor) {
                ScanResult::NoneYet => {
                    #[cfg(feature = "tracing")]
                    {
                        let discarded = self.buf.len().saturating_sub(self.cursor);
                        if discarded > 0 {
                            tracing::debug!(
                                bytes = discarded,
                                "discarded trailing garbage (no start delimiter)"
                            );
                        }
                    }
                    self.cursor = self.buf.len();
                    return None;
                }
                ScanResult::NeedMore { start } => {
                    #[cfg(feature = "tracing")]
                    {
                        let discarded = start.saturating_sub(self.cursor);
                        if discarded > 0 {
                            tracing::debug!(
                                bytes = discarded,
                                "discarded garbage before sentence start"
                            );
                        }
                    }
                    self.cursor = start;
                    return None;
                }
                ScanResult::OrphanedTag { advance_to } => {
                    #[cfg(feature = "tracing")]
                    tracing::debug!("discarded TAG block not immediately followed by a sentence");
                    self.cursor = advance_to;
                    // Fall through to the next loop iteration and rescan.
                }
                ScanResult::Complete {
                    start,
                    body_end,
                    consume_end,
                } => {
                    #[cfg(feature = "tracing")]
                    {
                        let discarded = start.saturating_sub(self.cursor);
                        if discarded > 0 {
                            tracing::debug!(
                                bytes = discarded,
                                "discarded garbage before sentence start"
                            );
                        }
                    }
                    self.cursor = consume_end;
                    let slice = self.buf.get(start..body_end).unwrap_or(&[]);
                    return Some(parser::parse_sentence(slice));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanResult {
    /// `buf[from..]` contains no region start (`$`, `!`, or `\`) at all.
    NoneYet,
    /// A region start was found at `start`, but the region is not yet
    /// complete (unterminated TAG block, missing sentence `*`, or
    /// insufficient hex digits). The cursor should sit on `start`.
    NeedMore { start: usize },
    /// A TAG block parsed structurally but was not immediately followed
    /// by `$` or `!`. The caller should advance `cursor` to `advance_to`
    /// (past the closing `\`) and re-scan; in [`Streaming`] mode this
    /// is treated as silent garbage, in [`OneShot`] as a malformed-tag
    /// error.
    OrphanedTag { advance_to: usize },
    /// A complete sentence (plus optional TAG prefix) occupies
    /// `buf[start..body_end]`. `start` points at the `\` of a TAG block
    /// if present, otherwise at the sentence's `$`/`!`. `body_end`
    /// excludes the terminator; `consume_end` extends past it.
    Complete {
        start: usize,
        body_end: usize,
        consume_end: usize,
    },
}

fn scan(buf: &[u8], from: usize) -> ScanResult {
    let tail = buf.get(from..).unwrap_or(&[]);
    let Some(offset) = tail.iter().position(|&b| matches!(b, b'\\' | b'$' | b'!')) else {
        return ScanResult::NoneYet;
    };
    let region_start = from.saturating_add(offset);
    let first = buf.get(region_start).copied().unwrap_or(0);

    // If the region begins with '\', step past the TAG block to locate
    // the real sentence start. The TAG body's own '*' would otherwise be
    // mistaken for the sentence's checksum delimiter.
    let sentence_start = if first == b'\\' {
        let after_open = region_start.saturating_add(1);
        let search = buf.get(after_open..).unwrap_or(&[]);
        let Some(close_off) = search.iter().position(|&b| b == b'\\') else {
            return ScanResult::NeedMore {
                start: region_start,
            };
        };
        after_open.saturating_add(close_off).saturating_add(1)
    } else {
        region_start
    };

    // The sentence itself must begin with '$' or '!' at sentence_start.
    match buf.get(sentence_start) {
        Some(&b'$' | &b'!') => {}
        None => {
            return ScanResult::NeedMore {
                start: region_start,
            }
        }
        Some(_) => {
            return ScanResult::OrphanedTag {
                advance_to: sentence_start,
            };
        }
    }

    // Find the sentence's '*' checksum delimiter.
    let after_sent = buf.get(sentence_start.saturating_add(1)..).unwrap_or(&[]);
    let Some(star_off) = after_sent.iter().position(|&b| b == b'*') else {
        return ScanResult::NeedMore {
            start: region_start,
        };
    };
    let star_abs = sentence_start.saturating_add(1).saturating_add(star_off);

    let body_end = star_abs.saturating_add(3);
    if buf.len() < body_end {
        return ScanResult::NeedMore {
            start: region_start,
        };
    }

    let consume_end = match buf.get(body_end) {
        Some(&b'\r') if buf.get(body_end.saturating_add(1)) == Some(&b'\n') => {
            body_end.saturating_add(2)
        }
        Some(&b'\r' | &b'\n') => body_end.saturating_add(1),
        _ => body_end,
    };

    ScanResult::Complete {
        start: region_start,
        body_end,
        consume_end,
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
        build_sentence, build_with_bad_tag_checksum, build_with_lowercase_checksum, build_with_tag,
        build_with_tag_and_terminator, build_with_terminator, build_with_wrong_checksum,
    };
    use alloc::vec::Vec;

    fn concat(parts: &[&[u8]]) -> Vec<u8> {
        let cap = parts.iter().map(|p| p.len()).sum();
        let mut out = Vec::with_capacity(cap);
        for p in parts {
            out.extend_from_slice(p);
        }
        out
    }

    // -----------------------------------------------------------------
    // Slice test (happy path, two sentences in one feed)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_yields_two_sentences_from_one_feed() {
        let s1 = build_sentence(b"GPGGA,123519,4807.038,N");
        let s2 = build_sentence(b"INHDT,100.0,T");
        let combined = concat(&[&s1, &s2]);

        let mut parser = Streaming::new();
        parser.feed(&combined);

        let first = parser.next_sentence().unwrap().unwrap();
        assert_eq!(first.talker, Some(*b"GP"));
        assert_eq!(first.sentence_type, "GGA");
        assert_eq!(first.fields.len(), 3);
        assert_eq!(first.fields[0], b"123519");

        let second = parser.next_sentence().unwrap().unwrap();
        assert_eq!(second.talker, Some(*b"IN"));
        assert_eq!(second.sentence_type, "HDT");

        assert!(parser.next_sentence().is_none());
    }

    // -----------------------------------------------------------------
    // Garbage handling (PRD E6)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_discards_garbage_before_sentence_start() {
        let sentence = build_sentence(b"GPGGA,1,2,3");
        let combined = concat(&[b"some garbage bytes before\x00\xff", &sentence]);

        let mut parser = Streaming::new();
        parser.feed(&combined);

        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
        assert_eq!(s.fields.len(), 3);
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn streaming_discards_garbage_between_sentences() {
        let s1 = build_sentence(b"GPGGA,A,B");
        let s2 = build_sentence(b"INHDT,99.9,T");
        let combined = concat(&[&s1, b"\r\njunkjunkjunk\r\n", &s2]);

        let mut parser = Streaming::new();
        parser.feed(&combined);

        let first = parser.next_sentence().unwrap().unwrap();
        assert_eq!(first.sentence_type, "GGA");
        let second = parser.next_sentence().unwrap().unwrap();
        assert_eq!(second.sentence_type, "HDT");
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn streaming_with_only_garbage_yields_none() {
        let mut parser = Streaming::new();
        parser.feed(b"this buffer never contains a start delimiter");
        assert!(parser.next_sentence().is_none());
    }

    // -----------------------------------------------------------------
    // Fragmentation across feed calls
    // -----------------------------------------------------------------

    #[test]
    fn streaming_yields_sentence_split_across_two_feed_calls() {
        let full = build_sentence(b"GPGGA,123519,4807.038,N,01131.000,E");
        let split_at = full.len() / 2;
        let (head, tail) = full.split_at(split_at);

        let mut parser = Streaming::new();
        parser.feed(head);
        assert!(parser.next_sentence().is_none());
        parser.feed(tail);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
        assert_eq!(s.fields.len(), 5);
    }

    #[test]
    fn streaming_yields_sentence_split_across_many_small_feeds() {
        let full = build_sentence(b"INHDT,123.456,T");

        let mut parser = Streaming::new();
        for chunk in full.chunks(3) {
            parser.feed(chunk);
        }
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "HDT");
    }

    // -----------------------------------------------------------------
    // Terminator variants (PRD T3)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_parses_sentences_separated_by_crlf() {
        let s1 = build_with_terminator(b"GPGGA,1,2", b"\r\n");
        let s2 = build_with_terminator(b"INHDT,3.0,T", b"\r\n");
        let combined = concat(&[&s1, &s2]);
        let mut parser = Streaming::new();
        parser.feed(&combined);
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "GGA"
        );
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "HDT"
        );
    }

    #[test]
    fn streaming_parses_sentences_separated_by_lf_only() {
        let s1 = build_with_terminator(b"GPGGA,a", b"\n");
        let s2 = build_with_terminator(b"INHDT,b,T", b"\n");
        let combined = concat(&[&s1, &s2]);
        let mut parser = Streaming::new();
        parser.feed(&combined);
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "GGA"
        );
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "HDT"
        );
    }

    #[test]
    fn streaming_parses_sentences_separated_by_cr_only() {
        let s1 = build_with_terminator(b"GPGGA,a", b"\r");
        let s2 = build_with_terminator(b"INHDT,b,T", b"\r");
        let combined = concat(&[&s1, &s2]);
        let mut parser = Streaming::new();
        parser.feed(&combined);
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "GGA"
        );
        assert_eq!(
            parser.next_sentence().unwrap().unwrap().sentence_type,
            "HDT"
        );
    }

    // -----------------------------------------------------------------
    // Field semantics
    // -----------------------------------------------------------------

    #[test]
    fn streaming_preserves_empty_fields() {
        let bytes = build_sentence(b"GPGGA,A,,,,B,");
        let mut parser = Streaming::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.fields.len(), 6);
        assert_eq!(s.fields[0], b"A");
        assert_eq!(s.fields[1], b"");
        assert_eq!(s.fields[2], b"");
        assert_eq!(s.fields[3], b"");
        assert_eq!(s.fields[4], b"B");
        assert_eq!(s.fields[5], b"");
    }

    // -----------------------------------------------------------------
    // Checksum handling
    // -----------------------------------------------------------------

    #[test]
    fn streaming_accepts_lowercase_hex_checksum() {
        let bytes = build_with_lowercase_checksum(b"GPGGA,1,2");
        let mut parser = Streaming::new();
        parser.feed(&bytes);
        assert!(parser.next_sentence().unwrap().unwrap().checksum_ok);
    }

    #[test]
    fn streaming_yields_error_on_bad_checksum_and_continues_with_next() {
        let bad = build_with_wrong_checksum(b"GPGGA,1,2");
        let good = build_sentence(b"INHDT,99.0,T");
        let combined = concat(&[&bad, b"\r\n", &good]);
        let mut parser = Streaming::new();
        parser.feed(&combined);

        match parser.next_sentence().unwrap() {
            Err(Error::ChecksumMismatch { .. }) => {}
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "HDT");
    }

    // -----------------------------------------------------------------
    // Buffer overflow (PRD N1)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_yields_overflow_and_recovers() {
        let sentence = build_sentence(b"GPGGA,1,2,3");
        // Max size small enough that a big flood overflows, but still
        // big enough to contain one sentence afterwards.
        let max = 64;
        assert!(sentence.len() <= max);

        let mut parser = Streaming::with_capacity(max);
        // Feed 200 bytes of garbage — forces overflow.
        let garbage = [b'?'; 200];
        parser.feed(&garbage);

        // First call: the queued overflow error.
        match parser.next_sentence().unwrap() {
            Err(Error::BufferOverflow) => {}
            other => panic!("expected BufferOverflow, got {other:?}"),
        }

        // After the overflow, the surviving buffer is just more garbage
        // (no '$' ever fed), so next call yields None.
        assert!(parser.next_sentence().is_none());

        // Feed a clean sentence — parser should recover.
        parser.feed(&sentence);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
    }

    #[test]
    fn streaming_overflow_preserves_last_bytes_and_yields_recent_sentence() {
        // When the overflow drop crosses into the feed itself, only the
        // last `max_size` bytes survive. Arrange for the last bytes to
        // contain a complete sentence.
        let sentence = build_sentence(b"GPGGA,x,y,z");
        let pre_garbage = [b'?'; 300];
        let combined = concat(&[&pre_garbage, &sentence]);
        let max = sentence.len().saturating_add(8); // a bit more than the sentence

        let mut parser = Streaming::with_capacity(max);
        parser.feed(&combined);

        // First the overflow signal.
        match parser.next_sentence().unwrap() {
            Err(Error::BufferOverflow) => {}
            other => panic!("expected BufferOverflow, got {other:?}"),
        }
        // Then the sentence the tail contained.
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
    }

    // -----------------------------------------------------------------
    // Steady-state compaction (verify memory stays bounded)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_stays_bounded_across_many_sentences() {
        let max = 256;
        let mut parser = Streaming::with_capacity(max);
        let sentence = build_sentence(b"GPGGA,1,2,3");
        // Feed enough to dwarf max_size several times over, consuming as
        // we go. No overflow error should ever fire, because compaction
        // keeps the buffer small.
        for _ in 0..200 {
            parser.feed(&sentence);
            let s = parser.next_sentence().unwrap().unwrap();
            assert_eq!(s.sentence_type, "GGA");
        }
    }

    // -----------------------------------------------------------------
    // Degenerate inputs
    // -----------------------------------------------------------------

    #[test]
    fn streaming_empty_feed_is_a_noop() {
        let mut parser = Streaming::new();
        parser.feed(&[]);
        assert!(parser.next_sentence().is_none());
        parser.feed(&[]);
        assert!(parser.next_sentence().is_none());
    }

    // -----------------------------------------------------------------
    // TAG block handling (PRD E4, decision 7)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_parses_sentence_with_tag_block() {
        let bytes = build_with_tag(b"c:1577836800", b"GPGGA,1,2,3");
        let mut parser = Streaming::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.tag_block, Some(b"c:1577836800".as_slice()));
        assert_eq!(s.sentence_type, "GGA");
        assert!(s.raw.starts_with(b"$GPGGA"));
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn streaming_parses_multiple_tagged_sentences_in_one_feed() {
        let s1 = build_with_tag_and_terminator(b"c:1", b"GPGGA,A", b"\r\n");
        let s2 = build_with_tag_and_terminator(b"s:src2", b"INHDT,200.0,T", b"\r\n");
        let combined = concat(&[&s1, &s2]);
        let mut parser = Streaming::new();
        parser.feed(&combined);

        let first = parser.next_sentence().unwrap().unwrap();
        assert_eq!(first.tag_block, Some(b"c:1".as_slice()));
        assert_eq!(first.sentence_type, "GGA");

        let second = parser.next_sentence().unwrap().unwrap();
        assert_eq!(second.tag_block, Some(b"s:src2".as_slice()));
        assert_eq!(second.sentence_type, "HDT");
    }

    #[test]
    fn streaming_accepts_tag_block_with_invalid_tag_checksum() {
        // PRD decision 7: TAG checksum mismatch is advisory.
        let bytes = build_with_bad_tag_checksum(b"c:999", b"GPGGA,1,2");
        let mut parser = Streaming::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.tag_block, Some(b"c:999".as_slice()));
        assert_eq!(s.sentence_type, "GGA");
    }

    #[test]
    fn streaming_discards_orphaned_tag_block() {
        // TAG with no sentence following, then a real sentence. Streaming
        // should skip the orphaned TAG and yield the sentence.
        let good = build_sentence(b"INHDT,0.0,T");

        // Build a TAG block not followed by '$' or '!', then junk, then
        // the real sentence.
        let mut bytes = Vec::new();
        bytes.push(b'\\');
        bytes.extend_from_slice(b"c:1");
        bytes.push(b'*');
        let cksum = b"c:1".iter().fold(0u8, |acc, &x| acc ^ x);
        let nibble = |v: u8| if v < 10 { b'0' + v } else { b'A' + (v - 10) };
        bytes.push(nibble(cksum >> 4));
        bytes.push(nibble(cksum & 0x0F));
        bytes.push(b'\\');
        bytes.extend_from_slice(b"garbage-between");
        bytes.extend_from_slice(&good);

        let mut parser = Streaming::new();
        parser.feed(&bytes);
        let s = parser.next_sentence().unwrap().unwrap();
        assert!(s.tag_block.is_none());
        assert_eq!(s.sentence_type, "HDT");
        assert!(parser.next_sentence().is_none());
    }

    #[test]
    fn streaming_partial_tag_block_waits_for_closing_backslash() {
        // Half a TAG block — no closing '\'. Streaming must NOT advance
        // past the opening '\' yet; it may still be completed.
        let mut parser = Streaming::new();
        parser.feed(b"\\c:1234567890*AB");
        assert!(parser.next_sentence().is_none());

        // Now complete the TAG and provide a sentence.
        let closing_and_sentence = {
            let mut v = Vec::from(b"\\$GPGGA,1,2,3*".as_slice());
            let cksum = b"GPGGA,1,2,3".iter().fold(0u8, |a, &b| a ^ b);
            let n = |v: u8| if v < 10 { b'0' + v } else { b'A' + (v - 10) };
            v.push(n(cksum >> 4));
            v.push(n(cksum & 0x0F));
            v
        };
        parser.feed(&closing_and_sentence);
        let s = parser.next_sentence().unwrap().unwrap();
        // tag_block is the content BEFORE the TAG's '*' (the checksum
        // byte `AB` is the TAG's own checksum, not part of the content).
        assert_eq!(s.tag_block, Some(b"c:1234567890".as_slice()));
        assert_eq!(s.sentence_type, "GGA");
    }

    // -----------------------------------------------------------------
    // Structurally malformed TAG blocks (surface via parser, in the
    // streaming context)
    // -----------------------------------------------------------------

    #[test]
    fn streaming_malformed_tag_without_star_yields_error() {
        // `\content\` — no '*' inside the TAG block at all.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\\content-no-star\\");
        // Then a real sentence so the slice is "complete".
        bytes.extend_from_slice(b"$GPGGA,1*");
        let cksum = b"GPGGA,1".iter().fold(0u8, |a, &b| a ^ b);
        let n = |v: u8| if v < 10 { b'0' + v } else { b'A' + (v - 10) };
        bytes.push(n(cksum >> 4));
        bytes.push(n(cksum & 0x0F));

        let mut parser = Streaming::new();
        parser.feed(&bytes);
        match parser.next_sentence().unwrap() {
            Err(Error::MalformedTagBlock) => {}
            other => panic!("expected MalformedTagBlock, got {other:?}"),
        }
    }

    #[test]
    fn streaming_malformed_tag_with_non_hex_checksum_yields_error() {
        // `\c:1*ZZ\` — bad hex digits in TAG checksum.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\\c:1*ZZ\\$GPGGA,A*");
        let cksum = b"GPGGA,A".iter().fold(0u8, |a, &b| a ^ b);
        let n = |v: u8| if v < 10 { b'0' + v } else { b'A' + (v - 10) };
        bytes.push(n(cksum >> 4));
        bytes.push(n(cksum & 0x0F));

        let mut parser = Streaming::new();
        parser.feed(&bytes);
        match parser.next_sentence().unwrap() {
            Err(Error::MalformedTagBlock) => {}
            other => panic!("expected MalformedTagBlock, got {other:?}"),
        }
    }

    #[test]
    fn streaming_single_byte_feeds_eventually_yield_sentence() {
        let full = build_sentence(b"GPGGA,1,2");
        let mut parser = Streaming::new();
        for b in &full {
            parser.feed(core::slice::from_ref(b));
        }
        let s = parser.next_sentence().unwrap().unwrap();
        assert_eq!(s.sentence_type, "GGA");
    }
}
