//! AIVDM/AIVDO wrapper parsing — extract header fields from a
//! [`RawSentence`].
//!
//! # Wire format
//!
//! ```text
//! !AIVDM,<frag_count>,<frag_num>,<seq_id>,<channel>,<payload>,<fill_bits>*hh
//! !AIVDO,...  (same shape; indicates own-ship data)
//! ```
//!
//! - `frag_count` (1..=9): number of `!AIVDM` sentences that make up
//!   the current AIS message. 1 for single-fragment messages.
//! - `frag_num` (1..=`frag_count`): 1-based fragment index.
//! - `seq_id` (0..=9 or empty): sequential message ID used to thread
//!   fragments of the same multi-sentence message together. Empty for
//!   single-fragment sentences.
//! - `channel` (`A`, `B`, or empty): VHF channel the message arrived on.
//! - `payload`: ASCII-armored 6-bit encoded message body.
//! - `fill_bits` (0..=5): number of padding bits at the end of the
//!   payload's bit stream.

use marlin_nmea_envelope::RawSentence;

use crate::AisError;

/// Fields extracted from an AIVDM/AIVDO wrapper.
///
/// Zero-copy: `payload` borrows from the [`RawSentence`], which in turn
/// borrows from the parser's buffer. Copy the payload out if you need
/// to retain it across further parser calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AivdmHeader<'a> {
    /// `true` if the sentence was `!AIVDO` (own-ship), `false` for
    /// `!AIVDM` (received from another vessel).
    pub is_own_ship: bool,
    /// Number of sentences in the underlying AIS message (1 for
    /// single-sentence messages).
    pub fragment_count: u8,
    /// 1-based index of this sentence within a multi-sentence message.
    pub fragment_number: u8,
    /// Sequential message ID for multi-sentence reassembly. `None`
    /// for single-fragment messages.
    pub sequential_id: Option<u8>,
    /// VHF channel (typically `b'A'` or `b'B'`). `None` if the
    /// transmitter didn't report one.
    pub channel: Option<u8>,
    /// ASCII-armored payload bytes. Feed to [`crate::armor::decode`].
    pub payload: &'a [u8],
    /// Trailing fill bits in the payload (0..=5).
    pub fill_bits: u8,
}

/// Minimum fields in an AIVDM wrapper: `frag_count, frag_num, seq_id,
/// channel, payload, fill_bits`.
const AIVDM_MIN_FIELDS: usize = 6;

/// Parse the AIVDM/AIVDO wrapper fields out of a [`RawSentence`].
///
/// Does **not** decode the armored payload — use
/// [`crate::armor::decode`] on [`AivdmHeader::payload`] to get the bit
/// stream, then [`crate::BitReader`] to extract typed fields.
///
/// # Errors
///
/// - [`AisError::NotAnAisSentence`] if the envelope isn't a `!VDM` or
///   `!VDO` encapsulation sentence.
/// - [`AisError::MalformedWrapper`] if the wrapper is missing fields,
///   has a non-numeric fragment count/number, or has a channel field
///   longer than one byte.
#[allow(clippy::indexing_slicing)] // field count validated above
pub fn parse_aivdm_wrapper<'a>(raw: &'a RawSentence<'a>) -> Result<AivdmHeader<'a>, AisError> {
    // AIS encapsulation: start delimiter must be '!', sentence type
    // must be VDM (received) or VDO (own-ship).
    if raw.start_delimiter != b'!' {
        return Err(AisError::NotAnAisSentence);
    }
    let is_own_ship = match raw.sentence_type {
        "VDM" => false,
        "VDO" => true,
        _ => return Err(AisError::NotAnAisSentence),
    };

    let f = raw.fields.as_slice();
    if f.len() < AIVDM_MIN_FIELDS {
        return Err(AisError::MalformedWrapper);
    }

    let fragment_count = parse_small_u8(f[0])?;
    let fragment_number = parse_small_u8(f[1])?;
    let sequential_id = if f[2].is_empty() {
        None
    } else {
        Some(parse_small_u8(f[2])?)
    };
    let channel = match f[3] {
        [] => None,
        [b] => Some(*b),
        _ => return Err(AisError::MalformedWrapper),
    };
    let payload = f[4];
    let fill_bits = parse_small_u8(f[5])?;

    Ok(AivdmHeader {
        is_own_ship,
        fragment_count,
        fragment_number,
        sequential_id,
        channel,
        payload,
        fill_bits,
    })
}

