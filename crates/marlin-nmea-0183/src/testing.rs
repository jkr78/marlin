//! Internal test helpers for `marlin-nmea-0183`. Only compiled for
//! tests; not part of the published API.

#![cfg(test)]
#![allow(dead_code, clippy::expect_used)]

use alloc::vec::Vec;

use marlin_nmea_envelope::RawSentence;

/// Parse a byte slice containing one complete sentence into a typed
/// `RawSentence`. Panics if the envelope rejects the bytes — test
/// callers should only pass well-formed input.
pub(crate) fn parse_raw(bytes: &[u8]) -> RawSentence<'_> {
    marlin_nmea_envelope::parse(bytes).expect("test fixture should be envelope-valid")
}

/// Build a minimal sentence around `body` (add `$`, compute checksum,
/// append `*HH`) and return the owned bytes. Use with [`parse_raw`]:
///
/// ```ignore
/// let bytes = build(b"INHDT,123.4,T");
/// let raw = parse_raw(&bytes);
/// let hdt = decode_hdt(&raw).unwrap();
/// ```
pub(crate) fn build(body: &[u8]) -> Vec<u8> {
    let cksum = body.iter().fold(0u8, |acc, &b| acc ^ b);
    let mut out = Vec::with_capacity(body.len().saturating_add(4));
    out.push(b'$');
    out.extend_from_slice(body);
    out.push(b'*');
    for nib in [cksum >> 4, cksum & 0x0F] {
        out.push(if nib < 10 {
            b'0' + nib
        } else {
            b'A' + (nib - 10)
        });
    }
    out
}
