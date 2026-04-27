//! Top-level AIS message enum and dispatcher.
//!
//! Two entry points, layered:
//!
//! - [`decode_message`] — bit-level primitive. Takes an already-armored
//!   bit buffer (as produced by [`crate::armor::decode`] or by an
//!   eventual multi-sentence reassembler), peeks the 6-bit `msg_type`
//!   field, and routes to the appropriate per-type decoder. The
//!   `is_own_ship` flag is envelope metadata the caller supplies from
//!   the [`AivdmHeader`](crate::AivdmHeader).
//!
//! - [`decode`] — single-fragment convenience over a [`RawSentence`].
//!   Runs the full `parse_aivdm_wrapper → armor::decode → decode_message`
//!   pipeline in one call. Intended for callers that know their input
//!   is single-fragment (`fragment_count == 1`); multi-fragment inputs
//!   will typically produce [`AisError::PayloadTooShort`] because the
//!   partial payload is too short for its declared type. Multi-sentence
//!   reassembly is the job of the (future) `AisFragmentParser`.

use alloc::vec::Vec;

use marlin_nmea_envelope::RawSentence;

use crate::{
    armor, decode_extended_position_report_b, decode_position_report_a, decode_position_report_b,
    decode_static_and_voyage_a, decode_static_data_b, parse_aivdm_wrapper, AisError, BitReader,
    ExtendedPositionReportB, PositionReportA, PositionReportB, StaticAndVoyageA, StaticDataB,
    StaticDataB24A, StaticDataB24B,
};

/// A fully decoded AIS message with envelope metadata.
///
/// The payload sits in `body`; `is_own_ship` carries the `!AIVDM` vs
/// `!AIVDO` distinction from the envelope (PRD §A7). The split exists
/// because the two pieces have different sources — `body` is
/// bit-level decode, `is_own_ship` is a wrapper-tag boolean — and
/// because future reassembly metadata (channel, fragment count) would
/// naturally live alongside `is_own_ship` if it were ever exposed.
#[derive(Debug, Clone, PartialEq)]
pub struct AisMessage {
    /// `true` if the source sentence was `!AIVDO` (own-ship loopback),
    /// `false` if `!AIVDM` (received from another vessel). The AIS
    /// binary payload layout is identical in both cases; only the
    /// envelope tag differs.
    pub is_own_ship: bool,
    /// The decoded message body.
    pub body: AisMessageBody,
}

/// The decoded AIS payload, dispatched on the 6-bit `msg_type` field.
///
/// `#[non_exhaustive]` so additional message types (Type 19, Type 4,
/// Type 21, ...) can be added as typed variants in minor versions
/// without a breaking change. Types this crate does not yet decode
/// are surfaced as [`Self::Other`] with the raw bit buffer preserved.
///
/// See PRD §5.3 for the normative list of variants targeted by v0.1.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AisMessageBody {
    /// Type 1 — Class A scheduled position report.
    Type1(PositionReportA),
    /// Type 2 — Class A assigned scheduled position report.
    Type2(PositionReportA),
    /// Type 3 — Class A special position report (interrogation response).
    Type3(PositionReportA),
    /// Type 5 — Class A static and voyage data.
    Type5(StaticAndVoyageA),
    /// Type 18 — Class B CS position report.
    Type18(PositionReportB),
    /// Type 19 — Class B extended position report (Type 18 plus the
    /// Class A static tail: vessel name, ship type, dimensions, EPFD).
    Type19(ExtendedPositionReportB),
    /// Type 24 Part A — Class B static, vessel name.
    Type24A(StaticDataB24A),
    /// Type 24 Part B — Class B static, ship type + dimensions + callsign.
    Type24B(StaticDataB24B),
    /// Any message type this crate does not yet decode. The raw bit
    /// buffer is preserved so callers can plug in their own decoder or
    /// log the payload for later inspection.
    Other {
        /// 6-bit message-type field from the start of the payload.
        msg_type: u8,
        /// The unconsumed bit buffer (caller owns, may pass to a
        /// custom [`BitReader`]).
        raw_payload: Vec<u8>,
        /// Number of valid bits in `raw_payload` (may be less than
        /// `raw_payload.len() * 8` when fill bits were applied).
        total_bits: usize,
    },
}

/// Minimum bits needed to read a `msg_type` field.
const MSG_TYPE_BITS: usize = 6;

