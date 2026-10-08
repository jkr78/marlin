//! AIS error type.

/// Errors that can occur while decoding AIS sentences.
///
/// Every variant is a message failure for a structural reason: the
/// envelope, the AIVDM wrapper, the armor, a payload below the message
/// type's floor, or multi-sentence reassembly. A field's value never
/// fails the message: a sentinel, reserved or out-of-range code decodes
/// to the field's [`FieldState`](crate::FieldState), and a field whose
/// bits lie past the payload end is not available. Trailing bits past
/// the type's length are ignored, and an unknown message type is a raw
/// payload, not an error.
///
/// `#[non_exhaustive]` so new variants can be added in minor versions
/// without a breaking change. Consumers must include a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AisError {
    /// Envelope-level failure (framing, checksum, TAG block, buffer
    /// overflow) forwarded from [`marlin_nmea_envelope::Error`].
    #[error("envelope error: {0}")]
    Envelope(#[from] marlin_nmea_envelope::Error),

    /// The sentence is not an AIS encapsulation sentence — its start
    /// delimiter is not `!` or its sentence type is not `VDM`/`VDO`.
    #[error("not an AIS encapsulation sentence")]
    NotAnAisSentence,

    /// The AIVDM/AIVDO wrapper fields are missing or structurally
    /// malformed (wrong field count, non-numeric fragment-count, etc.).
    #[error("malformed AIVDM/AIVDO wrapper")]
    MalformedWrapper,

    /// A byte in the armored payload is not in the AIS 6-bit armor
    /// alphabet (valid range: `0`–`W` or `` ` ``–`w` in ASCII).
    #[error("invalid armor character {0:#04x}")]
    InvalidArmorChar(u8),

    /// The declared fill-bits count is out of range (must be 0..=5).
    #[error("invalid fill-bits count {0} (must be 0..=5)")]
    InvalidFillBits(u8),

    /// The payload has fewer bits than the operation needs. Surfaces
    /// from `armor::decode` when the declared fill-bits count exceeds
    /// the payload's gross bit count, and from per-type decoders when
    /// `total_bits` is below the chosen message type's floor (e.g. 160
    /// bits for Type 24 Part A, 168 for Part B, 420 for Type 5).
    #[error("payload too short for the chosen decoder")]
    PayloadTooShort,

    /// The payload length × 6 would overflow `usize`.
    #[error("payload length overflows bit-count arithmetic")]
    PayloadTooLong,

    // -----------------------------------------------------------------
    // Multi-sentence reassembly — emitted by `AisReassembler` and
    // surfaced through `AisFragmentParser::next_message`.
    // -----------------------------------------------------------------
    /// Multi-sentence reassembly received a fragment out of the
    /// expected order (e.g. fragment 3 before fragment 2), or a
    /// continuation fragment with no partial open on its
    /// `(channel, sequential_id)` key.
    #[error("multi-sentence reassembly received fragments out of order")]
    ReassemblyOutOfOrder,

    /// A partial multi-sentence reassembly exceeded the configured age
    /// limit and was dropped before completing.
    #[error("multi-sentence reassembly timed out")]
    ReassemblyTimeout,
}
