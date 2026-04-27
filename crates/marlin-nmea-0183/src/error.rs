//! Decode error type.

/// Errors that can occur while decoding a typed NMEA sentence.
///
/// `#[non_exhaustive]` so new variants can be added in minor versions
/// without a breaking change. Consumers must include a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DecodeError {
    /// The sentence had fewer fields than the decoder requires.
    #[error("expected at least {expected} fields, got {got}")]
    NotEnoughFields {
        /// Minimum field count the decoder needed.
        expected: usize,
        /// Number of fields actually present.
        got: usize,
    },

    /// A field that should have been a number was not a valid number.
    #[error("field {field_index} is not a valid number")]
    InvalidNumber {
        /// 0-based position of the bad field within the sentence payload.
        field_index: usize,
    },

    /// A coordinate field was syntactically valid but out of range
    /// (latitude outside ±90°, longitude outside ±180°).
    #[error("field {field_index} is out of range for its type")]
    OutOfRange {
        /// 0-based position of the bad field within the sentence payload.
        field_index: usize,
    },

    /// A coordinate field's hemisphere byte was not one of `N`/`S` (for
    /// latitude) or `E`/`W` (for longitude).
    #[error("field {field_index} has an invalid hemisphere indicator")]
    InvalidHemisphere {
        /// 0-based position of the bad field within the sentence payload.
        field_index: usize,
    },

    /// A field contained bytes that are not valid UTF-8. Real NMEA
    /// content is always ASCII; this usually indicates corruption.
    #[error("field {field_index} contains invalid UTF-8")]
    InvalidUtf8 {
        /// 0-based position of the bad field within the sentence payload.
        field_index: usize,
    },

    /// A UTC time field had an invalid structure (wrong length, bad
    /// separator, out-of-range hour/minute/second).
    #[error("field {field_index} is not a valid UTC time (hhmmss[.ss])")]
    InvalidUtcTime {
        /// 0-based position of the bad field within the sentence payload.
        field_index: usize,
    },
}
