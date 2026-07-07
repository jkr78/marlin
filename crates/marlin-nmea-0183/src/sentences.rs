//! Typed decoders for individual NMEA 0183 sentence types.
//!
//! Each decoder is a plain public function taking a
//! [`RawSentence`](marlin_nmea_envelope::RawSentence) and returning a
//! typed struct or a [`DecodeError`](crate::DecodeError). The decoders
//! **do not** check `sentence_type` themselves — the caller asserts the
//! type and this module decodes the fields.

mod gga;
mod gll;
mod hdg;
mod hdt;
mod prdid;
mod psxn;
mod rmc;
mod status;
mod tll;
mod ttm;
mod utc_time;
mod vtg;

pub use gga::{decode_gga, GgaData, GgaFixQuality};
pub use gll::{decode_gll, GllData};
pub use hdg::{decode_hdg, HdgData};
pub use hdt::{decode_hdt, HdtData};
pub use prdid::{
    decode_prdid, decode_prdid_pitch_roll_heading, decode_prdid_roll_pitch_heading, PrdidData,
    PrdidDialect, PrdidPitchRollHeading, PrdidRollPitchHeading,
};
pub use psxn::{decode_psxn, PsxnData, PsxnLayout, PsxnLayoutParseError, PsxnSlot};
pub use rmc::{decode_rmc, RmcData, RmcNavStatus, UtcDate};
pub use status::{DataStatus, TargetStatus};
pub use tll::{decode_tll, TllData};
pub use ttm::{decode_ttm, AcquisitionType, AngleReference, DistanceUnits, TtmData};
pub use utc_time::UtcTime;
pub use vtg::{decode_vtg, VtgData, VtgMode};