/// Bit-level AIS message dispatcher.
///
/// Peeks the 6-bit `msg_type` at the head of `bits`, routes to the
/// per-type decoder, and wraps the result in [`AisMessage`] with the
/// caller-provided `is_own_ship` flag.
///
/// Each per-type decoder independently re-consumes the
/// `msg_type + repeat` prefix; the peek here does not advance a
/// shared cursor.
///
/// # Errors
///
/// - [`AisError::PayloadTooShort`] if `total_bits < 6`
///   (can't read `msg_type`) or if the per-type decoder's own minimum
///   is not met.
/// - Any error surfaced by the selected per-type decoder.
pub fn decode_message(
    bits: &[u8],
    total_bits: usize,
    is_own_ship: bool,
) -> Result<AisMessage, AisError> {
    if total_bits < MSG_TYPE_BITS {
        return Err(AisError::PayloadTooShort);
    }
    // Use BitReader to honour the saturating-zero contract even on
    // pathological inputs (empty slice with total_bits >= 6). The
    // reader's own bounds check prevents out-of-bounds indexing.
    #[allow(clippy::cast_possible_truncation)]
    let msg_type = (BitReader::new(bits, total_bits).u(MSG_TYPE_BITS) & 0x3F) as u8;

    let body = match msg_type {
        1 => AisMessageBody::Type1(decode_position_report_a(bits, total_bits)?),
        2 => AisMessageBody::Type2(decode_position_report_a(bits, total_bits)?),
        3 => AisMessageBody::Type3(decode_position_report_a(bits, total_bits)?),
        5 => AisMessageBody::Type5(decode_static_and_voyage_a(bits, total_bits)?),
        18 => AisMessageBody::Type18(decode_position_report_b(bits, total_bits)?),
        19 => AisMessageBody::Type19(decode_extended_position_report_b(bits, total_bits)?),
        24 => match decode_static_data_b(bits, total_bits)? {
            StaticDataB::PartA(a) => AisMessageBody::Type24A(a),
            StaticDataB::PartB(b) => AisMessageBody::Type24B(b),
            StaticDataB::Reserved { .. } => other_payload(msg_type, bits, total_bits),
        },
        _ => other_payload(msg_type, bits, total_bits),
    };

    Ok(AisMessage { is_own_ship, body })
}

/// Decode a single-fragment AIS sentence end-to-end.
///
/// Runs the full pipeline: envelope wrapper parse → armor decode →
/// [`decode_message`]. Intended for `fragment_count == 1` inputs.
/// Multi-fragment inputs are not rejected explicitly — their partial
/// payload usually decodes to [`AisError::PayloadTooShort`] because
/// the per-type decoders require a specific minimum bit count. Use
/// the reassembly parser (landing in a later commit) to handle
/// multi-fragment messages correctly.
///
/// # Errors
///
/// - [`AisError::NotAnAisSentence`] if `raw` is not `!VDM`/`!VDO`.
/// - [`AisError::MalformedWrapper`] on a structurally bad AIVDM header.
/// - [`AisError::InvalidArmorChar`] / [`AisError::InvalidFillBits`] /
///   [`AisError::PayloadTooShort`] / [`AisError::PayloadTooLong`] from
///   [`armor::decode`].
/// - Any error from [`decode_message`].
pub fn decode(raw: &RawSentence<'_>) -> Result<AisMessage, AisError> {
    let header = parse_aivdm_wrapper(raw)?;
    let (bits, total_bits) = armor::decode(header.payload, header.fill_bits)?;
    decode_message(&bits, total_bits, header.is_own_ship)
}

