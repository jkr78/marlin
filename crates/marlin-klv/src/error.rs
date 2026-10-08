use marlin_field::Kind;

/// Errors returned by [`crate::decode`] and [`crate::precision_timestamp`].
///
/// A decode fails only for a structural reason: the input ends early, a
/// length claim the input cannot back, a wrong local-set key, a checksum
/// that is missing or mismatches, or the one mandatory field, the Tag 2
/// precision timestamp, absent or malformed. A field's value never fails
/// the set: a scaled tag carrying the ST 0601 sentinel, or a known tag
/// with the wrong wire length, decodes to a field state on the
/// [`St0601`](crate::St0601) field. The crate never panics; malformed input
/// always surfaces as one of these.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before a length claim (the outer BER length, an
    /// item's BER length, or the 16-byte key) could be satisfied.
    #[error("input truncated: needed {needed} bytes at offset {offset}, had {available}")]
    Truncated {
        /// Byte offset into the input where the missing data was expected.
        offset: usize,
        /// Number of bytes required to satisfy the read.
        needed: usize,
        /// Number of bytes actually available from `offset` to the end of input.
        available: usize,
    },
    /// A BER long-form length claims more length-octets than fit in a `usize` on this
    /// platform, or uses the illegal indefinite-length form (`0x80`).
    #[error("BER length too large for this platform")]
    LengthOverflow,
    /// The embedded Tag 1 checksum does not match the BCC computed over the decoded bytes.
    #[error("checksum mismatch: computed {computed:#06x}, embedded {embedded:#06x}")]
    BadChecksum {
        /// Checksum computed over the input bytes.
        computed: u16,
        /// Checksum embedded in the input's Tag 1 item.
        embedded: u16,
    },
    /// The set carries no readable Tag 1 checksum: the item is absent, or its
    /// value is not 2 bytes.
    #[error("Tag 1 checksum absent or not 2 bytes")]
    MissingChecksum,
    /// The set carries no readable Tag 2 precision timestamp, the one mandatory
    /// field: the item is absent, or its value is not 8 bytes.
    #[error("Tag 2 precision timestamp absent or not 8 bytes")]
    BadTimestamp,
    /// The first 16 bytes of the input are not the UAS Datalink LS universal label
    /// ([`crate::UAS_LS_KEY`]).
    #[error("local-set key is not the UAS Datalink LS UL")]
    BadKey,
}

/// Errors returned by [`crate::encode`] and by `encode_to_bytes` (feature `bytes`).
///
/// Every field state [`crate::decode`] can produce re-encodes; a state it
/// cannot produce, or a value the tag's wire range cannot hold, is one of
/// these. On `Err` the output buffer is untouched.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// The field is in a state the tag cannot carry on the wire: over-range
    /// (`AtLeast`), an undefined raw code (`Invalid::Undefined`), or a sender
    /// error whose raw code is not the tag's own sentinel, including
    /// any sender error on an unsigned tag or on Tag 65.
    #[error("tag {tag} cannot carry the {} state {kind:?}", kind.name())]
    Unencodable {
        /// ST 0601 tag number of the field.
        tag: u8,
        /// The state the tag cannot carry.
        kind: Kind,
    },
    /// The field's value lies outside the tag's engineering range (inclusive
    /// at both ends), or is NaN.
    #[error("tag {tag} value is outside the tag's range or NaN")]
    OutOfRange {
        /// ST 0601 tag number of the field.
        tag: u8,
    },
}
