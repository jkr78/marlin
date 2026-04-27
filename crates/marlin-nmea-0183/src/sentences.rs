//! Typed decoders for individual NMEA 0183 sentence types.
//!
//! Each decoder is a plain public function taking a
//! [`RawSentence`](marlin_nmea_envelope::RawSentence) and returning a
//! typed struct or a [`DecodeError`](crate::DecodeError). The decoders
//! **do not** check `sentence_type` themselves — the caller asserts the
//! type and this module decodes the fields.

mod gga;
mod hdt;
mod prdid;
mod psxn;
mod utc_time;
mod vtg;

pub use gga::{decode_gga, GgaData, GgaFixQuality};
pub use hdt::{decode_hdt, HdtData};
pub use prdid::{
    decode_prdid, decode_prdid_pitch_roll_heading, decode_prdid_roll_pitch_heading, PrdidData,
    PrdidDialect, PrdidPitchRollHeading, PrdidRollPitchHeading,
};
pub use psxn::{decode_psxn, PsxnData, PsxnLayout, PsxnLayoutParseError, PsxnSlot};
pub use utc_time::UtcTime;
pub use vtg::{decode_vtg, VtgData, VtgMode};
