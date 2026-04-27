//! UTC time decoding (`hhmmss[.ss]` format).

use crate::DecodeError;

/// UTC time-of-day with millisecond resolution.
///
/// NMEA encodes UTC as `hhmmss` or `hhmmss.ss` — two-digit hour, two-
/// digit minute, two-digit second, optional fractional second. This
/// struct preserves resolution down to milliseconds; sub-millisecond
/// fractions are truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTime {
    /// Hour (0..=23).
    pub hour: u8,
    /// Minute (0..=59).
    pub minute: u8,
    /// Second (0..=60 — NMEA allows 60 for leap seconds).
    pub second: u8,
    /// Millisecond (0..=999).
    pub millisecond: u16,
}

impl UtcTime {
    /// Parse `hhmmss` or `hhmmss.fff` from an ASCII byte slice. Returns
    /// [`DecodeError::InvalidUtcTime`] on malformed input.
    #[allow(clippy::indexing_slicing)] // string length validated above each slice
    pub(crate) fn parse(bytes: &[u8], field_index: usize) -> Result<Self, DecodeError> {
        let s =
            core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtcTime { field_index })?;
        if s.len() < 6 {
            return Err(DecodeError::InvalidUtcTime { field_index });
        }

        let (fixed, frac) = match s.find('.') {
            Some(dot) => (&s[..dot], &s[dot.saturating_add(1)..]),
            None => (s, ""),
        };
        if fixed.len() != 6 || !fixed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DecodeError::InvalidUtcTime { field_index });
        }

        let parse_pair = |src: &str| -> Result<u8, DecodeError> {
            src.parse::<u8>()
                .map_err(|_| DecodeError::InvalidUtcTime { field_index })
        };

        let hour = parse_pair(&fixed[0..2])?;
        let minute = parse_pair(&fixed[2..4])?;
        let second = parse_pair(&fixed[4..6])?;
        if hour > 23 || minute > 59 || second > 60 {
            return Err(DecodeError::InvalidUtcTime { field_index });
        }

        // Fractional seconds: pad or truncate to exactly 3 digits for ms.
        let millisecond = if frac.is_empty() {
            0
        } else if !frac.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DecodeError::InvalidUtcTime { field_index });
        } else {
            let three: alloc::string::String = frac
                .chars()
                .chain(core::iter::repeat('0'))
                .take(3)
                .collect();
            three
                .parse::<u16>()
                .map_err(|_| DecodeError::InvalidUtcTime { field_index })?
        };

        Ok(Self {
            hour,
            minute,
            second,
            millisecond,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_hhmmss_no_fraction() {
        let t = UtcTime::parse(b"123519", 0).unwrap();
        assert_eq!(
            t,
            UtcTime {
                hour: 12,
                minute: 35,
                second: 19,
                millisecond: 0
            }
        );
    }

    #[test]
    fn parses_hhmmss_with_fraction() {
        let t = UtcTime::parse(b"092750.123", 0).unwrap();
        assert_eq!(
            t,
            UtcTime {
                hour: 9,
                minute: 27,
                second: 50,
                millisecond: 123
            }
        );
    }

    #[test]
    fn parses_hhmmss_with_short_fraction_pads() {
        // `.1` means 100 ms, not 1 ms.
        let t = UtcTime::parse(b"000000.1", 0).unwrap();
        assert_eq!(t.millisecond, 100);
    }

    #[test]
    fn rejects_bad_length() {
        match UtcTime::parse(b"12345", 2) {
            Err(DecodeError::InvalidUtcTime { field_index: 2 }) => {}
            other => panic!("expected InvalidUtcTime, got {other:?}"),
        }
    }

    #[test]
    fn rejects_out_of_range_hour() {
        match UtcTime::parse(b"243000", 0) {
            Err(DecodeError::InvalidUtcTime { .. }) => {}
            other => panic!("expected InvalidUtcTime, got {other:?}"),
        }
    }

    #[test]
    fn allows_leap_second() {
        // 23:59:60 is valid for a positive leap second.
        let t = UtcTime::parse(b"235960", 0).unwrap();
        assert_eq!(t.second, 60);
    }
}