/// Build an `Other` variant by copying the bit buffer out. Factored
/// so the "unknown" and "reserved Type 24 part" paths share a body.
fn other_payload(msg_type: u8, bits: &[u8], total_bits: usize) -> AisMessageBody {
    AisMessageBody::Other {
        msg_type,
        raw_payload: bits.to_vec(),
        total_bits,
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
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
mod tests {
    use super::*;
    use crate::testing::{build_aivdm, parse_raw, BitWriter};

    // -----------------------------------------------------------------
    // Helpers: synthetic payloads for dispatcher tests
    // -----------------------------------------------------------------

    /// Build a minimal 168-bit Type 1/2/3 payload with a caller-chosen
    /// `msg_type`. MMSI is the only interesting field; all others are
    /// zero or sentinel.
    fn build_class_a_position_report(msg_type: u8, mmsi: u32) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, u64::from(msg_type));
        w.u(2, 0); // repeat
        w.u(30, u64::from(mmsi));
        w.u(4, 0); // nav status
        w.i(8, -128); // rot sentinel
        w.u(10, 1023); // sog sentinel
        w.b(false);
        w.i(28, 0); // lon
        w.i(27, 0); // lat
        w.u(12, 3600); // cog sentinel
        w.u(9, 511); // heading sentinel
        w.u(6, 60); // timestamp N/A
        w.u(2, 0);
        w.u(3, 0);
        w.b(false);
        w.u(19, 0);
        w.finish()
    }

    fn build_type5(mmsi: u32) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 5); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(mmsi));
        // Pad remaining 386 bits with zeros. BitWriter tops out at u(64,..)
        // per call, so split into chunks.
        for _ in 0..6 {
            w.u(64, 0);
        }
        w.u(2, 0);
        w.finish()
    }

    fn build_type18(mmsi: u32) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 18);
        w.u(2, 0);
        w.u(30, u64::from(mmsi));
        w.u(8, 0); // reserved
        w.u(10, 1023); // sog sentinel
        w.b(false);
        w.i(28, 0);
        w.i(27, 0);
        w.u(12, 3600);
        w.u(9, 511);
        w.u(6, 60);
        w.u(2, 0);
        for _ in 0..7 {
            w.b(false);
        }
        w.u(20, 0);
        w.finish()
    }

    fn build_type24_part_a(mmsi: u32) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, u64::from(mmsi));
        w.u(2, 0); // part A
                   // 20 6-bit @-chars + 8 pad = 128 bits.
        for _ in 0..16 {
            w.u(8, 0);
        }
        w.finish()
    }

    fn build_type24_part_b(mmsi: u32) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, u64::from(mmsi));
        w.u(2, 1); // part B
        w.u(8, 0); // ship type
                   // 7+7 6-bit chars = 84 bits; then 30 dim bits; then 6 spare.
        for _ in 0..15 {
            w.u(8, 0);
        }
        w.finish()
    }

    /// Build an arbitrary-msg-type 168-bit payload (used for unknown /
    /// Type 19 routing tests).
    fn build_unknown(msg_type: u8) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, u64::from(msg_type));
        w.u(2, 0);
        // Pad to 168 bits total (160 more).
        for _ in 0..2 {
            w.u(64, 0);
        }
        w.u(32, 0);
        w.finish()
    }

    // -----------------------------------------------------------------
    // Dispatcher routes each known type to the right variant
    // -----------------------------------------------------------------

    #[test]
    fn routes_type_1_to_type1_variant() {
        let (bits, total) = build_class_a_position_report(1, 111);
        let msg = decode_message(&bits, total, false).unwrap();
        assert!(!msg.is_own_ship);
        match msg.body {
            AisMessageBody::Type1(pra) => assert_eq!(pra.mmsi, 111),
            other => panic!("expected Type1, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_2_to_type2_variant() {
        let (bits, total) = build_class_a_position_report(2, 222);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type2(pra) => assert_eq!(pra.mmsi, 222),
            other => panic!("expected Type2, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_3_to_type3_variant() {
        let (bits, total) = build_class_a_position_report(3, 333);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type3(pra) => assert_eq!(pra.mmsi, 333),
            other => panic!("expected Type3, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_5_to_type5_variant() {
        let (bits, total) = build_type5(555);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type5(sva) => assert_eq!(sva.mmsi, 555),
            other => panic!("expected Type5, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_18_to_type18_variant() {
        let (bits, total) = build_type18(18_000);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type18(prb) => assert_eq!(prb.mmsi, 18_000),
            other => panic!("expected Type18, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_24_part_a_to_type24a_variant() {
        let (bits, total) = build_type24_part_a(24_001);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type24A(a) => assert_eq!(a.mmsi, 24_001),
            other => panic!("expected Type24A, got {other:?}"),
        }
    }

    #[test]
    fn routes_type_24_part_b_to_type24b_variant() {
        let (bits, total) = build_type24_part_b(24_002);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type24B(b) => assert_eq!(b.mmsi, 24_002),
            other => panic!("expected Type24B, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Type 19 routes to its typed variant
    // -----------------------------------------------------------------

    #[test]
    fn routes_type_19_to_type19_variant() {
        // Minimum 312-bit payload with just msg_type + mmsi set.
        let mut w = BitWriter::new();
        w.u(6, 19);
        w.u(2, 0);
        w.u(30, 19_000);
        // Pad remaining 274 bits.
        for _ in 0..4 {
            w.u(64, 0);
        }
        w.u(18, 0);
        let (bits, total) = w.finish();
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Type19(eprb) => assert_eq!(eprb.mmsi, 19_000),
            other => panic!("expected Type19, got {other:?}"),
        }
    }

    #[test]
    fn type_19_short_payload_is_rejected() {
        // 168 bits is enough for Type 18 but not Type 19 (needs 312).
        let (bits, total) = build_unknown(19);
        match decode_message(&bits, total, false) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Unknown message types route to Other
    // -----------------------------------------------------------------

    #[test]
    fn unknown_type_goes_to_other_with_raw_payload() {
        let (bits, total) = build_unknown(42);
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Other {
                msg_type,
                raw_payload,
                total_bits,
            } => {
                assert_eq!(msg_type, 42);
                assert_eq!(raw_payload, bits);
                assert_eq!(total_bits, total);
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Reserved Type 24 part code routes to Other (falls through — the
    // part-A/B dispatcher surfaces Reserved, and we bucket it into
    // Other so callers only need one catch-all arm for unknowns).
    // -----------------------------------------------------------------

    #[test]
    fn type_24_reserved_part_routes_to_other() {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, 99);
        w.u(2, 2); // reserved part code
                   // Pad to 168 bits: 128 more.
        w.u(64, 0);
        w.u(64, 0);
        let (bits, total) = w.finish();
        match decode_message(&bits, total, false).unwrap().body {
            AisMessageBody::Other { msg_type, .. } => assert_eq!(msg_type, 24),
            other => panic!("expected Other for reserved Type 24 part, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // is_own_ship propagates unchanged
    // -----------------------------------------------------------------

    #[test]
    fn is_own_ship_propagates_true() {
        let (bits, total) = build_class_a_position_report(1, 1);
        let msg = decode_message(&bits, total, true).unwrap();
        assert!(msg.is_own_ship);
    }

    #[test]
    fn is_own_ship_propagates_false() {
        let (bits, total) = build_class_a_position_report(1, 1);
        let msg = decode_message(&bits, total, false).unwrap();
        assert!(!msg.is_own_ship);
    }

    // -----------------------------------------------------------------
    // Truncated input: too few bits to read msg_type
    // -----------------------------------------------------------------

    #[test]
    fn rejects_input_shorter_than_msg_type_field() {
        let buf = [0u8];
        match decode_message(&buf, 4, false) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    #[test]
    fn accepts_msg_type_peek_at_exactly_six_bits_even_on_short_type() {
        // 6 bits gets us msg_type = 5 (0b000101). Type 5 needs 424
        // bits, so the per-type decoder must then reject with
        // PayloadTooShort — dispatcher must not panic between peek
        // and dispatch.
        let buf = [0b0001_0100]; // top 6 bits = 000101 = 5
        match decode_message(&buf, 6, false) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort from type-5 decoder, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Empty slice with bogus total_bits does not panic
    // -----------------------------------------------------------------

    #[test]
    fn empty_slice_with_nonzero_total_bits_is_panic_free() {
        // BitReader saturates past-end reads to zero, so msg_type=0
        // is read and routed to Other.
        let buf: &[u8] = &[];
        match decode_message(buf, 168, false).unwrap().body {
            AisMessageBody::Other { msg_type, .. } => assert_eq!(msg_type, 0),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // decode(&RawSentence): end-to-end pipeline with classic Annex 5
    // fixture (known Type 1 MMSI 244 708 736)
    // -----------------------------------------------------------------

    #[test]
    fn decode_runs_full_pipeline_on_classic_type1_fixture() {
        let bytes = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        let raw = parse_raw(&bytes);
        let msg = decode(&raw).unwrap();
        assert!(!msg.is_own_ship);
        match msg.body {
            AisMessageBody::Type1(pra) => assert_eq!(pra.mmsi, 244_708_736),
            other => panic!("expected Type1, got {other:?}"),
        }
    }

    #[test]
    fn decode_sets_is_own_ship_on_aivdo() {
        // Same payload, wrapped as AIVDO instead of AIVDM.
        let bytes = crate::testing::build_with_address(
            b"!",
            b"AIVDO",
            b"1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0",
        );
        let raw = parse_raw(&bytes);
        let msg = decode(&raw).unwrap();
        assert!(msg.is_own_ship);
        assert!(matches!(msg.body, AisMessageBody::Type1(_)));
    }

    #[test]
    fn decode_forwards_non_ais_sentence_error() {
        let bytes =
            crate::testing::build_with_address(b"$", b"GPGGA", b"1,2,3,4,5,6,7,8,9,10,11,12,13,14");
        let raw = parse_raw(&bytes);
        match decode(&raw) {
            Err(AisError::NotAnAisSentence) => {}
            other => panic!("expected NotAnAisSentence, got {other:?}"),
        }
    }

    #[test]
    fn decode_forwards_malformed_wrapper_error() {
        // AIVDM with too few fields.
        let bytes = crate::testing::build_with_address(b"!", b"AIVDM", b"1,1,A");
        let raw = parse_raw(&bytes);
        match decode(&raw) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn decode_forwards_invalid_armor_char_error() {
        // The character '~' (0x7E) is outside the AIS armor alphabet.
        let bytes = build_aivdm(1, 1, None, Some(b'A'), b"~~~", 0);
        let raw = parse_raw(&bytes);
        match decode(&raw) {
            Err(AisError::InvalidArmorChar(0x7E)) => {}
            other => panic!("expected InvalidArmorChar(0x7E), got {other:?}"),
        }
    }
}
