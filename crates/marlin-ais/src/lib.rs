//! # marlin-ais
//!
//! Sans-I/O typed decoders for AIS (AIVDM/AIVDO) messages.
//!
//! Built on [`marlin_nmea_envelope`] for NMEA-0183 framing. This
//! crate adds:
//!
//! - **ASCII armor decode** — each `!AIVDM` payload character
//!   represents 6 bits; [`armor::decode`] unpacks them into a
//!   densely-packed bit stream with fill-bit handling.
//! - **Bit-level reader** — [`BitReader`] extracts unsigned, signed
//!   two's-complement, boolean, and 6-bit-ASCII string fields from the
//!   bit stream.
//! - **AIVDM wrapper parser** — [`parse_aivdm_wrapper`] extracts the
//!   header fields (fragment count, sequential id, channel, payload,
//!   fill bits) from a [`RawSentence`].
//!
//! Typed message decoders (Type 1/2/3, 5, 18, 19, 24A/24B) and
//! multi-sentence reassembly land in subsequent milestones.
//!
//! # Layered architecture
//!
//! ```text
//!  bytes ─▶ nmea_envelope ─▶ RawSentence ─▶ parse_aivdm_wrapper
//!                                             ↓
//!                                           armor::decode ─▶ (bits, total_bits)
//!                                                              ↓
//!                                                           BitReader ─▶ typed msg
//! ```
//!
//! Each layer is independently useful. Downstream crates that want
//! bit-level access to AIS payloads (e.g. to decode a message type
//! this crate doesn't support) use [`armor::decode`] and [`BitReader`]
//! directly.

#![doc(html_root_url = "https://docs.rs/marlin-ais/0.1.0")]
#![no_std]

extern crate alloc;

mod aivdm;
pub mod armor;
mod bit_reader;
mod error;
mod extended_position_report_b;
mod message;
mod parser;
mod position_report_a;
mod position_report_b;
mod reassembly;
mod shared_types;
mod static_data_b;
mod static_voyage_a;

#[cfg(test)]
pub(crate) mod testing;

pub use aivdm::{parse_aivdm_wrapper, AivdmHeader};
pub use bit_reader::BitReader;
pub use error::AisError;
pub use extended_position_report_b::{
    decode_extended_position_report_b, ExtendedPositionReportB, EXTENDED_POSITION_REPORT_B_BITS,
};
pub use message::{decode, decode_message, AisMessage, AisMessageBody};
pub use parser::{AisFragmentParser, Parser};
pub use position_report_a::{
    decode_position_report_a, ManeuverIndicator, NavStatus, PositionReportA, POSITION_REPORT_A_BITS,
};
pub use position_report_b::{decode_position_report_b, PositionReportB, POSITION_REPORT_B_BITS};
pub use reassembly::{AisReassembler, ReassembledPayload, DEFAULT_MAX_PARTIALS};
pub use shared_types::{Dimensions, EpfdType};
pub use static_data_b::{
    decode_static_data_b, decode_static_data_b_24a, decode_static_data_b_24b, StaticDataB,
    StaticDataB24A, StaticDataB24B, Type24Part, STATIC_DATA_B_BITS,
};
pub use static_voyage_a::{
    decode_static_and_voyage_a, AisVersion, Eta, StaticAndVoyageA, STATIC_VOYAGE_A_BITS,
};

// Convenience re-export for consumers that want to parse sentences
// through the envelope directly.
pub use marlin_nmea_envelope::RawSentence;
