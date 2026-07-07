//! MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder.
//!
//! Sans-I/O: bytes in via [`decode`], a typed [`St0601`] out; a typed [`St0601`]
//! in via [`encode`], framed KLV bytes out. No clock, no sockets, no panics —
//! every public function returns `Result<_, Error>` on malformed input.
//!
//! Wire format: 16-byte UAS LS Universal Label key, BER lengths (short + long form),
//! big-endian values, framed with Tag 2 (precision timestamp, first) and Tag 1
//! (16-bit BCC checksum, last).
//!
//! # Design
//! - **Raw storage:** [`St0601`] fields hold raw wire integers — decode→encode of this
//!   crate's own output is byte-lossless. Engineering values (degrees, meters) are exposed
//!   through accessor pairs (e.g. [`St0601::sensor_latitude_degrees`] /
//!   [`St0601::set_sensor_latitude_degrees`]); setters clamp to the tag's valid range.
//! - **Sentinels:** signed tags reserve `i16::MIN`/`i32::MIN` as error indicators —
//!   accessors return `None`; raw fields still expose the wire value.
//! - **Tolerant decode:** unknown tags round-trip verbatim via [`St0601::unknown`]; a known
//!   tag with an unexpected wire length is treated as unknown, not an error. Exception:
//!   Tag 2 (mandatory timestamp) with a malformed length fails the whole decode.
//!
//! # Example
//! ```
//! let mut set = marlin_klv::St0601 { timestamp_us: 1_700_000_000_000_000, ..Default::default() };
//! set.set_sensor_latitude_degrees(60.1768);
//! set.set_platform_heading_degrees(159.97);
//! let mut wire = Vec::new();
//! marlin_klv::encode(&set, &mut wire).unwrap();
//! let decoded = marlin_klv::decode(&wire).unwrap();
//! assert!((decoded.sensor_latitude_degrees().unwrap() - 60.1768).abs() < 1e-6);
//! ```
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
pub use tags::{tag_name, tag_number, tags, TagInfo};
