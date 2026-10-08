//! UTC time-of-day (`hhmmss[.ss]`) and calendar date (`ddmmyy`).

use core::str::FromStr;

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

/// Error from parsing a [`UtcTime`]: the text is not `hhmmss[.sss]`
/// with the hour, minute and second in range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a UTC time of the form hhmmss[.sss]")]
pub struct ParseUtcTimeError;

impl FromStr for UtcTime {
    type Err = ParseUtcTimeError;

    /// Parse `hhmmss` or `hhmmss.fff`. Fractional digits beyond three
    /// are truncated; fewer are padded (`.1` is 100 ms).
    #[allow(clippy::indexing_slicing)] // string length validated above each slice
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (fixed, frac) = match s.find('.') {
            Some(dot) => (&s[..dot], &s[dot.saturating_add(1)..]),
            None => (s, ""),
        };
        if fixed.len() != 6 || !fixed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseUtcTimeError);
        }
        let pair = |src: &str| src.parse::<u8>().map_err(|_| ParseUtcTimeError);
        let hour = pair(&fixed[0..2])?;
        let minute = pair(&fixed[2..4])?;
        let second = pair(&fixed[4..6])?;
        if hour > 23 || minute > 59 || second > 60 {
            return Err(ParseUtcTimeError);
        }

        let millisecond = if frac.is_empty() {
            0
        } else if !frac.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseUtcTimeError);
        } else {
            let three: alloc::string::String = frac
                .chars()
                .chain(core::iter::repeat('0'))
                .take(3)
                .collect();
            three.parse::<u16>().map_err(|_| ParseUtcTimeError)?
        };

        Ok(Self {
            hour,
            minute,
            second,
            millisecond,
        })
    }
}

/// UTC calendar date as carried by RMC's `ddmmyy` field.
///
/// The 2-digit year is preserved verbatim — the spec does not pin a
/// pivot year for century resolution, and different vendors use
/// different rules. Consumers convert to a full year by their own
/// policy (typically `+ 2000` if `year_yy < 80`, else `+ 1900`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDate {
    /// Day of month (1..=31).
    pub day: u8,
    /// Month of year (1..=12).
    pub month: u8,
    /// Year, last two digits (0..=99).
    pub year_yy: u8,
}

/// Error from parsing a [`UtcDate`]: the text is not six digits
/// `ddmmyy` with the day in 1..=31 and the month in 1..=12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a UTC date of the form ddmmyy")]
pub struct ParseUtcDateError;

impl FromStr for UtcDate {
    type Err = ParseUtcDateError;

    #[allow(clippy::indexing_slicing)] // length validated above each slice
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseUtcDateError);
        }
        let pair = |src: &str| src.parse::<u8>().map_err(|_| ParseUtcDateError);
        let day = pair(&s[0..2])?;
        let month = pair(&s[2..4])?;
        let year_yy = pair(&s[4..6])?;
        if !(1..=31).contains(&day) || !(1..=12).contains(&month) {
            return Err(ParseUtcDateError);
        }
        Ok(Self {
            day,
            month,
            year_yy,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn parses_hhmmss_no_fraction() {
        let t: UtcTime = "123519".parse().unwrap();
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
        let t: UtcTime = "092750.123".parse().unwrap();
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
        let t: UtcTime = "000000.1".parse().unwrap();
        assert_eq!(t.millisecond, 100);
    }

    #[test]
    fn rejects_bad_length() {
        assert_eq!("12345".parse::<UtcTime>(), Err(ParseUtcTimeError));
        assert_eq!("1234567".parse::<UtcTime>(), Err(ParseUtcTimeError));
    }

    #[test]
    fn rejects_out_of_range_hour() {
        assert_eq!("243000".parse::<UtcTime>(), Err(ParseUtcTimeError));
    }

    #[test]
    fn rejects_non_digit_fraction() {
        assert_eq!("123519.ab".parse::<UtcTime>(), Err(ParseUtcTimeError));
    }

    #[test]
    fn allows_leap_second() {
        // 23:59:60 is valid for a positive leap second.
        let t: UtcTime = "235960".parse().unwrap();
        assert_eq!(t.second, 60);
    }

    #[test]
    fn utc_date_parses_ddmmyy() {
        let d: UtcDate = "230394".parse().unwrap();
        assert_eq!(
            d,
            UtcDate {
                day: 23,
                month: 3,
                year_yy: 94
            }
        );
    }

    #[test]
    fn utc_date_rejects_wrong_length() {
        assert_eq!("23039".parse::<UtcDate>(), Err(ParseUtcDateError));
    }

    #[test]
    fn utc_date_rejects_invalid_month() {
        assert_eq!("231394".parse::<UtcDate>(), Err(ParseUtcDateError));
    }

    #[test]
    fn utc_date_rejects_zero_day() {
        assert_eq!("000394".parse::<UtcDate>(), Err(ParseUtcDateError));
    }

    #[test]
    fn parse_errors_display_the_expected_form() {
        assert_eq!(
            ParseUtcTimeError.to_string(),
            "not a UTC time of the form hhmmss[.sss]"
        );
        assert_eq!(
            ParseUtcDateError.to_string(),
            "not a UTC date of the form ddmmyy"
        );
    }
}
