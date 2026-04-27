//! AIS error type.

/// Errors that can occur while decoding AIS sentences.
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

    /// The declared fill-bits count exceeds the payload's bit count.
    #[error("fill-bits count exceeds payload size")]
    PayloadTooShort,

    /// The payload length × 6 would overflow `usize`.
    #[error("payload length overflows bit-count arithmetic")]
    PayloadTooLong,

    // -----------------------------------------------------------------
    // Reserved for upcoming milestones — variants are present so
    // callers can match against them already, but nothing emits them
    // yet.
    // -----------------------------------------------------------------
    /// Message type field did not match any decoder known to this crate.
    #[error("unknown AIS message type {0}")]
    UnknownMessageType(u8),

    /// Multi-sentence reassembly received a fragment out of the
    /// expected order (e.g. fragment 3 before fragment 2).
    #[error("multi-sentence reassembly received fragments out of order")]
    ReassemblyOutOfOrder,

    /// Multi-sentence reassembly saw fragments with mismatched channels
    /// (A vs B) for the same sequential-message-id.
    #[error("multi-sentence reassembly channel mismatch")]
    ReassemblyChannelMismatch,

    /// A partial multi-sentence reassembly exceeded the configured age
    /// limit and was dropped before completing.
    #[error("multi-sentence reassembly timed out")]
    ReassemblyTimeout,
}
