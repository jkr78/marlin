//! Decode error type.

/// Errors that can occur while decoding a typed NMEA sentence.
///
/// A decode fails only for a structural reason: the sentence has fewer
/// fields than the decoder's floor. A field's value never fails the
/// sentence; an empty field decodes to [`FieldState::NotAvailable`](crate::FieldState::NotAvailable)
/// and a field the decoder cannot give a meaning decodes to
/// [`FieldState::Invalid`](crate::FieldState::Invalid). No 0183 field is
/// mandatory, so a sentence with every field empty is well formed, and
/// fields past the newest known version are ignored. `decode` on a
/// known sentence type with enough fields never returns `Err`.
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
}
