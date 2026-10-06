//! Multi-sentence AIS reassembly (PRD §A5).
//!
//! AIVDM/AIVDO encapsulates one AIS message into one or more NMEA 0183
//! sentences — the AIS payload is armored into 6-bit ASCII and split
//! when it exceeds the NMEA sentence-length ceiling. Downstream
//! decoders need the complete armored payload; this module stitches
//! fragments back together.
//!
//! # Design choices
//!
//! - **Reassembly key: `(channel, sequential_id)`** per PRD §A5. This
//!   lets two multi-sentence messages interleave cleanly on different
//!   channels or different sequential IDs.
//!
//! - **Single-fragment fast path**: when `fragment_count == 1`, the
//!   reassembler returns a [`ReassembledPayload`] immediately without
//!   touching internal state. No allocation beyond the one `Vec<u8>`
//!   that owns the payload copy.
//!
//! - **Strict fragment ordering**: fragments must arrive in order
//!   1, 2, 3, ... . Any out-of-order arrival yields
//!   [`AisError::ReassemblyOutOfOrder`] and discards the partial.
//!
//! - **Bounded-slots eviction for memory safety**: rather than a
//!   wall-clock timeout (this crate is sans-I/O and `#![no_std]`),
//!   the reassembler caps the number of concurrent partials. When the
//!   cap is exceeded, the oldest partial is evicted and
//!   [`AisError::ReassemblyTimeout`] is queued for the next
//!   [`take_pending_error`](AisReassembler::take_pending_error) call.
//!   PRD §A5 specifies a 60 s timeout as the motivation — this
//!   bounded-slot approach satisfies the underlying memory-safety goal
//!   clock-free. Time-based expiry is opt-in on top of it:
//!   [`AisReassembler::with_timeout_ms`] sets an age limit, and the
//!   caller supplies the clock through [`tick`](AisReassembler::tick)
//!   (ADR-0004). The reassembler keeps the last ticked time and
//!   stamps each fragment with it; it never reads a clock itself.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::{AisError, AivdmHeader};

/// The output of [`AisReassembler::feed_fragment`] when a complete
/// armored payload is ready for [`crate::armor::decode`].
///
/// `payload` is the **armored** byte slice (ASCII-6-bit), concatenated
/// across fragments. `fill_bits` is the final fragment's fill count,
/// which applies to the whole concatenated stream (intermediate
/// fragments conventionally carry `fill_bits = 0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassembledPayload {
    /// Concatenated armored payload bytes.
    pub payload: Vec<u8>,
    /// Fill bits to strip from the end of the decoded bit stream
    /// (taken from the last fragment; 0..=5).
    pub fill_bits: u8,
    /// `is_own_ship` propagated from the first fragment.
    pub is_own_ship: bool,
}

/// Default maximum number of concurrent multi-sentence assemblies held
/// by a reassembler. Sized for typical AIS receiver loads — real-world
/// channels rarely see more than a handful of in-flight multi-sentence
/// messages at once.
pub const DEFAULT_MAX_PARTIALS: usize = 16;

