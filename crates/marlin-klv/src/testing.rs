//! Internal test helpers. Compiled only for tests; not part of the public API.
#![cfg(test)]
#![allow(dead_code, clippy::expect_used)]

use alloc::vec::Vec;

use crate::ber::ber_encode_len;
use crate::checksum::bcc;
use crate::st0601::UAS_LS_KEY;

/// Assemble a KLV datagram from raw `(tag, value)` items, framing with the UAS LS key,
/// a correct outer BER length, and a correct Tag 1 checksum. Lets negative-case tests
/// place arbitrary tags / wrong lengths without hand-computing the checksum.
pub(crate) struct KlvBuilder {
    items: Vec<u8>,
}

impl KlvBuilder {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Append Tag 2 (precision timestamp, 8 BE bytes). First item by convention.
    pub(crate) fn timestamp(self, us: u64) -> Self {
        self.tag(2, &us.to_be_bytes())
    }

    /// Append Tag 65 (UAS LS version, 1 byte).
    pub(crate) fn version(self, v: u8) -> Self {
        self.tag(65, &[v])
    }

    /// Append an arbitrary `(tag, value)` item verbatim.
    pub(crate) fn tag(mut self, tag: u8, value: &[u8]) -> Self {
        self.items.push(tag);
        ber_encode_len(value.len(), &mut self.items);
        self.items.extend_from_slice(value);
        self
    }

    /// Frame into a complete datagram with a correct Tag 1 checksum.
    pub(crate) fn build(self) -> Vec<u8> {
        let value_len = self.items.len() + 4;
        let mut out = Vec::new();
        out.extend_from_slice(&UAS_LS_KEY);
        ber_encode_len(value_len, &mut out);
        out.extend_from_slice(&self.items);
        out.push(1);
        out.push(2);
        let checksum = bcc(&out);
        out.extend_from_slice(&checksum.to_be_bytes());
        out
    }

    /// Frame like [`build`] but corrupt the low checksum byte (for `BadChecksum` tests).
    pub(crate) fn build_bad_checksum(self) -> Vec<u8> {
        let mut out = self.build();
        if let Some(last) = out.last_mut() {
            *last ^= 0xFF;
        }
        out
    }
}
