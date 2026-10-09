//! MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder.
//!
//! Sans-I/O: bytes in via [`decode`], a typed [`St0601`] out; a typed [`St0601`]
//! in via [`encode`], framed KLV bytes out. No clock, no sockets, no panics —
//! `decode` returns `Result<_, KlvDecodeError>` on malformed input and `encode`
//! returns `Result<_, KlvEncodeError>` on a set the wire cannot carry.
//!
//! Wire format: 16-byte UAS LS Universal Label key, BER lengths (short + long form),
//! big-endian values, framed with Tag 2 (precision timestamp, first) and Tag 1
//! (16-bit BCC checksum, last).
//!
//! # Design
//! - **Engineering units:** every scaled [`St0601`] tag is a [`FieldState<f64>`] in
//!   degrees, meters or m/s. Count → `f64` → count is exact for every count in each
//!   tag's width, so a decoded packet in framing order re-encodes to its source bytes.
//! - **Field state:** an omitted tag is `NotAvailable`; the ST 0601 sentinel
//!   (`i16::MIN` / `i32::MIN`) on a signed tag is `SenderError` with the raw code; a
//!   known tag with the wrong wire length is `Invalid(Unparsable)` with its bytes kept
//!   in [`St0601::unknown`]. Unknown tags round-trip verbatim via `unknown`.
//! - **Structural failures only:** `decode` fails for framing, the checksum and the
//!   mandatory Tag 2 timestamp; a field's value never fails the set. `encode` fails
//!   for a state the wire cannot carry and for a value outside the tag's range or NaN;
//!   it never clamps.
//!
//! # Example
//! ```
//! use marlin_klv::{FieldState, St0601};
//!
//! let mut set = St0601::new(1_700_000_000_000_000);
//! set.sensor_latitude_degrees = FieldState::Value(60.1768);
//! set.platform_heading_degrees = 159.97.into();
//! let mut wire = Vec::new();
//! marlin_klv::encode(&set, &mut wire)?;
//! let decoded = marlin_klv::decode(&wire)?;
//! let latitude = decoded.sensor_latitude_degrees.value().expect("sent as a value");
//! assert!((latitude - 60.1768).abs() < 1e-6);
//! assert_eq!(decoded.platform_pitch_degrees, FieldState::NotAvailable);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
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

pub use error::{KlvDecodeError, KlvEncodeError};
pub use marlin_field::{FieldState, Invalid, Kind, RawCode};
#[cfg(feature = "bytes")]
pub use st0601::encode_to_bytes;
pub use st0601::{decode, encode, precision_timestamp, St0601, UAS_LS_KEY};
pub use tags::{tag_name, tag_number, tags, TagInfo};
