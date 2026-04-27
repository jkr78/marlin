//! Types shared across multiple AIS message variants.

/// Vessel dimensions as emitted in Type 5 (Class A static) and Type
/// 24B (Class B static part B).
///
/// The four fields measure the distances from the position-reference
/// point (typically the antenna) to the bow, stern, port, and
/// starboard edges of the vessel, in metres. `0` is the "not
/// available" sentinel and maps to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    /// Distance from position-reference to bow, in metres.
    /// Wire width: 9 bits, max 511 m. `None` on sentinel `0`.
    pub to_bow_m: Option<u16>,
    /// Distance from position-reference to stern, in metres.
    /// Wire width: 9 bits, max 511 m. `None` on sentinel `0`.
    pub to_stern_m: Option<u16>,
    /// Distance from position-reference to port side, in metres.
    /// Wire width: 6 bits, max 63 m. `None` on sentinel `0`.
    pub to_port_m: Option<u8>,
    /// Distance from position-reference to starboard side, in metres.
    /// Wire width: 6 bits, max 63 m. `None` on sentinel `0`.
    pub to_starboard_m: Option<u8>,
}

/// Electronic Position-Fixing Device type — 4-bit field in Type 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpfdType {
    /// 0 — undefined (default).
    Undefined,
    /// 1 — GPS.
    Gps,
    /// 2 — GLONASS.
    Glonass,
    /// 3 — Combined GPS/GLONASS.
    CombinedGpsGlonass,
    /// 4 — Loran-C.
    LoranC,
    /// 5 — Chayka.
    Chayka,
    /// 6 — Integrated navigation system.
    IntegratedNavigation,
    /// 7 — Surveyed position.
    Surveyed,
    /// 8 — Galileo.
    Galileo,
    /// 15 — Internal GNSS.
    InternalGnss,
    /// 9..=14 reserved; carried as raw byte.
    Reserved(u8),
}

impl EpfdType {
    pub(crate) fn from_u4(v: u8) -> Self {
        match v {
            0 => Self::Undefined,
            1 => Self::Gps,
            2 => Self::Glonass,
            3 => Self::CombinedGpsGlonass,
            4 => Self::LoranC,
            5 => Self::Chayka,
            6 => Self::IntegratedNavigation,
            7 => Self::Surveyed,
            8 => Self::Galileo,
            15 => Self::InternalGnss,
            other => Self::Reserved(other),
        }
    }
}

/// Trim an AIS 6-bit string (decoded from [`crate::BitReader::string`]),
/// returning `None` when the content is entirely padding.
///
/// AIS pads short strings with `@` (value 0) at the end and sometimes
/// with trailing spaces. A field consisting solely of these characters
/// means "not available".
pub(crate) fn trim_ais_string(s: alloc::string::String) -> Option<alloc::string::String> {
    let trimmed_len = s.trim_end_matches(['@', ' ']).len();
    if trimmed_len == 0 {
        None
    } else {
        let mut out = s;
        out.truncate(trimmed_len);
        Some(out)
    }
}

/// Decode a 9-bit dimension field to `Option<u16>` (sentinel `0` → `None`).
pub(crate) fn dim_u9(raw: u64) -> Option<u16> {
    if raw == 0 {
        None
    } else {
        // 9-bit value fits easily in u16.
        #[allow(clippy::cast_possible_truncation)]
        Some((raw & 0x1FF) as u16)
    }
}

/// Decode a 6-bit dimension field to `Option<u8>` (sentinel `0` → `None`).
pub(crate) fn dim_u6(raw: u64) -> Option<u8> {
    if raw == 0 {
        None
    } else {
        #[allow(clippy::cast_possible_truncation)]
        Some((raw & 0x3F) as u8)
    }
}
