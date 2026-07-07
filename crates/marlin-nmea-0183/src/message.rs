//! The typed message enum returned by [`crate::decode`].

use marlin_nmea_envelope::RawSentence;

use crate::sentences::{
    GgaData, GllData, HdgData, HdtData, PrdidData, PsxnData, RmcData, TllData, TtmData, VtgData,
};

/// A typed NMEA 0183 message.
///
/// `#[non_exhaustive]` so future sentence-type support (GSA, GSV, ZDA,
/// DBT, MWV, ...) lands without a breaking change. Consumers must
/// include a wildcard arm in their matches.
///
/// The variant for unrecognized sentences is [`Self::Unknown`], which
/// carries the original [`RawSentence`] — callers can log, skip, or
/// decode it further with their own logic.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Nmea0183Message<'a> {
    /// GGA — Global Positioning System Fix Data.
    Gga(GgaData),
    /// GLL — Geographic Position, Latitude/Longitude.
    Gll(GllData),
    /// HDT — True Heading.
    Hdt(HdtData),
    /// RMC — Recommended Minimum Specific GNSS Data (single-sentence
    /// carrier of position + velocity + UTC date/time).
    Rmc(RmcData),
    /// VTG — Course Over Ground and Ground Speed.
    Vtg(VtgData),
    /// HDG — Heading, Deviation & Variation.
    Hdg(HdgData),
    /// TTM — Tracked Target Message (radar/ARPA).
    Ttm(TtmData),
    /// TLL — Target Latitude/Longitude (radar/ARPA).
    Tll(TllData),
    /// PSXN — Kongsberg-family proprietary motion sentence. The
    /// interpretation of the 6 data slots depends on the
    /// [`PsxnLayout`](crate::PsxnLayout) configured via
    /// [`DecodeOptions`](crate::DecodeOptions).
    Psxn(PsxnData),
    /// PRDID — proprietary attitude sentence. Field ordering varies by
    /// vendor; the variant depends on the
    /// [`PrdidDialect`](crate::PrdidDialect) configured via
    /// [`DecodeOptions`](crate::DecodeOptions). Default dialect is
    /// `Unknown`, which preserves the raw fields.
    Prdid(PrdidData),
    /// Envelope-valid sentence of a type this crate doesn't decode. The
    /// raw sentence is preserved for callers that want to decode it
    /// themselves or log the address.
    Unknown(RawSentence<'a>),
}
