use core::mem::size_of;

use alloc::vec::Vec;

use crate::error::Error;

/// Encode a BER length: short form (`len < 128`) is one byte; long form is `0x80 | n`
/// followed by `n` big-endian length bytes (minimal, no leading zeros).
// `len < 0x80` in the short-form branch; `significant.len() <= size_of::<usize>()`
// in the long-form branch — both casts to u8 are provably in range.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn ber_encode_len(len: usize, out: &mut Vec<u8>) {
    if len < 0x80 {
        out.push(len as u8);
        return;
    }
    let bytes = len.to_be_bytes();
    let first = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    let significant = bytes.get(first..).unwrap_or(&[]);
    out.push(0x80 | significant.len() as u8);
    out.extend_from_slice(significant);
}

/// Decode a BER length at `offset`. Returns `(length, next_offset)`.
pub(crate) fn ber_decode_len(input: &[u8], offset: usize) -> Result<(usize, usize), Error> {
    let first = *input.get(offset).ok_or(Error::Truncated {
        offset,
        needed: 1,
        available: input.len().saturating_sub(offset),
    })?;
    if first < 0x80 {
        return Ok((first as usize, offset + 1));
    }
    let n = (first & 0x7F) as usize;
    if n == 0 || n > size_of::<usize>() {
        return Err(Error::LengthOverflow);
    }
    let bytes = read_bytes(input, offset + 1, n)?;
    let mut value: usize = 0;
    for b in bytes {
        value = (value << 8) | (*b as usize);
    }
    Ok((value, offset + 1 + n))
}

/// Borrow `input[start..start + len]` or report exactly how many bytes were missing.
pub(crate) fn read_bytes(input: &[u8], start: usize, len: usize) -> Result<&[u8], Error> {
    let end = start.checked_add(len).ok_or(Error::LengthOverflow)?;
    input.get(start..end).ok_or(Error::Truncated {
        offset: start,
        needed: len,
        available: input.len().saturating_sub(start),
    })
}

pub(crate) fn be_u16(bytes: &[u8]) -> Result<u16, Error> {
    match bytes {
        [hi, lo] => Ok(u16::from_be_bytes([*hi, *lo])),
        _ => Err(Error::Truncated {
            offset: 0,
            needed: 2,
            available: bytes.len(),
        }),
    }
}

pub(crate) fn be_u64(bytes: &[u8]) -> Result<u64, Error> {
    let array: [u8; 8] = bytes.try_into().map_err(|_| Error::Truncated {
        offset: 0,
        needed: 8,
        available: bytes.len(),
    })?;
    Ok(u64::from_be_bytes(array))
}

/// Fixed-width big-endian readers for typed tag values. `None` on length mismatch
/// (callers fall back to the unknown-tag list — tolerant decode).
pub(crate) fn read_u8(v: &[u8]) -> Option<u8> {
    match v {
        [b] => Some(*b),
        _ => None,
    }
}

pub(crate) fn read_u16(v: &[u8]) -> Option<u16> {
    v.try_into().ok().map(u16::from_be_bytes)
}

pub(crate) fn read_i16(v: &[u8]) -> Option<i16> {
    v.try_into().ok().map(i16::from_be_bytes)
}

pub(crate) fn read_u32(v: &[u8]) -> Option<u32> {
    v.try_into().ok().map(u32::from_be_bytes)
}

pub(crate) fn read_i32(v: &[u8]) -> Option<i32> {
    v.try_into().ok().map(i32::from_be_bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod ber_tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn short_form_encodes_single_byte() {
        let mut out = Vec::new();
        ber_encode_len(5, &mut out);
        assert_eq!(out, [0x05]);
    }

    #[test]
    fn short_form_boundary_127_is_single_byte() {
        let mut out = Vec::new();
        ber_encode_len(127, &mut out);
        assert_eq!(out, [0x7F]);
    }

    #[test]
    fn long_form_128_is_two_bytes() {
        let mut out = Vec::new();
        ber_encode_len(128, &mut out);
        assert_eq!(out, [0x81, 0x80]);
    }

    #[test]
    fn long_form_200_is_81_c8() {
        let mut out = Vec::new();
        ber_encode_len(200, &mut out);
        assert_eq!(out, [0x81, 0xC8]);
    }

    #[test]
    fn long_form_300_is_82_01_2c() {
        let mut out = Vec::new();
        ber_encode_len(300, &mut out);
        assert_eq!(out, [0x82, 0x01, 0x2C]);
    }

    #[test]
    fn decode_short_form_returns_value_and_next_offset() {
        let (len, next) = ber_decode_len(&[0x05], 0).expect("short-form length");
        assert_eq!(len, 5);
        assert_eq!(next, 1);
    }

    #[test]
    fn decode_long_form_300_round_trips() {
        let mut buf = Vec::new();
        ber_encode_len(300, &mut buf);
        let (len, next) = ber_decode_len(&buf, 0).expect("long-form length");
        assert_eq!(len, 300);
        assert_eq!(next, 3);
    }

    #[test]
    fn decode_empty_input_is_truncated() {
        let err = ber_decode_len(&[], 0).expect_err("no length byte present");
        assert_eq!(
            err,
            Error::Truncated {
                offset: 0,
                needed: 1,
                available: 0
            }
        );
    }

    #[test]
    fn decode_long_form_missing_length_bytes_is_truncated() {
        // 0x82 promises two length bytes but only one follows.
        let err = ber_decode_len(&[0x82, 0x01], 0).expect_err("missing one length byte");
        assert!(matches!(err, Error::Truncated { .. }), "got {err:?}");
    }

    #[test]
    fn decode_oversized_length_count_is_overflow() {
        // 0x89 => 9 length bytes, more than a usize can hold.
        let err = ber_decode_len(&[0x89, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0)
            .expect_err("9 length bytes overflow usize");
        assert_eq!(err, Error::LengthOverflow);
    }

    #[test]
    fn decode_indefinite_length_is_rejected() {
        // 0x80 = indefinite form, illegal in KLV.
        let err = ber_decode_len(&[0x80], 0).expect_err("indefinite length illegal");
        assert_eq!(err, Error::LengthOverflow);
    }
}
