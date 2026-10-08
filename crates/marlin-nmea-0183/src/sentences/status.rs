//! Data validity status — A/V indicator field shared by RMC and GLL.

/// Two-state validity flag from the RMC and GLL `Status` field.
///
/// Per NMEA 0183, position-bearing sentences include a single-byte
/// status indicator:
/// - `A` — Active / valid / data reliable
/// - `V` — Void / invalid / data not reliable
///
/// A status field: it qualifies the other fields of its sentence and
/// does not change their field state. Safety-critical consumers reject
/// [`Self::Void`] before acting on the position or velocity values in
/// the same sentence. An empty field is not available; an unnamed byte
/// is invalid with the byte as its raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataStatus {
    /// `A` — data is valid and active.
    Active,
    /// `V` — data is void; receiver flagged it as unreliable.
    Void,
}

impl DataStatus {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'A' | b'a' => Some(Self::Active),
            b'V' | b'v' => Some(Self::Void),
            _ => None,
        }
    }
}

/// Radar/ARPA target tracking state, shared by the TTM and TLL target
/// status field.
///
/// - `L` — Lost (target no longer tracked)
/// - `Q` — Query (target being acquired)
/// - `T` — Tracking (target under track)
///
/// An unnamed byte decodes to the invalid field state with the byte as
/// its raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetStatus {
    /// `L` — target lost.
    Lost,
    /// `Q` — target being acquired / queried.
    Query,
    /// `T` — target under track.
    Tracking,
}

impl TargetStatus {
    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'L' | b'l' => Some(Self::Lost),
            b'Q' | b'q' => Some(Self::Query),
            b'T' | b't' => Some(Self::Tracking),
            _ => None,
        }
    }
}
