//! Shared parser core used by every [`SentenceSource`](crate::SentenceSource)
//! implementation in this crate.
//!
//! Given a byte slice that already represents one complete framed NMEA 0183
//! sentence (terminator stripped by the caller), [`parse_sentence`] extracts
//! the envelope fields and verifies the XOR checksum. It performs no buffer
//! management and no boundary detection — those are the responsibility of
//! `OneShot` and `Streaming`.
//!
//! Uses `nom` complete combinators for the structural pieces and direct
//! byte operations for the one-pass XOR and field split. Per PRD §E7 and
//! the architectural rationale in §9 decision 3, `nom::*::streaming`
//! variants are intentionally avoided.

use alloc::vec::Vec;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while_m_n},
    IResult, Parser,
};

use crate::{Error, RawSentence};

/// Parse one complete framed sentence.
///
/// The input must:
///
/// - begin with `$` or `!`;
/// - contain `*` followed by two ASCII hex digits;
/// - have any terminator (`\r\n`, `\n`, `\r`) already stripped by the
///   caller.
///
/// Bytes beyond the second checksum digit are tolerated but ignored; the
/// returned [`RawSentence::raw`] is the exact byte range that was consumed.
pub(crate) fn parse_sentence(input: &[u8]) -> Result<RawSentence<'_>, Error> {
    let (tag_block, remainder) = extract_tag_block(input)?;

    let (after_start, start_delimiter) =
        start_delim(remainder).map_err(|_| Error::MissingStartDelimiter)?;

    let (after_body, body) =
        body_up_to_star(after_start).map_err(|_| Error::MissingChecksumDelimiter)?;

    // `body_up_to_star` already confirmed the next byte is `*`, so any
    // failure here is a hex-digit problem (wrong characters or too few).
    let (_rest, hex) = star_then_hex(after_body).map_err(|_| Error::InvalidChecksumDigits)?;

    let expected = decode_hex_byte(hex);
    let found = body.iter().fold(0u8, |acc, &b| acc ^ b);
    if expected != found {
        return Err(Error::ChecksumMismatch { expected, found });
    }

    let BodyParts {
        talker,
        sentence_type,
        fields,
    } = split_body(body, start_delimiter)?;

    // raw = from the start delimiter through the last hex digit, within
    // `remainder` (i.e. excluding any TAG block prefix).
    let raw_len = body.len().saturating_add(4);
    let raw = remainder.get(..raw_len).unwrap_or(remainder);

    Ok(RawSentence {
        tag_block,
        raw,
        start_delimiter,
        talker,
        sentence_type,
        fields,
        checksum_ok: true,
    })
}

/// Peel a NMEA 4.10 TAG block (`\content*hh\`) off the front of `input`,
/// if present. Returns the captured content slice (bytes between the
/// opening `\` and the `*`, exclusive of both) and the remaining input
/// starting at the sentence's `$` or `!`.
///
/// Per PRD decision 7, a TAG block with a mismatched checksum is **not**
/// rejected — the content is preserved and the mismatch is surfaced only
/// as a `tracing::debug!` event when the `tracing` feature is enabled.
/// Only structurally malformed TAG blocks (unterminated, missing `*`, or
/// non-hex checksum digits) produce [`Error::MalformedTagBlock`].
fn extract_tag_block(input: &[u8]) -> Result<(Option<&[u8]>, &[u8]), Error> {
    if input.first() != Some(&b'\\') {
        return Ok((None, input));
    }

    let after_open = input.get(1..).unwrap_or(&[]);
    let close_off = after_open
        .iter()
        .position(|&b| b == b'\\')
        .ok_or(Error::MalformedTagBlock)?;
    let tag_content = after_open.get(..close_off).unwrap_or(&[]);

    let star_off = tag_content
        .iter()
        .position(|&b| b == b'*')
        .ok_or(Error::MalformedTagBlock)?;
    let tag_body = tag_content.get(..star_off).unwrap_or(&[]);
    let tag_hex = tag_content.get(star_off.saturating_add(1)..).unwrap_or(&[]);

    if tag_hex.len() != 2 || !tag_hex.iter().all(u8::is_ascii_hexdigit) {
        return Err(Error::MalformedTagBlock);
    }

    // Verify the TAG block's own checksum. Per PRD decision 7 a mismatch
    // is advisory, not fatal: we emit a tracing event (if the feature is
    // enabled) but still accept the content. When the `tracing` feature
    // is off the whole block compiles to nothing.
    #[cfg(feature = "tracing")]
    {
        let expected = decode_hex_byte(tag_hex);
        let computed = tag_body.iter().fold(0u8, |acc, &b| acc ^ b);
        if expected != computed {
            tracing::debug!(
                expected,
                computed,
                "TAG block checksum mismatch; content preserved (PRD decision 7)"
            );
        }
    }

    // Absolute position of the closing `\` in `input` is 1 + close_off;
    // the remainder (sentence body) begins one byte past that.
    let remainder = input.get(close_off.saturating_add(2)..).unwrap_or(&[]);
    Ok((Some(tag_body), remainder))
}

