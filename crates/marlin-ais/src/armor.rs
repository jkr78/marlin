//! AIS ASCII armor — convert `!AIVDM` payload characters to a
//! bit-packed byte buffer for consumption by [`crate::BitReader`].
//!
//! # Encoding
//!
//! Each character in an AIS payload carries **6 bits** of data, packed
//! into the printable ASCII range to avoid NMEA-reserved bytes. The
//! decode rule (ITU-R M.1371-5 §8.2.4):
//!
//! | ASCII range | Value range |
//! | --- | --- |
//! | `0`..=`W` (0x30..=0x57) | 0..=39 |
//! | `` ` ``..=`w` (0x60..=0x77) | 40..=63 |
//!
//! Everything else is invalid and produces [`AisError::InvalidArmorChar`].
//!
//! # Fill bits
//!
//! The AIS binary message size is frequently not a multiple of 6, so
//! the last character may have fewer than 6 significant bits. The
//! `!AIVDM` wrapper declares how many trailing bits of the last
//! character are padding (`fill_bits`, 0..=5). [`decode`] strips those
//! from the returned `total_bits`.

use alloc::vec::Vec;

use crate::AisError;

/// Decode an AIS armored payload into a dense bit buffer.
///
/// Returns the packed byte buffer (8 bits per byte, MSB first across
/// byte boundaries) and the count of valid bits (`total_bits`). Pass
/// both to [`BitReader::new`](crate::BitReader::new).
///
/// # Errors
///
/// - [`AisError::InvalidArmorChar`] if any byte in `payload` is not
///   in the AIS armor alphabet.
/// - [`AisError::InvalidFillBits`] if `fill_bits > 5`.
/// - [`AisError::PayloadTooShort`] if `fill_bits` exceeds the
///   payload's bit count.
/// - [`AisError::PayloadTooLong`] if `payload.len() × 6` overflows
///   `usize` (unreachable in practice; guarded for robustness).
pub fn decode(payload: &[u8], fill_bits: u8) -> Result<(Vec<u8>, usize), AisError> {
    if fill_bits > 5 {
        return Err(AisError::InvalidFillBits(fill_bits));
    }
    let gross_bits = payload
        .len()
        .checked_mul(6)
        .ok_or(AisError::PayloadTooLong)?;
    let total_bits = gross_bits
        .checked_sub(fill_bits as usize)
        .ok_or(AisError::PayloadTooShort)?;

    // Allocate a tight buffer. `div_ceil(n, 8)` rounds up.
    let byte_count = total_bits.div_ceil(8);
    let mut bits = alloc::vec![0u8; byte_count];

    let mut bit_pos = 0usize;
    for &c in payload {
        let v = decode_char(c)?;
        // Each 6-bit value feeds MSB-first into the output stream.
        for shift in (0..6_u32).rev() {
            if bit_pos >= total_bits {
                break;
            }
            let bit = (v >> shift) & 1;
            let byte_idx = bit_pos / 8;
            let bit_in_byte = 7 - (bit_pos % 8);
            // byte_idx < bits.len() because total_bits.div_ceil(8) bounds it.
            if let Some(b) = bits.get_mut(byte_idx) {
                *b |= bit << bit_in_byte;
            }
            bit_pos = bit_pos.saturating_add(1);
        }
    }

    Ok((bits, total_bits))
}

