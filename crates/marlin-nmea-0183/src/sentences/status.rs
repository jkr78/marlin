//! Data validity status — A/V indicator field shared by RMC and GLL.

/// Two-state validity flag from the RMC and GLL `Status` field.
///
/// Per NMEA 0183, position-bearing sentences include a single-byte
/// status indicator:
/// - `A` — Active / valid / data reliable
/// - `V` — Void / invalid / data not reliable
///
/// Safety-critical consumers reject [`Self::Void`] before acting on
/// the position or velocity values in the same sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataStatus {
    /// `A` — data is valid and active.
    Active,
    /// `V` — data is void; receiver flagged it as unreliable.
    Void,
    /// Any byte not covered above; raw byte preserved.
    Other(u8),
}

impl DataStatus {
    pub(crate) fn from_byte(b: u8) -> Self {
        match b {
            b'A' | b'a' => Self::Active,
            b'V' | b'v' => Self::Void,
            other => Self::Other(other),
        }
    }
}

/// Radar/ARPA target tracking state, shared by the TTM and TLL target
/// status field.
///
/// - `L` — Lost (target no longer tracked)
/// - `Q` — Query (target being acquired)
/// - `T` — Tracking (target under track)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetStatus {
    /// `L` — target lost.
    Lost,
    /// `Q` — target being acquired / queried.
    Query,
    /// `T` — target under track.
    Tracking,
    /// Any byte not covered above; raw byte preserved.
    Other(u8),
}

impl TargetStatus {
    pub(crate) fn from_byte(b: u8) -> Self {
        match b {
            b'L' | b'l' => Self::Lost,
            b'Q' | b'q' => Self::Query,
            b'T' | b't' => Self::Tracking,
            other => Self::Other(other),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn maps_active_and_void() {
        assert_eq!(DataStatus::from_byte(b'A'), DataStatus::Active);
        assert_eq!(DataStatus::from_byte(b'a'), DataStatus::Active);
        assert_eq!(DataStatus::from_byte(b'V'), DataStatus::Void);
        assert_eq!(DataStatus::from_byte(b'v'), DataStatus::Void);
    }

    #[test]
    fn unknown_byte_preserved() {
        assert_eq!(DataStatus::from_byte(b'X'), DataStatus::Other(b'X'));
    }

    #[test]
    fn target_status_maps_lqt_case_insensitive() {
        assert_eq!(TargetStatus::from_byte(b'L'), TargetStatus::Lost);
        assert_eq!(TargetStatus::from_byte(b'Q'), TargetStatus::Query);
        assert_eq!(TargetStatus::from_byte(b'T'), TargetStatus::Tracking);
        assert_eq!(TargetStatus::from_byte(b't'), TargetStatus::Tracking);
        assert_eq!(TargetStatus::from_byte(b'Z'), TargetStatus::Other(b'Z'));
    }
}