/// Multi-sentence reassembler.
///
/// Feed it [`AivdmHeader`] values in the order they arrive; it buffers
/// multi-fragment messages and returns a [`ReassembledPayload`] when a
/// complete armored payload is available. Single-fragment messages are
/// returned immediately on the first call.
///
/// # Errors
///
/// [`feed_fragment`](Self::feed_fragment) can surface:
///
/// - [`AisError::MalformedWrapper`] — a multi-fragment header missing
///   `sequential_id`, or `fragment_number == 0`, or
///   `fragment_number > fragment_count`.
/// - [`AisError::ReassemblyOutOfOrder`] — fragment arrived with a
///   number other than the next expected one for its key, or no
///   partial is open on its `(channel, sequential_id)` key.
///
/// Evictions caused by exceeding
/// [`max_partials`](Self::max_partials) surface
/// [`AisError::ReassemblyTimeout`] via
/// [`take_pending_error`](Self::take_pending_error), not via
/// [`feed_fragment`](Self::feed_fragment). This keeps "new fragment
/// accepted, but an old partial was dropped" a distinct signal from
/// "the current fragment failed."
#[derive(Debug)]
pub struct AisReassembler {
    partials: Vec<PartialMessage>,
    max_partials: usize,
    /// Caller-supplied timeout in monotonic milliseconds. `None`
    /// means time-based eviction is disabled; only bounded-slots
    /// eviction applies.
    timeout_ms: Option<u64>,
    /// The clock as of the last [`tick`](AisReassembler::tick), in
    /// monotonic milliseconds. `None` until the first tick; partials
    /// opened before then carry no stamp.
    now_ms: Option<u64>,
    /// Queue of errors deferred until the next
    /// [`take_pending_error`](AisReassembler::take_pending_error)
    /// call — today that's `ReassemblyTimeout` raised by slot or
    /// time eviction. Multiple simultaneous evictions are preserved
    /// one per call.
    pending_errors: VecDeque<AisError>,
}

#[derive(Debug)]
struct PartialMessage {
    channel: Option<u8>,
    sequential_id: u8,
    fragment_count: u8,
    /// 1-based number of the next fragment we expect to see.
    next_expected: u8,
    is_own_ship: bool,
    payload: Vec<u8>,
    /// Last-seen `fill_bits` — overwritten on each append; the final
    /// fragment's value is the one we use at completion.
    last_fill_bits: u8,
    /// The reassembler's clock when the last fragment was seen.
    /// `None` means the partial was opened before the first
    /// [`tick`](AisReassembler::tick) and is **not** subject to time
    /// eviction — bounded-slots still applies.
    last_touched_ms: Option<u64>,
}

impl PartialMessage {
    /// Whether this partial belongs to the `(channel, sequential_id)`
    /// reassembly key. Every lookup goes through here so the two
    /// channels can never alias each other's sequential-id space.
    fn has_key(&self, channel: Option<u8>, sequential_id: u8) -> bool {
        self.channel == channel && self.sequential_id == sequential_id
    }
}

impl Default for AisReassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl AisReassembler {
    /// Construct a reassembler with [`DEFAULT_MAX_PARTIALS`] slots.
    #[must_use]
    pub fn new() -> Self {
        Self::with_max_partials(DEFAULT_MAX_PARTIALS)
    }

    /// Construct a reassembler with a caller-specified maximum number
    /// of concurrent partials. `max_partials` must be at least 1; a
    /// value of 0 is treated as 1 to keep the reassembler functional.
    #[must_use]
    pub fn with_max_partials(max_partials: usize) -> Self {
        Self {
            partials: Vec::new(),
            max_partials: max_partials.max(1),
            timeout_ms: None,
            now_ms: None,
            pending_errors: VecDeque::new(),
        }
    }

    /// Construct a reassembler with both bounded-slots eviction AND
    /// a clock-based timeout policy. Callers advance the clock with
    /// [`tick`](Self::tick) before feeding fragments.
    ///
    /// PRD §A5 suggests 60 000 ms (60 s) as a typical value. The
    /// library itself never calls a clock — time is the caller's
    /// responsibility, which keeps the crate `#![no_std]` and
    /// sans-I/O.
    #[must_use]
    pub fn with_timeout_ms(max_partials: usize, timeout_ms: u64) -> Self {
        Self {
            partials: Vec::new(),
            max_partials: max_partials.max(1),
            timeout_ms: Some(timeout_ms),
            now_ms: None,
            pending_errors: VecDeque::new(),
        }
    }

    /// Maximum number of in-flight partials this reassembler will
    /// hold before evicting the oldest.
    #[must_use]
    pub fn max_partials(&self) -> usize {
        self.max_partials
    }

