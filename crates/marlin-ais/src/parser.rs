//! AIS parser wrapper — typed `feed` / `next_message` over any
//! [`SentenceSource`].
//!
//! This is the AIS-layer analogue of `marlin_nmea_0183::Nmea0183Parser`.
//! It composes four pieces this crate already exposes:
//!
//! 1. An envelope-level [`SentenceSource`] (`OneShot` / `Streaming`)
//!    that yields [`RawSentence`] values.
//! 2. [`parse_aivdm_wrapper`](crate::parse_aivdm_wrapper) to pull the
//!    AIVDM/AIVDO header out of each `RawSentence`.
//! 3. [`AisReassembler`] to glue multi-fragment messages back together.
//! 4. [`decode_message`](crate::decode_message) to turn the armored
//!    payload into a typed [`AisMessage`].
//!
//! The downstream caller sees the same shape as every other parser in
//! the workspace:
//!
//! ```text
//! parser.feed(&bytes);
//! while let Some(result) = parser.next_message() {
//!     match result { ... }
//! }
//! ```

use marlin_nmea_envelope::{OneShot, RawSentence, SentenceSource, Streaming};

use crate::{
    armor, decode_message, parse_aivdm_wrapper, AisError, AisMessage, AisReassembler,
    ReassembledPayload,
};

// ---------------------------------------------------------------------------
// Generic wrapper
// ---------------------------------------------------------------------------

/// Typed AIS parser wrapping any envelope-level [`SentenceSource`].
///
/// Carries its own [`AisReassembler`] so multi-sentence messages are
/// transparently glued back together. Each [`next_message`](Self::next_message)
/// call either yields a complete [`AisMessage`], an error, or `None`
/// (no complete message available yet).
#[derive(Debug)]
pub struct AisFragmentParser<P> {
    inner: P,
    reassembler: AisReassembler,
}

