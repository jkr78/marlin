//! Error type for the envelope parser.
//!
//! All parsing failures surface as [`Error`]. The enum is `#[non_exhaustive]`
//! so additional variants can be added in future versions without a breaking
//! change; downstream consumers must include a wildcard arm.

/// All errors returned by the envelope parser.
///
/// The parser is panic-free on all input; every malformed byte sequence
/// produces a variant of this enum instead.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A sentence did not begin with `$` or `!`.
    ///
    /// In `Streaming` mode this usually indicates junk bytes that have been
    /// skipped; it is only yielded when no valid start delimiter can be
    /// located in the available buffer. In `OneShot` mode it means the
    /// caller fed bytes that do not look like a sentence at all.
    #[error("sentence did not begin with '$' or '!'")]
    MissingStartDelimiter,

    /// No `*` was found before the end of the sentence body.
    ///
    /// The `*` introduces the two-digit hex checksum. Without it the
    /// sentence cannot be validated.
    #[error("sentence is missing the '*' checksum delimiter")]
    MissingChecksumDelimiter,

    /// The two characters following `*` were not valid hexadecimal digits.
    ///
    /// Hex digits are case-insensitive (both `*5A` and `*5a` are accepted);
    /// only non-hex bytes trigger this variant.
    #[error("checksum digits after '*' are not valid hexadecimal")]
    InvalidChecksumDigits,

    /// The computed XOR checksum did not match the one declared in the sentence.
    ///
    /// The sentence is otherwise structurally well-formed. `expected` is the
    /// checksum declared after `*` in the sentence; `found` is the checksum
    /// computed from the body by XOR.
    #[error("checksum mismatch: expected {expected:#04x}, computed {found:#04x}")]
    ChecksumMismatch {
        /// The checksum declared by the sentence (hex digits after `*`).
        expected: u8,
        /// The checksum computed by `XOR`-ing the body bytes.
        found: u8,
    },

    /// The sentence type (the three or more characters after the talker ID)
    /// contained bytes that are not valid UTF-8.
    ///
    /// Real NMEA sentence types are always ASCII, so this effectively means
    /// the input is corrupt.
    #[error("sentence type contains invalid UTF-8")]
    InvalidUtf8InSentenceType,

    /// The talker ID was shorter than the two bytes the NMEA 0183 standard
    /// requires.
    ///
    /// Proprietary sentences starting with `$P` are a single exception to the
    /// two-byte-talker rule and are recognized by the parser; other short
    /// prefixes produce this error.
    #[error("talker ID is shorter than 2 bytes")]
    TalkerTooShort,

    /// A TAG block prefix (`\...*hh\`) was detected but could not be parsed.
    ///
    /// Reasons include an unterminated opening backslash, a missing `*`
    /// separator inside the TAG block, or invalid hex digits in the TAG
    /// block's own checksum.
    ///
    /// Note: per the PRD, a TAG block checksum *mismatch* does **not**
    /// produce this error — the sentence checksum is authoritative, and the
    /// TAG block content is preserved even if its own checksum is wrong.
    /// Only a structurally malformed TAG block surfaces here.
    #[error("TAG block prefix is malformed")]
    MalformedTagBlock,

    /// In `OneShot` mode: the accumulated bytes do not form a complete
    /// sentence (the `*` and two checksum digits have not yet arrived).
    ///
    /// Callers may feed more bytes and retry.
    #[error("sentence is truncated (missing checksum)")]
    Truncated,

    /// In `Streaming` mode: the internal buffer reached its configured
    /// maximum size without producing a complete sentence.
    ///
    /// The parser discards buffered data up to the next candidate start
    /// delimiter and continues. Callers may treat this as a data-quality
    /// signal from the upstream feed.
    #[error("streaming buffer overflowed its configured maximum size")]
    BufferOverflow,
}