/// Parse an ASCII decimal byte slice as `u8`. Used for fragment count,
/// fragment number, sequential id, fill bits. All expected to be
/// single digits (0..=9) but the parser accepts up to 3 digits for
/// robustness.
fn parse_small_u8(bytes: &[u8]) -> Result<u8, AisError> {
    if bytes.is_empty() {
        return Err(AisError::MalformedWrapper);
    }
    let s = core::str::from_utf8(bytes).map_err(|_| AisError::MalformedWrapper)?;
    s.parse::<u8>().map_err(|_| AisError::MalformedWrapper)
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
    use crate::testing::{build_aivdm, parse_raw};

    // -----------------------------------------------------------------
    // Single-fragment !AIVDM
    // -----------------------------------------------------------------

    #[test]
    fn parses_classic_single_fragment_aivdm() {
        // Classic Type 1 position report fixture.
        let bytes = build_aivdm(1, 1, None, Some(b'A'), b"13aGmP0P00PD;88MD5MTDww@2<0L", 0);
        let raw = parse_raw(&bytes);
        let h = parse_aivdm_wrapper(&raw).expect("parse");

        assert!(!h.is_own_ship);
        assert_eq!(h.fragment_count, 1);
        assert_eq!(h.fragment_number, 1);
        assert_eq!(h.sequential_id, None);
        assert_eq!(h.channel, Some(b'A'));
        assert_eq!(h.payload, b"13aGmP0P00PD;88MD5MTDww@2<0L");
        assert_eq!(h.fill_bits, 0);
    }

    // -----------------------------------------------------------------
    // AIVDO sentences → is_own_ship
    // -----------------------------------------------------------------

    #[test]
    fn aivdo_marks_is_own_ship_true() {
        let bytes = crate::testing::build_with_address(
            b"!",
            b"AIVDO",
            b"1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0",
        );
        let raw = parse_raw(&bytes);
        let h = parse_aivdm_wrapper(&raw).expect("parse");
        assert!(h.is_own_ship);
    }

    // -----------------------------------------------------------------
    // Multi-fragment header (first fragment of a 2-part message)
    // -----------------------------------------------------------------

    #[test]
    fn parses_multi_fragment_header() {
        let bytes = build_aivdm(2, 1, Some(3), Some(b'B'), b"some_payload", 2);
        let raw = parse_raw(&bytes);
        let h = parse_aivdm_wrapper(&raw).expect("parse");
        assert_eq!(h.fragment_count, 2);
        assert_eq!(h.fragment_number, 1);
        assert_eq!(h.sequential_id, Some(3));
        assert_eq!(h.channel, Some(b'B'));
        assert_eq!(h.fill_bits, 2);
    }

    // -----------------------------------------------------------------
    // Optional-field handling
    // -----------------------------------------------------------------

    #[test]
    fn empty_channel_yields_none() {
        let bytes = build_aivdm(1, 1, None, None, b"A", 0);
        let raw = parse_raw(&bytes);
        let h = parse_aivdm_wrapper(&raw).expect("parse");
        assert_eq!(h.channel, None);
    }

    #[test]
    fn empty_sequential_id_on_single_fragment_is_none() {
        let bytes = build_aivdm(1, 1, None, Some(b'A'), b"A", 0);
        let raw = parse_raw(&bytes);
        let h = parse_aivdm_wrapper(&raw).expect("parse");
        assert_eq!(h.sequential_id, None);
    }

    // -----------------------------------------------------------------
    // Error paths
    // -----------------------------------------------------------------

    #[test]
    fn rejects_non_ais_sentence_types() {
        // $GPGGA is not an AIS sentence.
        let body =
            crate::testing::build_with_address(b"$", b"GPGGA", b"1,2,3,4,5,6,7,8,9,10,11,12,13,14");
        let raw = parse_raw(&body);
        match parse_aivdm_wrapper(&raw) {
            Err(AisError::NotAnAisSentence) => {}
            other => panic!("expected NotAnAisSentence, got {other:?}"),
        }
    }

    #[test]
    fn rejects_bang_with_non_aisvdm_sentence_type() {
        // Different encapsulation type using '!' (hypothetical).
        let bytes = crate::testing::build_with_address(b"!", b"BBMMM", b"1,1,,A,X,0");
        let raw = parse_raw(&bytes);
        match parse_aivdm_wrapper(&raw) {
            Err(AisError::NotAnAisSentence) => {}
            other => panic!("expected NotAnAisSentence, got {other:?}"),
        }
    }

    #[test]
    fn rejects_wrapper_missing_fields() {
        // Only 3 fields — need at least 6.
        let bytes = crate::testing::build_with_address(b"!", b"AIVDM", b"1,1,A");
        let raw = parse_raw(&bytes);
        match parse_aivdm_wrapper(&raw) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_numeric_fragment_count() {
        let bytes = crate::testing::build_with_address(b"!", b"AIVDM", b"xx,1,,A,X,0");
        let raw = parse_raw(&bytes);
        match parse_aivdm_wrapper(&raw) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }

    #[test]
    fn rejects_multi_byte_channel_field() {
        let bytes = crate::testing::build_with_address(b"!", b"AIVDM", b"1,1,,AB,X,0");
        let raw = parse_raw(&bytes);
        match parse_aivdm_wrapper(&raw) {
            Err(AisError::MalformedWrapper) => {}
            other => panic!("expected MalformedWrapper, got {other:?}"),
        }
    }
}
