/// Errors returned by [`crate::encode`], [`crate::decode`], and [`crate::precision_timestamp`].
///
/// Every variant is a decode-time input problem (truncated bytes, a length claim the
/// input can't back, a wrong checksum, or a wrong local-set key) or a length that
/// overflows `usize` on this platform. The crate never panics; malformed input always
/// surfaces as one of these.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before a length claim (BER length, or a fixed-width field like
    /// the Tag 2 timestamp) could be satisfied.
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
    /// The first 16 bytes of the input are not the UAS Datalink LS universal label
    /// ([`crate::UAS_LS_KEY`]).
    #[error("local-set key is not the UAS Datalink LS UL")]
    BadKey,
}
