//! Internal test helpers. Compiled only for tests; not part of the
//! public API.

#![cfg(test)]
#![allow(dead_code, clippy::expect_used)]

use alloc::vec::Vec;

use marlin_nmea_envelope::RawSentence;

/// Parse a complete sentence byte slice into a [`RawSentence`]. Panics
/// on invalid input — test callers must only pass well-formed bytes.
pub(crate) fn parse_raw(bytes: &[u8]) -> RawSentence<'_> {
    marlin_nmea_envelope::parse(bytes).expect("test fixture should be envelope-valid")
}

/// Build an AIVDM sentence with the given wrapper fields and XOR
/// checksum computed over the body. Used for wrapper-parsing tests.
pub(crate) fn build_aivdm(
    fragment_count: u8,
    fragment_number: u8,
    sequential_id: Option<u8>,
    channel: Option<u8>,
    payload: &[u8],
    fill_bits: u8,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"AIVDM,");
    push_u8(&mut body, fragment_count);
    body.push(b',');
    push_u8(&mut body, fragment_number);
    body.push(b',');
    if let Some(id) = sequential_id {
        push_u8(&mut body, id);
    }
    body.push(b',');
    if let Some(ch) = channel {
        body.push(ch);
    }
    body.push(b',');
    body.extend_from_slice(payload);
    body.push(b',');
    push_u8(&mut body, fill_bits);
    build_from_body(b"!", &body)
}

/// Build a sentence with an arbitrary start delimiter and address.
/// Useful for negative-case tests that need non-AIS addresses.
pub(crate) fn build_with_address(start: &[u8], address: &[u8], fields: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(address.len() + 1 + fields.len());
    body.extend_from_slice(address);
    if !fields.is_empty() {
        body.push(b',');
        body.extend_from_slice(fields);
    }
    build_from_body(start, &body)
}

fn build_from_body(start: &[u8], body: &[u8]) -> Vec<u8> {
    let cksum = body.iter().fold(0u8, |acc, &b| acc ^ b);
    let mut out = Vec::with_capacity(body.len() + start.len() + 3);
    out.extend_from_slice(start);
    out.extend_from_slice(body);
    out.push(b'*');
    for nibble in [cksum >> 4, cksum & 0x0F] {
        out.push(if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + (nibble - 10)
        });
    }
    out
}

fn push_u8(buf: &mut Vec<u8>, v: u8) {
    if v >= 100 {
        buf.push(b'0' + v / 100);
    }
    if v >= 10 {
        buf.push(b'0' + (v / 10) % 10);
    }
    buf.push(b'0' + v % 10);
}

/// Bit-packing writer — the inverse of [`crate::BitReader`]. Used by
/// tests to construct synthetic AIS payloads with specific field
/// values (sentinels, boundaries, etc.).
pub(crate) struct BitWriter {
    bits: Vec<u8>,
    total_bits: usize,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        Self {
            bits: Vec::new(),
            total_bits: 0,
        }
    }

    /// Append `n` bits of `value`, MSB-first.
    #[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
    pub(crate) fn u(&mut self, n: usize, value: u64) {
        for i in 0..n {
            let bit = ((value >> (n - 1 - i)) & 1) as u8;
            let byte_idx = self.total_bits / 8;
            let bit_in_byte = 7 - (self.total_bits % 8);
            if byte_idx >= self.bits.len() {
                self.bits.push(0);
            }
            self.bits[byte_idx] |= bit << bit_in_byte;
            self.total_bits += 1;
        }
    }

    /// Append `n` bits of a signed value using two's-complement at width `n`.
    #[allow(clippy::cast_sign_loss)]
    pub(crate) fn i(&mut self, n: usize, value: i64) {
        let mask: u64 = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
        let u_val = (value as u64) & mask;
        self.u(n, u_val);
    }

    pub(crate) fn b(&mut self, v: bool) {
        self.u(1, u64::from(v));
    }

    pub(crate) fn finish(self) -> (Vec<u8>, usize) {
        (self.bits, self.total_bits)
    }
}