    /// Current time-based eviction threshold, in monotonic
    /// milliseconds. `None` means disabled.
    #[must_use]
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    /// Replace the time-based eviction threshold. Pass `None` to
    /// disable time eviction; stamped partials keep their stamps but
    /// no tick will expire them until a new timeout is set.
    pub fn set_timeout_ms(&mut self, timeout_ms: Option<u64>) {
        self.timeout_ms = timeout_ms;
    }

    /// Number of partials currently held. Mostly useful for tests and
    /// diagnostics.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.partials.len()
    }

    /// Pop the next deferred error from the queue — today that's
    /// [`AisError::ReassemblyTimeout`] raised by slot-eviction or by
    /// [`tick`](Self::tick). The parser wrapper calls this between
    /// fragments so evictions are surfaced in order.
    pub fn take_pending_error(&mut self) -> Option<AisError> {
        self.pending_errors.pop_front()
    }

    /// Feed an AIVDM/AIVDO header into the reassembler.
    ///
    /// The partial it opens or appends to is stamped with the clock
    /// as of the last [`tick`](Self::tick). Before the first tick
    /// there is no stamp, and bounded-slots eviction is the only
    /// mechanism that can retire such a partial.
    ///
    /// Returns:
    /// - `Ok(Some(payload))` — a complete armored payload is ready.
    /// - `Ok(None)` — fragment accepted; still waiting for more.
    /// - `Err(..)` — the fragment is malformed or violates the
    ///   reassembly contract; no complete payload was emitted.
    pub fn feed_fragment(
        &mut self,
        header: &AivdmHeader<'_>,
    ) -> Result<Option<ReassembledPayload>, AisError> {
        // Validate the fragment-numbering invariants first — these
        // checks apply to single and multi-fragment messages alike.
        if header.fragment_count == 0
            || header.fragment_number == 0
            || header.fragment_number > header.fragment_count
        {
            return Err(AisError::MalformedWrapper);
        }

        // Fast path: a self-contained single-fragment message.
        if header.fragment_count == 1 {
            return Ok(Some(ReassembledPayload {
                payload: header.payload.to_vec(),
                fill_bits: header.fill_bits,
                is_own_ship: header.is_own_ship,
            }));
        }

        // Multi-fragment: sequential_id is required.
        let seq_id = header.sequential_id.ok_or(AisError::MalformedWrapper)?;

        if header.fragment_number == 1 {
            self.open_partial(header, seq_id);
            return Ok(None);
        }

        self.append_to_partial(header, seq_id)
    }

    /// Advance the clock to `now_ms` and evict any partial whose
    /// last-touched stamp is older than
    /// [`timeout_ms`](Self::timeout_ms). Each evicted partial queues
    /// one [`AisError::ReassemblyTimeout`] for retrieval via
    /// [`take_pending_error`](Self::take_pending_error). Fragments
    /// fed after this call are stamped with `now_ms`.
    ///
    /// Partials opened before the first tick have no stamp and are
    /// immune. Eviction is a no-op when `timeout_ms` is `None`; the
    /// clock still advances.
    pub fn tick(&mut self, now_ms: u64) {
        self.now_ms = Some(now_ms);
        let Some(timeout) = self.timeout_ms else {
            return;
        };
        let before = self.partials.len();
        self.partials.retain(|p| match p.last_touched_ms {
            Some(t) => now_ms.saturating_sub(t) <= timeout,
            None => true,
        });
        let evicted = before - self.partials.len();
        for _ in 0..evicted {
            self.pending_errors.push_back(AisError::ReassemblyTimeout);
        }
    }

    /// Start (or restart) a partial for a first fragment.
    fn open_partial(&mut self, header: &AivdmHeader<'_>, seq_id: u8) {
        // If there's already a partial on the same key, replace it
        // silently — the old one got interrupted by a fresh fragment 1.
        // This matches real-world behavior when a multi-sentence
        // message is truncated and the transmitter immediately starts
        // a new one.
        self.partials.retain(|p| !p.has_key(header.channel, seq_id));

        // Evict if full.
        if self.partials.len() >= self.max_partials {
            // Drop the oldest slot (index 0 is the longest-standing).
            // Safe: len >= max_partials >= 1.
            let _ = self.partials.remove(0);
            self.pending_errors.push_back(AisError::ReassemblyTimeout);
        }

        self.partials.push(PartialMessage {
            channel: header.channel,
            sequential_id: seq_id,
            fragment_count: header.fragment_count,
            next_expected: 2,
            is_own_ship: header.is_own_ship,
            payload: header.payload.to_vec(),
            last_fill_bits: header.fill_bits,
            last_touched_ms: self.now_ms,
        });
    }

    /// Append to an existing partial. `fragment_number` is guaranteed
    /// to be >= 2 at this point.
    fn append_to_partial(
        &mut self,
        header: &AivdmHeader<'_>,
        seq_id: u8,
    ) -> Result<Option<ReassembledPayload>, AisError> {
        // Find the partial on the same `(channel, sequential_id)` key.
        // Channels A and B carry independent sequential-id spaces, so
        // a partial on the other channel is simply not ours; a
        // fragment with no partial on its own key is out of order.
        let Some(idx) = self
            .partials
            .iter()
            .position(|p| p.has_key(header.channel, seq_id))
        else {
            return Err(AisError::ReassemblyOutOfOrder);
        };

        // Validate against the partial through an immutable borrow so
        // we stay clear of `clippy::indexing-slicing`.
        let Some(partial) = self.partials.get(idx) else {
            return Err(AisError::ReassemblyOutOfOrder);
        };
        if partial.fragment_count != header.fragment_count
            || partial.next_expected != header.fragment_number
        {
            let _ = self.partials.remove(idx);
            return Err(AisError::ReassemblyOutOfOrder);
        }

        // Append via a scoped mutable borrow; the borrow ends before
        // we touch `self.partials` again to remove on completion.
        let is_complete = {
            let Some(partial) = self.partials.get_mut(idx) else {
                return Err(AisError::ReassemblyOutOfOrder);
            };
            partial.payload.extend_from_slice(header.payload);
            partial.next_expected = partial.next_expected.saturating_add(1);
            partial.last_fill_bits = header.fill_bits;
            // Refresh the age stamp. A partial opened before the first
            // tick picks up a stamp on its first append after one.
            partial.last_touched_ms = self.now_ms;
            header.fragment_number == partial.fragment_count
        };

        if is_complete {
            let complete = self.partials.remove(idx);
            return Ok(Some(ReassembledPayload {
                payload: complete.payload,
                fill_bits: complete.last_fill_bits,
                is_own_ship: complete.is_own_ship,
            }));
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn header(
        frag_count: u8,
        frag_num: u8,
        seq_id: Option<u8>,
        channel: Option<u8>,
        payload: &[u8],
        fill_bits: u8,
    ) -> AivdmHeader<'_> {
        AivdmHeader {
            is_own_ship: false,
            fragment_count: frag_count,
            fragment_number: frag_num,
            sequential_id: seq_id,
            channel,
            payload,
            fill_bits,
        }
    }

    // -----------------------------------------------------------------
    // Single-fragment fast path
    // -----------------------------------------------------------------

    #[test]
    fn single_fragment_returns_immediately() {
        let mut r = AisReassembler::new();
        let h = header(1, 1, None, Some(b'A'), b"HELLO", 2);
        let out = r.feed_fragment(&h).unwrap().unwrap();
        assert_eq!(out.payload, b"HELLO");
        assert_eq!(out.fill_bits, 2);
        assert_eq!(r.in_flight(), 0);
    }

    #[test]
    fn single_fragment_propagates_is_own_ship() {
        let mut r = AisReassembler::new();
        let mut h = header(1, 1, None, Some(b'A'), b"X", 0);
        h.is_own_ship = true;
        let out = r.feed_fragment(&h).unwrap().unwrap();
        assert!(out.is_own_ship);
    }

    // -----------------------------------------------------------------
    // Two-fragment in-order happy path
    // -----------------------------------------------------------------

    #[test]
    fn two_fragments_in_order_complete() {
        let mut r = AisReassembler::new();
        assert!(r
            .feed_fragment(&header(2, 1, Some(3), Some(b'A'), b"AAA", 0))
            .unwrap()
            .is_none());
        assert_eq!(r.in_flight(), 1);
        let out = r
            .feed_fragment(&header(2, 2, Some(3), Some(b'A'), b"BBB", 2))
            .unwrap()
            .unwrap();
        assert_eq!(out.payload, b"AAABBB");
        assert_eq!(out.fill_bits, 2);
        assert_eq!(r.in_flight(), 0);
    }

    #[test]
    fn three_fragments_in_order_complete() {
        let mut r = AisReassembler::new();
        for (n, frag) in [(1u8, &b"AA"[..]), (2, &b"BB"[..])] {
            let h = header(3, n, Some(5), Some(b'A'), frag, 0);
            assert!(r.feed_fragment(&h).unwrap().is_none());
        }
        let out = r
            .feed_fragment(&header(3, 3, Some(5), Some(b'A'), b"CC", 4))
            .unwrap()
            .unwrap();
        assert_eq!(out.payload, b"AABBCC");
        assert_eq!(out.fill_bits, 4);
    }

    // -----------------------------------------------------------------
    // Out-of-order handling
    // -----------------------------------------------------------------

    #[test]
    fn out_of_order_yields_error_and_discards_partial() {
        let mut r = AisReassembler::new();
        assert!(r
            .feed_fragment(&header(3, 1, Some(1), Some(b'A'), b"AAA", 0))
            .unwrap()
            .is_none());
        // Skip fragment 2, jump to 3.
        match r.feed_fragment(&header(3, 3, Some(1), Some(b'A'), b"CCC", 0)) {
            Err(AisError::ReassemblyOutOfOrder) => {}
            other => panic!("expected ReassemblyOutOfOrder, got {other:?}"),
        }
        assert_eq!(r.in_flight(), 0);
    }

    #[test]
    fn fragment_without_prior_first_is_out_of_order() {
        let mut r = AisReassembler::new();
        match r.feed_fragment(&header(2, 2, Some(9), Some(b'A'), b"X", 0)) {
            Err(AisError::ReassemblyOutOfOrder) => {}
            other => panic!("expected ReassemblyOutOfOrder, got {other:?}"),
        }
    }

    #[test]
    fn inconsistent_fragment_count_is_out_of_order() {
        let mut r = AisReassembler::new();
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"A", 0))
            .unwrap();
        match r.feed_fragment(&header(3, 2, Some(1), Some(b'A'), b"B", 0)) {
            Err(AisError::ReassemblyOutOfOrder) => {}
            other => panic!("expected ReassemblyOutOfOrder, got {other:?}"),
        }
        assert_eq!(r.in_flight(), 0);
    }

    // -----------------------------------------------------------------
    // Same sequential id on the other channel is a different key
    // -----------------------------------------------------------------

    #[test]
    fn fragment_on_other_channel_is_out_of_order_and_keeps_partial() {
        let mut r = AisReassembler::new();
        r.feed_fragment(&header(2, 1, Some(4), Some(b'A'), b"A", 0))
            .unwrap();
        // Fragment 2 on B has no partial on (B, 4) — it is stray.
        match r.feed_fragment(&header(2, 2, Some(4), Some(b'B'), b"B", 0)) {
            Err(AisError::ReassemblyOutOfOrder) => {}
            other => panic!("expected ReassemblyOutOfOrder, got {other:?}"),
        }
        // The partial on (A, 4) is untouched and still completes.
        assert_eq!(r.in_flight(), 1);
        let out = r
            .feed_fragment(&header(2, 2, Some(4), Some(b'A'), b"a", 0))
            .unwrap()
            .unwrap();
        assert_eq!(out.payload, b"Aa");
        assert_eq!(r.in_flight(), 0);
    }

    // -----------------------------------------------------------------
    // Interleaving on different sequential IDs
    // -----------------------------------------------------------------

    #[test]
    fn interleaved_messages_on_different_seq_ids_both_complete() {
        let mut r = AisReassembler::new();
        // Two multi-fragment messages in flight, fragments arriving
        // interleaved: (1,seq=3), (1,seq=5), (2,seq=3), (2,seq=5).
        assert!(r
            .feed_fragment(&header(2, 1, Some(3), Some(b'A'), b"A3a", 0))
            .unwrap()
            .is_none());
        assert!(r
            .feed_fragment(&header(2, 1, Some(5), Some(b'A'), b"A5a", 0))
            .unwrap()
            .is_none());
        assert_eq!(r.in_flight(), 2);
        let m3 = r
            .feed_fragment(&header(2, 2, Some(3), Some(b'A'), b"A3b", 0))
            .unwrap()
            .unwrap();
        assert_eq!(m3.payload, b"A3aA3b");
        let m5 = r
            .feed_fragment(&header(2, 2, Some(5), Some(b'A'), b"A5b", 0))
            .unwrap()
            .unwrap();
        assert_eq!(m5.payload, b"A5aA5b");
        assert_eq!(r.in_flight(), 0);
    }

    // -----------------------------------------------------------------
    // Same sequential id on different channels: treated as separate
    // partials (both complete independently).
    // -----------------------------------------------------------------

    #[test]
    fn interleaved_messages_on_different_channels_both_complete() {
        let mut r = AisReassembler::new();
        // First fragment on A.
        r.feed_fragment(&header(2, 1, Some(7), Some(b'A'), b"Aa", 0))
            .unwrap();
        // First fragment on B with same seq_id — legitimately independent.
        r.feed_fragment(&header(2, 1, Some(7), Some(b'B'), b"Ba", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 2);
        let from_a = r
            .feed_fragment(&header(2, 2, Some(7), Some(b'A'), b"Ab", 0))
            .unwrap()
            .unwrap();
        let from_b = r
            .feed_fragment(&header(2, 2, Some(7), Some(b'B'), b"Bb", 0))
            .unwrap()
            .unwrap();
        assert_eq!(from_a.payload, b"AaAb");
        assert_eq!(from_b.payload, b"BaBb");
    }

    #[test]
    fn same_sequence_id_on_both_channels_completes_both() {
        // Regression: the second fragments arrive in the opposite order
        // to the first ones — (1,A), (1,B), (2,B), (2,A). A seq-id-only
        // lookup matched B's second fragment against A's partial, raised
        // a channel mismatch, and dropped both messages.
        let mut r = AisReassembler::new();
        assert!(r
            .feed_fragment(&header(2, 1, Some(7), Some(b'A'), b"Aa", 0))
            .unwrap()
            .is_none());
        assert!(r
            .feed_fragment(&header(2, 1, Some(7), Some(b'B'), b"Ba", 0))
            .unwrap()
            .is_none());
        assert_eq!(r.in_flight(), 2);
        let from_b = r
            .feed_fragment(&header(2, 2, Some(7), Some(b'B'), b"Bb", 0))
            .unwrap()
            .unwrap();
        assert_eq!(r.in_flight(), 1);
        let from_a = r
            .feed_fragment(&header(2, 2, Some(7), Some(b'A'), b"Ab", 0))
            .unwrap()
            .unwrap();
        assert_eq!(from_b.payload, b"BaBb");
        assert_eq!(from_a.payload, b"AaAb");
        assert_eq!(r.in_flight(), 0);
        assert!(r.take_pending_error().is_none());
    }

    // -----------------------------------------------------------------
    // Missing sequential_id on multi-fragment → MalformedWrapper
    // -----------------------------------------------------------------

    #[test]
    fn multi_fragment_without_seq_id_is_malformed() {
        let mut r = AisReassembler::new();
        match r.feed_fragment(&header(2, 1, None, Some(b'A'), b"X", 0)) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Malformed fragment numbering
    // -----------------------------------------------------------------

    #[test]
    fn zero_fragment_count_is_malformed() {
        let mut r = AisReassembler::new();
        match r.feed_fragment(&header(0, 1, None, Some(b'A'), b"X", 0)) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn zero_fragment_number_is_malformed() {
        let mut r = AisReassembler::new();
        match r.feed_fragment(&header(2, 0, Some(1), Some(b'A'), b"X", 0)) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn fragment_number_beyond_count_is_malformed() {
        let mut r = AisReassembler::new();
        match r.feed_fragment(&header(2, 3, Some(1), Some(b'A'), b"X", 0)) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Eviction surfaces ReassemblyTimeout on the next error poll
    // -----------------------------------------------------------------

    #[test]
    fn bounded_slots_evict_oldest_and_queue_timeout() {
        let mut r = AisReassembler::with_max_partials(2);
        // Fill the two slots with first-fragments on different seq_ids.
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.feed_fragment(&header(2, 1, Some(2), Some(b'A'), b"b", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 2);
        assert!(r.take_pending_error().is_none());

        // A third first-fragment triggers eviction of the oldest.
        r.feed_fragment(&header(2, 1, Some(3), Some(b'A'), b"c", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 2);

        // The eviction was reported.
        match r.take_pending_error() {
            Some(AisError::ReassemblyTimeout) => {}
            other => panic!("expected ReassemblyTimeout, got {other:?}"),
        }
        // Only one timeout is queued.
        assert!(r.take_pending_error().is_none());
    }

    // -----------------------------------------------------------------
    // Restarted-in-flight: fragment 1 for the same key replaces
    // any prior partial silently (real AIS behavior when an interrupted
    // multi-sentence message is followed by a fresh one).
    // -----------------------------------------------------------------

    #[test]
    fn duplicate_fragment_one_restarts_partial() {
        let mut r = AisReassembler::new();
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"old", 0))
            .unwrap();
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"new", 0))
            .unwrap();
        let out = r
            .feed_fragment(&header(2, 2, Some(1), Some(b'A'), b"tail", 0))
            .unwrap()
            .unwrap();
        assert_eq!(out.payload, b"newtail");
    }

    // -----------------------------------------------------------------
    // with_max_partials(0) clamps to 1 — still functional
    // -----------------------------------------------------------------

    #[test]
    fn zero_max_partials_clamps_to_one() {
        let mut r = AisReassembler::with_max_partials(0);
        assert_eq!(r.max_partials(), 1);
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 1);
    }

    // -----------------------------------------------------------------
    // Clock-based timeout: tick, then feed
    // -----------------------------------------------------------------

    #[test]
    fn tick_expires_partial_past_timeout() {
        let mut r = AisReassembler::with_timeout_ms(16, 60_000);
        r.tick(1_000);
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 1);

        // 60.001 s later — still within timeout window.
        r.tick(1_000 + 60_000);
        assert_eq!(r.in_flight(), 1);
        assert!(r.take_pending_error().is_none());

        // One ms past timeout — expire.
        r.tick(1_000 + 60_001);
        assert_eq!(r.in_flight(), 0);
        match r.take_pending_error() {
            Some(AisError::ReassemblyTimeout) => {}
            other => panic!("expected ReassemblyTimeout, got {other:?}"),
        }
    }

    #[test]
    fn tick_expiring_multiple_partials_queues_multiple_timeouts() {
        let mut r = AisReassembler::with_timeout_ms(16, 1_000);
        r.tick(100);
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.tick(200);
        r.feed_fragment(&header(2, 1, Some(2), Some(b'A'), b"b", 0))
            .unwrap();
        r.tick(300);
        r.feed_fragment(&header(2, 1, Some(3), Some(b'A'), b"c", 0))
            .unwrap();
        assert_eq!(r.in_flight(), 3);

        // At t = 1500, all three are past their timeout (age > 1000).
        r.tick(1_500);
        assert_eq!(r.in_flight(), 0);
        // One ReassemblyTimeout per expired partial — the queue
        // preserves the count rather than collapsing to one signal.
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
        assert!(r.take_pending_error().is_none());
    }

    #[test]
    fn tick_without_timeout_set_is_noop() {
        let mut r = AisReassembler::new(); // no timeout
        r.tick(0);
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.tick(u64::MAX); // the end of time — should still not expire.
        assert_eq!(r.in_flight(), 1);
        assert!(r.take_pending_error().is_none());
    }

    #[test]
    fn partials_opened_before_the_first_tick_are_immune() {
        let mut r = AisReassembler::with_timeout_ms(16, 100);
        // No tick yet — no stamp.
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.tick(1_000_000);
        assert_eq!(r.in_flight(), 1);
        assert!(r.take_pending_error().is_none());
    }

    #[test]
    fn append_after_a_tick_stamps_a_partial_opened_before_it() {
        let mut r = AisReassembler::with_timeout_ms(16, 100);
        r.feed_fragment(&header(3, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.tick(1_000);
        r.feed_fragment(&header(3, 2, Some(1), Some(b'A'), b"b", 0))
            .unwrap();
        // Stamped at 1_000 on the append; 101 ms later it is evicted.
        r.tick(1_101);
        assert_eq!(r.in_flight(), 0);
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
    }

    #[test]
    fn append_refreshes_partial_timestamp() {
        // A multi-fragment message that arrives slowly: fragment 1 at
        // t=0, fragment 2 at t=500 (inside the 1000ms timeout). The
        // append must refresh the stamp so a later tick(t=1200)
        // computes age against 500, not 0.
        let mut r = AisReassembler::with_timeout_ms(16, 1_000);
        r.tick(0);
        r.feed_fragment(&header(3, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.tick(500);
        r.feed_fragment(&header(3, 2, Some(1), Some(b'A'), b"b", 0))
            .unwrap();

        // At t=1200 age-from-last-touch is 700 < 1000 — still alive.
        r.tick(1_200);
        assert_eq!(r.in_flight(), 1);
        assert!(r.take_pending_error().is_none());

        // Fragment 3 completes the message.
        r.tick(1_300);
        let out = r
            .feed_fragment(&header(3, 3, Some(1), Some(b'A'), b"c", 2))
            .unwrap()
            .unwrap();
        assert_eq!(out.payload, b"abc");
    }

    #[test]
    fn set_timeout_ms_toggles_eviction_policy() {
        let mut r = AisReassembler::new();
        assert_eq!(r.timeout_ms(), None);
        r.set_timeout_ms(Some(100));
        assert_eq!(r.timeout_ms(), Some(100));
        r.set_timeout_ms(None);
        assert_eq!(r.timeout_ms(), None);
    }

    // -----------------------------------------------------------------
    // Slot eviction still queues per-eviction timeouts (now via VecDeque).
    // -----------------------------------------------------------------

    #[test]
    fn slot_eviction_queues_one_timeout_per_eviction() {
        let mut r = AisReassembler::with_max_partials(1);
        // Three first-fragments on different seq_ids → two evictions.
        r.feed_fragment(&header(2, 1, Some(1), Some(b'A'), b"a", 0))
            .unwrap();
        r.feed_fragment(&header(2, 1, Some(2), Some(b'A'), b"b", 0))
            .unwrap();
        r.feed_fragment(&header(2, 1, Some(3), Some(b'A'), b"c", 0))
            .unwrap();
        // Two evictions; two distinct timeout signals queued.
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
        assert!(matches!(
            r.take_pending_error(),
            Some(AisError::ReassemblyTimeout)
        ));
        assert!(r.take_pending_error().is_none());
    }
}
