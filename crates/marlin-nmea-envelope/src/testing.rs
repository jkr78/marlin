//! Shared test helpers. Only compiled into test builds.
//!
//! Kept minimal on purpose — just sentence construction utilities.
//! Assertions live in each test module.

#![cfg(test)]
#![allow(dead_code)]

use alloc::vec::Vec;

/// Build `$<body>*<HH>` with no terminator. `HH` is the XOR checksum of
/// `body` as two uppercase hex digits.
pub(crate) fn build_sentence(body: &[u8]) -> Vec<u8> {
    build_with_delim_and_term(b'$', body, b"")
}

/// Build `$<body>*<HH><terminator>` with the given terminator bytes.
pub(crate) fn build_with_terminator(body: &[u8], terminator: &[u8]) -> Vec<u8> {
    build_with_delim_and_term(b'$', body, terminator)
}

/// Build `<delim><body>*<HH><terminator>`. `delim` is typically `$` or `!`.
pub(crate) fn build_with_delim_and_term(delim: u8, body: &[u8], terminator: &[u8]) -> Vec<u8> {
    let checksum = xor_checksum(body);
    let mut out = Vec::with_capacity(
        body.len()
            .saturating_add(4)
            .saturating_add(terminator.len()),
    );
    out.push(delim);
    out.extend_from_slice(body);
    out.push(b'*');
    out.extend_from_slice(&hex_ascii(checksum, HexCase::Upper));
    out.extend_from_slice(terminator);
    out
}

/// Build a sentence with the checksum rendered in lowercase hex, useful
/// for verifying case-insensitive acceptance.
pub(crate) fn build_with_lowercase_checksum(body: &[u8]) -> Vec<u8> {
    let checksum = xor_checksum(body);
    let mut out = Vec::with_capacity(body.len().saturating_add(4));
    out.push(b'$');
    out.extend_from_slice(body);
    out.push(b'*');
    out.extend_from_slice(&hex_ascii(checksum, HexCase::Lower));
    out
}

/// Build a sentence whose declared checksum deliberately differs from the
/// XOR of its body, for bad-checksum tests.
pub(crate) fn build_with_wrong_checksum(body: &[u8]) -> Vec<u8> {
    let checksum = xor_checksum(body).wrapping_add(1); // guaranteed wrong
    let mut out = Vec::with_capacity(body.len().saturating_add(4));
    out.push(b'$');
    out.extend_from_slice(body);
    out.push(b'*');
    out.extend_from_slice(&hex_ascii(checksum, HexCase::Upper));
    out
}

pub(crate) fn xor_checksum(body: &[u8]) -> u8 {
    body.iter().fold(0u8, |acc, &b| acc ^ b)
}

/// Build `\<tag>*<HH>\$<body>*<HH>` with valid TAG and sentence checksums.
pub(crate) fn build_with_tag(tag_content: &[u8], sentence_body: &[u8]) -> Vec<u8> {
    build_with_tag_inner(tag_content, sentence_body, TagChecksumMode::Valid, b"")
}

/// Build a TAG-prefixed sentence with a deliberately-wrong TAG checksum.
/// Sentence checksum remains valid — PRD decision 7 says the sentence
/// should still parse.
pub(crate) fn build_with_bad_tag_checksum(tag_content: &[u8], sentence_body: &[u8]) -> Vec<u8> {
    build_with_tag_inner(tag_content, sentence_body, TagChecksumMode::Wrong, b"")
}

/// Build a TAG-prefixed sentence with a trailing terminator.
pub(crate) fn build_with_tag_and_terminator(
    tag_content: &[u8],
    sentence_body: &[u8],
    terminator: &[u8],
) -> Vec<u8> {
    build_with_tag_inner(
        tag_content,
        sentence_body,
        TagChecksumMode::Valid,
        terminator,
    )
}

#[derive(Copy, Clone)]
enum TagChecksumMode {
    Valid,
    Wrong,
}

fn build_with_tag_inner(
    tag_content: &[u8],
    sentence_body: &[u8],
    tag_mode: TagChecksumMode,
    terminator: &[u8],
) -> Vec<u8> {
    let mut tag_cksum = xor_checksum(tag_content);
    if matches!(tag_mode, TagChecksumMode::Wrong) {
        tag_cksum = tag_cksum.wrapping_add(1);
    }
    let sentence_cksum = xor_checksum(sentence_body);

    let mut out = Vec::with_capacity(
        tag_content
            .len()
            .saturating_add(sentence_body.len())
            .saturating_add(8)
            .saturating_add(terminator.len()),
    );
    out.push(b'\\');
    out.extend_from_slice(tag_content);
    out.push(b'*');
    out.extend_from_slice(&hex_ascii(tag_cksum, HexCase::Upper));
    out.push(b'\\');
    out.push(b'$');
    out.extend_from_slice(sentence_body);
    out.push(b'*');
    out.extend_from_slice(&hex_ascii(sentence_cksum, HexCase::Upper));
    out.extend_from_slice(terminator);
    out
}

#[derive(Copy, Clone)]
enum HexCase {
    Upper,
    Lower,
}

fn hex_ascii(b: u8, case: HexCase) -> [u8; 2] {
    [nibble(b >> 4, case), nibble(b & 0x0F, case)]
}

fn nibble(n: u8, case: HexCase) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        match case {
            HexCase::Upper => b'A' + (n - 10),
            HexCase::Lower => b'a' + (n - 10),
        }
    }
}