impl<P> AisFragmentParser<P> {
    /// Wrap `inner` with a default-sized reassembler
    /// ([`crate::DEFAULT_MAX_PARTIALS`] slots).
    #[must_use]
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            reassembler: AisReassembler::new(),
        }
    }

    /// Wrap `inner` with a caller-supplied reassembler. Use this to
    /// pick a non-default `max_partials`.
    #[must_use]
    pub fn with_reassembler(inner: P, reassembler: AisReassembler) -> Self {
        Self { inner, reassembler }
    }

    /// Borrow the underlying envelope parser.
    #[must_use]
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Mutably borrow the underlying envelope parser.
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Borrow the reassembler — useful for diagnostics
    /// ([`AisReassembler::in_flight`](crate::AisReassembler::in_flight)).
    #[must_use]
    pub fn reassembler(&self) -> &AisReassembler {
        &self.reassembler
    }

    /// Unwrap back to the underlying envelope parser. The reassembler
    /// and any in-flight partials are dropped.
    #[must_use]
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> AisFragmentParser<P>
where
    P: for<'a> SentenceSource<Item<'a> = RawSentence<'a>>,
{
    /// Push raw bytes into the underlying envelope parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.inner.feed(bytes);
    }

    /// Pull the next complete AIS message.
    ///
    /// Returns:
    ///
    /// - `Some(Ok(message))` — envelope parsed, fragments reassembled,
    ///   typed decode succeeded.
    /// - `Some(Err(AisError::Envelope(_)))` — envelope-level failure
    ///   (checksum, framing). The underlying parser has advanced past
    ///   the bad bytes.
    /// - `Some(Err(AisError::NotAnAisSentence))` — a non-AIS sentence
    ///   (e.g. `$GPGGA`) leaked through the envelope parser. Callers
    ///   that mix AIS and NMEA on the same channel should run a
    ///   `marlin_nmea_0183::Parser` alongside this one, or strip
    ///   non-AIS traffic before feeding here.
    /// - `Some(Err(AisError::Reassembly...))` — fragment-ordering or
    ///   channel-mismatch issue (see
    ///   [`AisReassembler`](crate::AisReassembler)).
    /// - `Some(Err(..))` — other decoding failures (armor, bit reader,
    ///   payload length).
    /// - `None` — no complete sentence is buffered yet; call
    ///   [`feed`](Self::feed) with more bytes.
    pub fn next_message(&mut self) -> Option<Result<AisMessage, AisError>> {
        loop {
            if let Some(err) = self.reassembler.take_pending_error() {
                return Some(Err(err));
            }
            let raw = match self.inner.next_sentence() {
                Some(Ok(r)) => r,
                Some(Err(e)) => return Some(Err(AisError::from(e))),
                None => return None,
            };
            let header = match parse_aivdm_wrapper(&raw) {
                Ok(h) => h,
                Err(e) => return Some(Err(e)),
            };
            match self.reassembler.feed_fragment(&header) {
                Ok(Some(assembled)) => return Some(finish_assembled(&assembled)),
                Ok(None) => {}
                Err(e) => return Some(Err(e)),
            }
        }
    }

    /// Time-aware variant of [`next_message`](Self::next_message).
    ///
    /// Before pulling work, advances the reassembler's clock to
    /// `now_ms` (expiring any partials past their TTL). Fragments
    /// drained during this call are stamped with `now_ms` so the next
    /// [`tick`](crate::AisReassembler::tick) measures their age
    /// correctly.
    ///
    /// Use this with an
    /// [`AisReassembler::with_timeout_ms`](crate::AisReassembler::with_timeout_ms)
    /// reassembler. `now_ms` is a monotonic millisecond timestamp
    /// chosen by the caller (e.g. `Instant::now().duration_since(epoch).as_millis()`
    /// in std, or a platform tick counter in embedded).
    //
    // The body duplicates `next_message`'s loop because Rust's HRTB
    // handling chokes when a `for<'a> SentenceSource<Item<'a> = ...>`-
    // bounded method calls another such method or function that
    // shares the bound (rustc emits "P does not live long enough").
    // Inlining sidesteps the compiler limitation.
    pub fn next_message_at(&mut self, now_ms: u64) -> Option<Result<AisMessage, AisError>> {
        self.reassembler.tick(now_ms);
        loop {
            if let Some(err) = self.reassembler.take_pending_error() {
                return Some(Err(err));
            }
            let raw = match self.inner.next_sentence() {
                Some(Ok(r)) => r,
                Some(Err(e)) => return Some(Err(AisError::from(e))),
                None => return None,
            };
            let header = match parse_aivdm_wrapper(&raw) {
                Ok(h) => h,
                Err(e) => return Some(Err(e)),
            };
            match self.reassembler.feed_fragment_at(&header, now_ms) {
                Ok(Some(assembled)) => return Some(finish_assembled(&assembled)),
                Ok(None) => {}
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// Decode a reassembled armored payload into a typed [`AisMessage`].
///
/// Factored out of `next_message` / `next_message_at` so the duplicated
/// loop bodies share the completion tail. The tail takes only concrete
/// types (no HRTB), so extracting it sidesteps the nested-HRTB compiler
/// limitation that forced the loop itself to stay inlined.
fn finish_assembled(assembled: &ReassembledPayload) -> Result<AisMessage, AisError> {
    let (bits, total_bits) = armor::decode(&assembled.payload, assembled.fill_bits)?;
    decode_message(&bits, total_bits, assembled.is_own_ship)
}

impl<P: Default> Default for AisFragmentParser<P> {
    fn default() -> Self {
        Self::new(P::default())
    }
}

// ---------------------------------------------------------------------------
// Runtime-dispatch enum
// ---------------------------------------------------------------------------

/// Runtime selector between one-shot and streaming AIS parsers,
/// mirroring `marlin_nmea_envelope::Parser`.
///
/// Dispatch is a match (static, inlined) — no heap allocation, no
/// vtable lookup.
#[derive(Debug)]
pub enum Parser {
    /// Single-sentence-per-feed mode; wraps
    /// [`AisFragmentParser`]`<`[`OneShot`]`>`.
    OneShot(AisFragmentParser<OneShot>),
    /// Buffered streaming mode; wraps
    /// [`AisFragmentParser`]`<`[`Streaming`]`>`.
    Streaming(AisFragmentParser<Streaming>),
}

impl Parser {
    /// Construct a one-shot parser with a default-sized reassembler.
    #[must_use]
    pub fn one_shot() -> Self {
        Self::OneShot(AisFragmentParser::new(OneShot::new()))
    }

    /// Construct a streaming parser with the envelope's default
    /// maximum buffer size and a default-sized reassembler.
    #[must_use]
    pub fn streaming() -> Self {
        Self::Streaming(AisFragmentParser::new(Streaming::new()))
    }

    /// Construct a streaming parser with a specified maximum envelope
    /// buffer size.
    #[must_use]
    pub fn streaming_with_capacity(max_size: usize) -> Self {
        Self::Streaming(AisFragmentParser::new(Streaming::with_capacity(max_size)))
    }

    /// Push raw bytes into the active parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        match self {
            Self::OneShot(p) => p.feed(bytes),
            Self::Streaming(p) => p.feed(bytes),
        }
    }

    /// Pull the next typed AIS message. See
    /// [`AisFragmentParser::next_message`] for the full semantics.
    pub fn next_message(&mut self) -> Option<Result<AisMessage, AisError>> {
        match self {
            Self::OneShot(p) => p.next_message(),
            Self::Streaming(p) => p.next_message(),
        }
    }

    /// Time-aware variant of [`next_message`](Self::next_message). See
    /// [`AisFragmentParser::next_message_at`].
    pub fn next_message_at(&mut self, now_ms: u64) -> Option<Result<AisMessage, AisError>> {
        match self {
            Self::OneShot(p) => p.next_message_at(now_ms),
            Self::Streaming(p) => p.next_message_at(now_ms),
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
    use crate::{testing::build_aivdm, AisMessageBody};
    use alloc::vec::Vec;

    // -----------------------------------------------------------------
    // Single-fragment happy path through OneShot
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_single_fragment_classic_type1() {
        let mut p = Parser::one_shot();
        let sentence = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        p.feed(&sentence);
        let msg = p.next_message().unwrap().unwrap();
        match msg.body {
            AisMessageBody::Type1(pra) => assert_eq!(pra.mmsi, 244_708_736),
            other => panic!("expected Type1, got {other:?}"),
        }
        assert!(!msg.is_own_ship);
    }

    // -----------------------------------------------------------------
    // AIVDO → is_own_ship true
    // -----------------------------------------------------------------

    #[test]
    fn one_shot_aivdo_sets_is_own_ship() {
        let mut p = Parser::one_shot();
        p.feed(&crate::testing::build_with_address(
            b"!",
            b"AIVDO",
            b"1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0",
        ));
        let msg = p.next_message().unwrap().unwrap();
        assert!(msg.is_own_ship);
    }

    // -----------------------------------------------------------------
    // Streaming mode — multiple sentences in one feed
    // -----------------------------------------------------------------

    #[test]
    fn streaming_decodes_multiple_single_fragment_sentences_per_feed() {
        let mut p = Parser::streaming();
        let s1 = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        let s2 = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        let mut combined = Vec::new();
        combined.extend_from_slice(&s1);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&s2);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        let mut count = 0;
        while let Some(r) = p.next_message() {
            r.unwrap();
            count += 1;
        }
        assert_eq!(count, 2);
    }

    // -----------------------------------------------------------------
    // Multi-sentence reassembly: classic Type 5 captured pair.
    // AIVDM payloads taken from the gpsd AIS test corpus (public domain).
    // -----------------------------------------------------------------
    //
    // These are well-known public-sample fragments for a Type 5 static-
    // and-voyage report. Reassembled they produce a single valid
    // 424-bit Type 5 payload; we just need to confirm the reassembly
    // path yields Type5 (not an error).
    const TYPE5_FRAG_A: &[u8] = b"55P5TL01VIaAL@7WKO@mBplU@<PDhh000000001S;AJ::4A80?4i@E53";
    const TYPE5_FRAG_B: &[u8] = b"1CQWBDhH888888888880";

    #[test]
    fn streaming_reassembles_two_fragment_type5() {
        let mut p = Parser::streaming();
        let frag1 = build_aivdm(2, 1, Some(3), Some(b'A'), TYPE5_FRAG_A, 0);
        let frag2 = build_aivdm(2, 2, Some(3), Some(b'A'), TYPE5_FRAG_B, 2);
        let mut combined = Vec::new();
        combined.extend_from_slice(&frag1);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&frag2);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        let msg = p.next_message().unwrap().unwrap();
        match msg.body {
            AisMessageBody::Type5(_) => {}
            other => panic!("expected Type5 after reassembly, got {other:?}"),
        }
        // No residual message after the reassembled one.
        assert!(p.next_message().is_none());
    }

    // -----------------------------------------------------------------
    // Multi-fragment fed across two feeds (simulating a slow UDP /
    // serial arrival) — wrapper must hold state across feed calls.
    // -----------------------------------------------------------------

    #[test]
    fn streaming_holds_state_across_feeds_for_multi_fragment() {
        let mut p = Parser::streaming();
        let frag1 = build_aivdm(2, 1, Some(3), Some(b'A'), TYPE5_FRAG_A, 0);
        let frag2 = build_aivdm(2, 2, Some(3), Some(b'A'), TYPE5_FRAG_B, 2);

        p.feed(&frag1);
        p.feed(b"\r\n");
        // First fragment accepted; no complete message yet.
        assert!(p.next_message().is_none());

        p.feed(&frag2);
        p.feed(b"\r\n");
        let msg = p.next_message().unwrap().unwrap();
        assert!(matches!(msg.body, AisMessageBody::Type5(_)));
    }

    // -----------------------------------------------------------------
    // Out-of-order fragment surfaces ReassemblyOutOfOrder
    // -----------------------------------------------------------------

    #[test]
    fn streaming_out_of_order_fragment_surfaces_error() {
        let mut p = Parser::streaming();
        // Fragment 1 of 2 arrives, followed directly by fragment 3 of 2
        // (nonsense) — the wrapper should surface MalformedWrapper
        // (caught by reassembler's frag_num > frag_count check).
        let frag1 = build_aivdm(2, 1, Some(4), Some(b'A'), b"AA", 0);
        let frag_bad = build_aivdm(2, 3, Some(4), Some(b'A'), b"BB", 0);
        let mut combined = Vec::new();
        combined.extend_from_slice(&frag1);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&frag_bad);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        // First message: fragment 1 accepted, no complete message
        // available. Loop drains the bad second fragment next and
        // yields the error.
        match p.next_message().unwrap() {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn streaming_skipped_fragment_surfaces_out_of_order() {
        let mut p = Parser::streaming();
        // Fragment 1 of 3, then fragment 3 of 3 — the middle is missing.
        let frag1 = build_aivdm(3, 1, Some(4), Some(b'A'), b"AA", 0);
        let frag3 = build_aivdm(3, 3, Some(4), Some(b'A'), b"CC", 0);
        let mut combined = Vec::new();
        combined.extend_from_slice(&frag1);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&frag3);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        match p.next_message().unwrap() {
            Err(AisError::ReassemblyOutOfOrder) => {}
            other => panic!("expected ReassemblyOutOfOrder, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Channel mismatch
    // -----------------------------------------------------------------

    #[test]
    fn streaming_channel_mismatch_surfaces_error() {
        let mut p = Parser::streaming();
        let frag1 = build_aivdm(2, 1, Some(6), Some(b'A'), TYPE5_FRAG_A, 0);
        let frag2 = build_aivdm(2, 2, Some(6), Some(b'B'), TYPE5_FRAG_B, 2);
        let mut combined = Vec::new();
        combined.extend_from_slice(&frag1);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&frag2);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        match p.next_message().unwrap() {
            Err(AisError::ReassemblyChannelMismatch) => {}
            other => panic!("expected ReassemblyChannelMismatch, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Envelope-level error is forwarded through the wrapper
    // -----------------------------------------------------------------

    #[test]
    fn streaming_forwards_envelope_checksum_error() {
        let mut p = Parser::streaming();
        // Hand-built AIVDM with a deliberately wrong checksum.
        p.feed(b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*FF\r\n");
        match p.next_message().unwrap() {
            Err(AisError::Envelope(_)) => {}
            other => panic!("expected Envelope(_), got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Non-AIS sentence in the stream is surfaced, stream recovers
    // -----------------------------------------------------------------

    #[test]
    fn streaming_surfaces_non_ais_sentence_error_and_recovers() {
        let mut p = Parser::streaming();
        let gga =
            crate::testing::build_with_address(b"$", b"GPGGA", b"1,2,3,4,5,6,7,8,9,10,11,12,13,14");
        let ais = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        let mut combined = Vec::new();
        combined.extend_from_slice(&gga);
        combined.extend_from_slice(b"\r\n");
        combined.extend_from_slice(&ais);
        combined.extend_from_slice(b"\r\n");
        p.feed(&combined);

        match p.next_message().unwrap() {
            Err(AisError::NotAnAisSentence) => {}
            other => panic!("expected NotAnAisSentence, got {other:?}"),
        }
        // The next call succeeds: stream recovered.
        let msg = p.next_message().unwrap().unwrap();
        assert!(matches!(msg.body, AisMessageBody::Type1(_)));
    }

    // -----------------------------------------------------------------
    // None on empty / partial buffer
    // -----------------------------------------------------------------

    #[test]
    fn streaming_returns_none_on_empty_buffer() {
        let mut p = Parser::streaming();
        assert!(p.next_message().is_none());
    }

    #[test]
    fn streaming_returns_none_on_partial_sentence() {
        let mut p = Parser::streaming();
        p.feed(b"!AIVDM,1,1,,A,13aGmP0P"); // no `*hh` yet
        assert!(p.next_message().is_none());
    }

    // -----------------------------------------------------------------
    // Generic wrapper (without the Parser enum) works too
    // -----------------------------------------------------------------

    #[test]
    fn generic_wrapper_works_with_one_shot() {
        let mut p: AisFragmentParser<OneShot> = AisFragmentParser::new(OneShot::new());
        p.feed(&build_aivdm(
            1,
            1,
            None,
            Some(b'A'),
            b"13aGmP0P00PD;88MD5MTDww@2<0L",
            0,
        ));
        let msg = p.next_message().unwrap().unwrap();
        assert!(matches!(msg.body, AisMessageBody::Type1(_)));
    }

    #[test]
    fn generic_wrapper_works_with_streaming() {
        let mut p: AisFragmentParser<Streaming> = AisFragmentParser::default();
        p.feed(&build_aivdm(
            1,
            1,
            None,
            Some(b'A'),
            b"13aGmP0P00PD;88MD5MTDww@2<0L",
            0,
        ));
        let msg = p.next_message().unwrap().unwrap();
        assert!(matches!(msg.body, AisMessageBody::Type1(_)));
    }

    // -----------------------------------------------------------------
    // Reassembler eviction: ReassemblyTimeout surfaces via next_message
    // -----------------------------------------------------------------

    #[test]
    fn streaming_surfaces_reassembly_timeout_from_eviction() {
        // Small reassembler → easy to overflow in one test.
        let inner = Streaming::new();
        let reasm = AisReassembler::with_max_partials(2);
        let mut p = AisFragmentParser::with_reassembler(inner, reasm);

        // Three first-fragments on different seq_ids → evicts the oldest.
        let f1 = build_aivdm(2, 1, Some(1), Some(b'A'), b"aa", 0);
        let f2 = build_aivdm(2, 1, Some(2), Some(b'A'), b"bb", 0);
        let f3 = build_aivdm(2, 1, Some(3), Some(b'A'), b"cc", 0);
        let mut combined = Vec::new();
        for f in [&f1, &f2, &f3] {
            combined.extend_from_slice(f);
            combined.extend_from_slice(b"\r\n");
        }
        p.feed(&combined);

        // Fragments accepted silently, then the eviction timeout
        // surfaces on the next poll.
        match p.next_message().unwrap() {
            Err(AisError::ReassemblyTimeout) => {}
            other => panic!("expected ReassemblyTimeout, got {other:?}"),
        }
        assert!(p.next_message().is_none());
    }

    // -----------------------------------------------------------------
    // Time-aware wrapper API: next_message_at drives reassembler clock
    // -----------------------------------------------------------------

    #[test]
    fn next_message_at_expires_stale_partial() {
        let inner = Streaming::new();
        let reasm = AisReassembler::with_timeout_ms(16, 1_000);
        let mut p = AisFragmentParser::with_reassembler(inner, reasm);

        // First fragment of a 2-fragment message at t=0.
        p.feed(&build_aivdm(2, 1, Some(1), Some(b'A'), b"aa", 0));
        p.feed(b"\r\n");
        assert!(p.next_message_at(0).is_none());
        assert_eq!(p.reassembler().in_flight(), 1);

        // Time jumps past the timeout without fragment 2 arriving.
        // next_message_at first ticks, which expires the partial,
        // then returns the queued ReassemblyTimeout.
        match p.next_message_at(5_000).unwrap() {
            Err(AisError::ReassemblyTimeout) => {}
            other => panic!("expected ReassemblyTimeout, got {other:?}"),
        }
        assert_eq!(p.reassembler().in_flight(), 0);
    }

    #[test]
    fn next_message_at_does_not_expire_within_timeout() {
        let inner = Streaming::new();
        let reasm = AisReassembler::with_timeout_ms(16, 10_000);
        let mut p = AisFragmentParser::with_reassembler(inner, reasm);

        // Two-fragment message that arrives slowly but within the
        // timeout window.
        p.feed(&build_aivdm(2, 1, Some(2), Some(b'A'), TYPE5_FRAG_A, 0));
        p.feed(b"\r\n");
        assert!(p.next_message_at(0).is_none());

        p.feed(&build_aivdm(2, 2, Some(2), Some(b'A'), TYPE5_FRAG_B, 2));
        p.feed(b"\r\n");
        // Time advanced 5s — still under the 10s TTL.
        let msg = p.next_message_at(5_000).unwrap().unwrap();
        assert!(matches!(msg.body, AisMessageBody::Type5(_)));
    }
}
