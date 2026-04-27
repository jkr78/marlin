//! The parsed-sentence borrow type.

use alloc::vec::Vec;

/// A single NMEA 0183 sentence, borrowed from the parser's internal buffer.
///
/// `RawSentence` is zero-copy: every slice field borrows from memory owned
/// by the parser. The lifetime `'a` ties a `RawSentence` to the
/// `&mut self` borrow of the call that produced it, so exactly one
/// `RawSentence` can exist at a time per parser. To keep a sentence's
/// contents across another `feed` or `next_sentence` call, copy the needed
/// fields into owned data before proceeding.
///
/// # Invariants
///
/// The parser guarantees, before constructing a `RawSentence`:
///
/// - `start_delimiter` is either `b'$'` or `b'!'`.
/// - `raw` is the complete sentence body (from the `$`/`!` to the last hex
///   digit of the checksum, inclusive of both) with any terminator already
///   stripped.
/// - `talker` is `Some([_, _])` for standard sentences — two ASCII bytes
///   identifying the source (e.g. `GP`, `IN`, `AI`). For **proprietary**
///   sentences beginning with `$P` (e.g. `$PSXN`, `$PRDID`, `$PGRMT`),
///   `talker` is `None` and the full address including the leading `P`
///   lives in `sentence_type`. Proprietary sentences have no standardised
///   talker/type split, so `Option` makes the distinction explicit.
/// - `sentence_type` is a valid UTF-8 `&str` (in practice always ASCII).
///   For standard sentences it's the 3+ chars after the 2-byte talker
///   (e.g. `"GGA"`, `"VDM"`). For proprietary sentences it's the whole
///   address including `P` (e.g. `"PSXN"`, `"PRDID"`).
/// - `fields` contains one element per comma-separated field of the payload;
///   **empty fields are preserved as empty slices**. Many NMEA sentences
///   use empty fields to mean "no data available" and this distinction is
///   essential downstream.
/// - `checksum_ok` is `true` iff the XOR of all bytes strictly between the
///   start delimiter and `*` equals the value expressed by the two hex
///   digits after `*`. A sentence whose checksum failed is returned as an
///   [`Error::ChecksumMismatch`](crate::Error::ChecksumMismatch), not as a
///   `RawSentence` with `checksum_ok = false` — the field exists only for
///   future relaxation (e.g. a "lax" mode that accepts bad checksums with a
///   warning).
/// - `tag_block` is `Some(bytes)` if the sentence was preceded by a NMEA
///   4.10 TAG block `\<bytes>*hh\`, with `bytes` being the raw content
///   **excluding** the surrounding backslashes and checksum. The TAG block's
///   own checksum is computed by the parser and mismatches are logged via
///   the `tracing` feature; a mismatch does **not** reject the sentence
///   (see PRD decision 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSentence<'a> {
    /// Raw content of the preceding NMEA 4.10 TAG block, excluding the
    /// surrounding backslashes and the trailing `*hh`. `None` if no TAG
    /// block was present.
    pub tag_block: Option<&'a [u8]>,

    /// The sentence body from the start delimiter up to and including the
    /// final checksum hex digit, with any terminator stripped.
    pub raw: &'a [u8],

    /// `b'$'` for data sentences, `b'!'` for encapsulation sentences
    /// (AIVDM/AIVDO).
    pub start_delimiter: u8,

    /// Two-byte talker ID for standard sentences (e.g. `Some(*b"GP")`,
    /// `Some(*b"IN")`, `Some(*b"AI")`). `None` for proprietary sentences
    /// beginning with `$P`, which have no standardised talker/type split;
    /// for those, the full address including the `P` is in
    /// [`sentence_type`](Self::sentence_type).
    pub talker: Option<[u8; 2]>,

    /// The sentence type tag. For standard sentences, the 3+ chars after
    /// the talker (e.g. `"GGA"`, `"VDM"`). For proprietary `$P...`
    /// sentences, the full address including the leading `P` (e.g.
    /// `"PSXN"`, `"PRDID"`). Always ASCII in practice.
    pub sentence_type: &'a str,

    /// Payload fields, comma-separated, **empty fields preserved as empty
    /// slices**.
    pub fields: Vec<&'a [u8]>,

    /// Result of the XOR checksum verification. Always `true` for sentences
    /// returned as `Ok`; reserved for future "lax" modes.
    pub checksum_ok: bool,
}
