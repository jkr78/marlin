//! MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder.
#![doc(html_root_url = "https://docs.rs/marlin-klv/0.1.1")]
#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod ber;
mod checksum;
mod error;
mod scale;
mod st0601;
mod tags;
#[cfg(test)]
pub(crate) mod testing;

pub use error::Error;
#[cfg(feature = "bytes")]
pub use st0601::encode_to_bytes;
pub use st0601::{decode, encode, precision_timestamp, St0601, UAS_LS_KEY};
