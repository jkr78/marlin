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
//! - **Typed message decoders** — [`decode_message`] and [`decode`]
//!   route on the 6-bit message type to a typed [`AisMessageBody`]
//!   variant: Types 1/2/3 ([`PositionReportA`]), 5
//!   ([`StaticAndVoyageA`]), 9 ([`SarAircraftPositionReport`]), 18
//!   ([`PositionReportB`]), 19 ([`ExtendedPositionReportB`]), 21
//!   ([`AidToNavigationReport`]) and 24 Parts A and B
//!   ([`StaticDataB24A`], [`StaticDataB24B`]). Every other type
//!   surfaces as [`AisMessageBody::Other`] with the raw bits.
//! - **Multi-sentence reassembly** — [`AisReassembler`] joins
//!   fragments keyed on `(channel, sequential_id)`;
//!   [`AisFragmentParser`] runs the whole pipeline from bytes to
//!   [`AisMessage`].
//!
//! # Layered architecture
//!
//! ```text
//!  bytes ─▶ nmea_envelope ─▶ RawSentence ─▶ parse_aivdm_wrapper
//!                                             ↓
//!                                           AisReassembler (multi-sentence only)
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

#![no_std]

extern crate alloc;

mod aid_to_navigation_report;
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
mod sar_aircraft_position_report;
mod shared_types;
mod static_data_b;
mod static_voyage_a;

#[cfg(test)]
pub(crate) mod testing;

pub use aid_to_navigation_report::{
    decode_aid_to_navigation_report, AidToNavigationReport, AtonType, AID_TO_NAVIGATION_REPORT_BITS,
};
pub use aivdm::{parse_aivdm_wrapper, AivdmHeader};
pub use bit_reader::BitReader;
pub use error::AisError;
pub use extended_position_report_b::{
    decode_extended_position_report_b, ExtendedPositionReportB, EXTENDED_POSITION_REPORT_B_BITS,
};
pub use message::{decode, decode_message, AisMessage, AisMessageBody};
pub use parser::{AisFragmentParser, Parser};
pub use position_report_a::{
    decode_position_report_a, ManeuverIndicator, NavStatus, PositionReportA, RateOfTurn,
    TurnDirection, POSITION_REPORT_A_BITS,
};
pub use position_report_b::{decode_position_report_b, PositionReportB, POSITION_REPORT_B_BITS};
pub use reassembly::{AisReassembler, ReassembledPayload, DEFAULT_MAX_PARTIALS};
pub use sar_aircraft_position_report::{
    decode_sar_aircraft_position_report, AltitudeSensor, SarAircraftPositionReport,
    SAR_AIRCRAFT_POSITION_REPORT_BITS,
};
pub use shared_types::{is_auxiliary_craft_mmsi, sentinel, Dimensions, EpfdType};
pub use static_data_b::{
    decode_static_data_b, decode_static_data_b_24a, decode_static_data_b_24b, StaticDataB,
    StaticDataB24A, StaticDataB24B, Type24BExtent, Type24Part, STATIC_DATA_B_24A_BITS,
    STATIC_DATA_B_24B_BITS,
};
pub use static_voyage_a::{
    decode_static_and_voyage_a, AisVersion, Eta, StaticAndVoyageA, STATIC_VOYAGE_A_BITS,
};

// Convenience re-export for consumers that want to parse sentences
// through the envelope directly.
pub use marlin_nmea_envelope::RawSentence;