fn start_delim(i: &[u8]) -> IResult<&[u8], u8> {
    let (rest, matched) = alt((tag("$".as_bytes()), tag("!".as_bytes()))).parse(i)?;
    Ok((rest, matched.first().copied().unwrap_or(0)))
}

fn body_up_to_star(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, body) = take_till(|b: u8| b == b'*').parse(i)?;
    // take_till matches even when the terminator byte never appears; in
    // that case the parser "succeeded" but no '*' follows. Reject that
    // case by requiring '*' as the next byte.
    if rest.first() != Some(&b'*') {
        return Err(nom::Err::Error(nom::error::Error::new(
            rest,
            nom::error::ErrorKind::Char,
        )));
    }
    Ok((rest, body))
}

fn star_then_hex(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let (rest, _) = tag("*".as_bytes()).parse(i)?;
    take_while_m_n(2usize, 2usize, |b: u8| b.is_ascii_hexdigit()).parse(rest)
}

/// Decode two ASCII hex digits to a byte. Input is guaranteed length 2 and
/// pre-validated as hex by the nom parser, so this function is infallible.
fn decode_hex_byte(hex: &[u8]) -> u8 {
    let hi = decode_nibble(hex.first().copied().unwrap_or(0));
    let lo = decode_nibble(hex.get(1).copied().unwrap_or(0));
    (hi << 4) | lo
}

fn decode_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => 10 + (b - b'a'),
        b'A'..=b'F' => 10 + (b - b'A'),
        // Unreachable: caller pre-validates via `is_ascii_hexdigit`. A wrong
        // byte here would silently decode as 0 rather than panic, preserving
        // the panic-free contract (PRD §C3).
        _ => 0,
    }
}

struct BodyParts<'a> {
    talker: Option<[u8; 2]>,
    sentence_type: &'a str,
    fields: Vec<&'a [u8]>,
}

/// Split a validated body into talker, `sentence_type`, and fields.
///
/// For a standard sentence body like `GPGGA,a,b,c` (start delimiter `$`):
/// - `talker` = `Some(*b"GP")`
/// - `sentence_type` = `"GGA"`
/// - `fields` = `["a", "b", "c"]`
///
/// For a proprietary sentence body like `PSXN,23,...` (start delimiter `$`),
/// where NMEA reserves `$P…` for manufacturer-specific formats with no
/// standardised talker/type split:
/// - `talker` = `None`
/// - `sentence_type` = `"PSXN"` (the full address including `P`)
/// - `fields` = `["23", ...]`
///
/// Encapsulation sentences (`!…`) are always standard, never proprietary.
///
/// Empty fields between commas are preserved as empty slices.
fn split_body(body: &[u8], start_delimiter: u8) -> Result<BodyParts<'_>, Error> {
    // Locate the end of the address (first comma, or end of body).
    let comma_pos = body.iter().position(|&b| b == b',');
    let address_end = comma_pos.unwrap_or(body.len());
    let address = body.get(..address_end).unwrap_or(&[]);

    // Proprietary marker: `$P…` with at least one more byte after the `P`.
    // `!P…` is not a thing in NMEA — encapsulation sentences never use the
    // proprietary namespace.
    let is_proprietary =
        start_delimiter == b'$' && address.first() == Some(&b'P') && address.len() >= 2;

    let (talker, type_bytes) = if is_proprietary {
        (None, address)
    } else {
        let slice = address.get(..2).ok_or(Error::TalkerTooShort)?;
        let mut arr = [0u8; 2];
        arr.copy_from_slice(slice);
        let type_bytes = address.get(2..).unwrap_or(&[]);
        (Some(arr), type_bytes)
    };

    let sentence_type =
        core::str::from_utf8(type_bytes).map_err(|_| Error::InvalidUtf8InSentenceType)?;

    let fields: Vec<&[u8]> = if comma_pos.is_some() {
        let fields_bytes = body.get(address_end.saturating_add(1)..).unwrap_or(&[]);
        fields_bytes.split(|&b| b == b',').collect()
    } else {
        Vec::new()
    };

    Ok(BodyParts {
        talker,
        sentence_type,
        fields,
    })
}

/// Strip a single trailing terminator sequence, if any.
///
/// Recognizes `\r\n`, `\n`, and `\r`. Returns the input unchanged if no
/// terminator is present. This is exposed at crate-internal scope so both
/// modes can call it before handing a slice to [`parse_sentence`].
pub(crate) fn strip_terminator(s: &[u8]) -> &[u8] {
    if s.ends_with(b"\r\n") {
        s.get(..s.len().saturating_sub(2)).unwrap_or(s)
    } else if s.ends_with(b"\n") || s.ends_with(b"\r") {
        s.get(..s.len().saturating_sub(1)).unwrap_or(s)
    } else {
        s
    }
}