/// Decode a single AIS armor character to its 6-bit value.
///
/// Exposed publicly so downstream crates doing custom armor work can
/// reuse the decode table; typical callers use [`decode`] instead.
///
/// # Errors
///
/// [`AisError::InvalidArmorChar`] if the byte is outside the AIS
/// armor alphabet.
#[inline]
pub fn decode_char(c: u8) -> Result<u8, AisError> {
    match c {
        0x30..=0x57 => Ok(c - 0x30),     // '0'..='W' → 0..=39
        0x60..=0x77 => Ok(c - 0x30 - 8), // '`'..='w' → 40..=63
        _ => Err(AisError::InvalidArmorChar(c)),
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

    #[test]
    fn decode_char_handles_low_range() {
        assert_eq!(decode_char(b'0').unwrap(), 0);
        assert_eq!(decode_char(b'9').unwrap(), 9);
        assert_eq!(decode_char(b':').unwrap(), 10);
        assert_eq!(decode_char(b'W').unwrap(), 39);
    }

    #[test]
    fn decode_char_handles_high_range() {
        assert_eq!(decode_char(b'`').unwrap(), 40);
        assert_eq!(decode_char(b'a').unwrap(), 41);
        assert_eq!(decode_char(b'w').unwrap(), 63);
    }

    #[test]
    fn decode_char_rejects_invalid_bytes() {
        for bad in [0x00, 0x2F, 0x58, 0x5F, 0x78, 0xFF] {
            match decode_char(bad) {
                Err(AisError::InvalidArmorChar(b)) => assert_eq!(b, bad),
                other => panic!("expected InvalidArmorChar({bad:#x}), got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_single_char_produces_six_bits() {
        // 'A' → decode_char = 17 (0x11) → 6 bits: 010001
        // Packed MSB-first into a single byte: 0100_0100 = 0x44
        //                                            ↑ 2 low bits zero
        //                                      (only 6 of 8 bits valid)
        let (bits, total) = decode(b"A", 0).unwrap();
        assert_eq!(total, 6);
        assert_eq!(bits, [0x44]);
    }

    #[test]
    fn decode_fill_bits_shortens_total() {
        // 1-char payload, fill_bits = 2 → total_bits = 4.
        let (_bits, total) = decode(b"A", 2).unwrap();
        assert_eq!(total, 4);
    }

    #[test]
    fn decode_classic_aivdm_payload_type1_position() {
        // Real example from ITU-R M.1371-5 Annex 5 (Type 1 position
        // report). Payload "13aGmP0P00PD;88MD5MTDww@2<0L" with
        // fill_bits=0 is 28*6 = 168 bits.
        let (bits, total) = decode(b"13aGmP0P00PD;88MD5MTDww@2<0L", 0).unwrap();
        assert_eq!(total, 168);
        assert_eq!(bits.len(), 21); // 168 / 8

        // First 6 bits encode the message type. '1' → decode = 1, so
        // top 6 bits of byte 0 should be 0b000001 → value in bits 7..2
        // of byte 0 is 00_0001 → byte 0 top 6 bits = 0x04.
        assert_eq!(bits[0] >> 2, 0x01);
    }

    #[test]
    fn decode_rejects_invalid_char_mid_payload() {
        // The character 'X' (0x58) is outside the armor alphabet — it
        // falls in the gap between 'W' (0x57) and '`' (0x60).
        match decode(b"1X3", 0) {
            Err(AisError::InvalidArmorChar(b'X')) => {}
            other => panic!("expected InvalidArmorChar, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_bad_fill_bits() {
        match decode(b"AAAA", 6) {
            Err(AisError::InvalidFillBits(6)) => {}
            other => panic!("expected InvalidFillBits(6), got {other:?}"),
        }
    }

    #[test]
    fn decode_empty_payload_with_zero_fill_is_empty() {
        let (bits, total) = decode(b"", 0).unwrap();
        assert_eq!(total, 0);
        assert!(bits.is_empty());
    }

    #[test]
    fn decode_empty_payload_with_nonzero_fill_errors() {
        // 0 gross bits minus 3 fill bits would underflow.
        match decode(b"", 3) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Integration: armor → BitReader
    // -----------------------------------------------------------------

    #[test]
    fn armor_feeds_bit_reader_for_type1_message() {
        // Read the first three fields of a Type 1 position report
        // (msg_type, repeat, MMSI) from the ITU-R M.1371 Annex 5
        // example payload. MMSI verified by hand-decoding the 6-bit
        // armor: chars "13aGmP..." give a 30-bit MMSI of 244_708_736
        // starting at bit 8.
        let (bits, total) = decode(b"13aGmP0P00PD;88MD5MTDww@2<0L", 0).unwrap();
        let mut r = crate::BitReader::new(&bits, total);
        assert_eq!(r.u(6), 1, "message type");
        let _repeat = r.u(2);
        assert_eq!(r.u(30), 244_708_736, "MMSI");
    }
}
