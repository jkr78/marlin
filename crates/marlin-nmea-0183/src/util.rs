//! Field-level parsing primitives shared by sentence decoders.
//!
//! Public module — downstream crates building their own proprietary
//! decoders can call these.

use alloc::string::{String, ToString};

use crate::DecodeError;

/// Return `None` for an empty byte slice (NMEA's "no data available"
/// marker). Otherwise return `Some(bytes)`.
#[inline]
pub(crate) fn non_empty(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}

/// Parse an optional ASCII float field. Empty → `None`. Non-empty must
/// parse cleanly or an `InvalidNumber` error is returned.
pub(crate) fn optional_f32(bytes: &[u8], field_index: usize) -> Result<Option<f32>, DecodeError> {
    let Some(bytes) = non_empty(bytes) else {
        return Ok(None);
    };
    let s = core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 { field_index })?;
    s.parse::<f32>()
        .map(Some)
        .map_err(|_| DecodeError::InvalidNumber { field_index })
}

/// Parse an optional ASCII unsigned integer field.
pub(crate) fn optional_u8(bytes: &[u8], field_index: usize) -> Result<Option<u8>, DecodeError> {
    let Some(bytes) = non_empty(bytes) else {
        return Ok(None);
    };
    let s = core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 { field_index })?;
    s.parse::<u8>()
        .map(Some)
        .map_err(|_| DecodeError::InvalidNumber { field_index })
}

/// Parse an optional ASCII unsigned 16-bit integer field.
pub(crate) fn optional_u16(bytes: &[u8], field_index: usize) -> Result<Option<u16>, DecodeError> {
    let Some(bytes) = non_empty(bytes) else {
        return Ok(None);
    };
    let s = core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 { field_index })?;
    s.parse::<u16>()
        .map(Some)
        .map_err(|_| DecodeError::InvalidNumber { field_index })
}

/// Parse an optional ASCII text field into an owned `String`. Empty →
/// `None`. Non-empty must be valid UTF-8 (real NMEA text is ASCII).
pub(crate) fn optional_string(
    bytes: &[u8],
    field_index: usize,
) -> Result<Option<String>, DecodeError> {
    let Some(bytes) = non_empty(bytes) else {
        return Ok(None);
    };
    core::str::from_utf8(bytes)
        .map(|s| Some(s.to_string()))
        .map_err(|_| DecodeError::InvalidUtf8 { field_index })
}

/// Parse a magnitude field paired with an `E`/`W` direction byte into a
/// signed `f32` (`E` → positive, `W` → negative). Both empty → `None`;
/// exactly one empty → [`DecodeError::InvalidHemisphere`]; a non-`E`/`W`
/// direction → the same error. Mirrors [`optional_coordinate`]'s
/// both-or-neither rule for the HDG deviation/variation fields.
pub(crate) fn optional_signed_ew(
    magnitude_bytes: &[u8],
    dir_bytes: &[u8],
    magnitude_field_index: usize,
    dir_field_index: usize,
) -> Result<Option<f32>, DecodeError> {
    match (magnitude_bytes.is_empty(), dir_bytes.is_empty()) {
        (true, true) => return Ok(None),
        (true, false) | (false, true) => {
            return Err(DecodeError::InvalidHemisphere {
                field_index: dir_field_index,
            })
        }
        _ => {}
    }
    let s = core::str::from_utf8(magnitude_bytes).map_err(|_| DecodeError::InvalidUtf8 {
        field_index: magnitude_field_index,
    })?;
    let magnitude: f32 = s.parse().map_err(|_| DecodeError::InvalidNumber {
        field_index: magnitude_field_index,
    })?;
    let signed = match dir_bytes.first().copied().unwrap_or(0) {
        b'E' | b'e' => magnitude,
        b'W' | b'w' => -magnitude,
        _ => {
            return Err(DecodeError::InvalidHemisphere {
                field_index: dir_field_index,
            })
        }
    };
    Ok(Some(signed))
}

/// Parse a NMEA coordinate pair (value field + hemisphere field) into a
/// signed decimal-degrees `f64`.
///
/// NMEA encodes latitude as `ddmm.mmmm` and longitude as `dddmm.mmmm`,
/// paired with a hemisphere byte:
/// - Latitude: `N` → positive, `S` → negative.
/// - Longitude: `E` → positive, `W` → negative.
///
/// Empty value field OR empty hemisphere field → `Ok(None)` (NMEA
/// "no data"). Otherwise the two fields must both be present and valid.
///
/// `is_longitude = true` allows degree values up to 180; `false` caps at
/// 90 (latitude).
pub(crate) fn optional_coordinate(
    value_bytes: &[u8],
    hemi_bytes: &[u8],
    value_field_index: usize,
    hemi_field_index: usize,
    is_longitude: bool,
) -> Result<Option<f64>, DecodeError> {
    // Both fields empty → no data. Either both, or neither.
    match (value_bytes.is_empty(), hemi_bytes.is_empty()) {
        (true, true) => return Ok(None),
        (true, false) | (false, true) => {
            return Err(DecodeError::InvalidHemisphere {
                field_index: hemi_field_index,
            })
        }
        _ => {}
    }

    let raw = core::str::from_utf8(value_bytes).map_err(|_| DecodeError::InvalidUtf8 {
        field_index: value_field_index,
    })?;

    // Split at the decimal point to separate "degrees * 100 + minutes"
    // from the fractional minutes.
    let num: f64 = raw.parse().map_err(|_| DecodeError::InvalidNumber {
        field_index: value_field_index,
    })?;

    // Latitude: degrees are the whole-number part / 100.
    // ddmm.mmmm: degrees = num div 100, minutes = num mod 100.
    let degrees_int = (num / 100.0).trunc();
    let minutes = num - (degrees_int * 100.0);
    let decimal = degrees_int + (minutes / 60.0);

    let max = if is_longitude { 180.0 } else { 90.0 };
    if !(-max..=max).contains(&decimal) {
        return Err(DecodeError::OutOfRange {
            field_index: value_field_index,
        });
    }

    let hemi = *hemi_bytes.first().unwrap_or(&0);
    let signed = if is_longitude {
        match hemi {
            b'E' | b'e' => decimal,
            b'W' | b'w' => -decimal,
            _ => {
                return Err(DecodeError::InvalidHemisphere {
                    field_index: hemi_field_index,
                })
            }
        }
    } else {
        match hemi {
            b'N' | b'n' => decimal,
            b'S' | b's' => -decimal,
            _ => {
                return Err(DecodeError::InvalidHemisphere {
                    field_index: hemi_field_index,
                })
            }
        }
    };

    Ok(Some(signed))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn empty_field_is_none() {
        assert!(non_empty(b"").is_none());
        assert_eq!(non_empty(b"abc"), Some(b"abc".as_slice()));
    }

    #[test]
    fn optional_f32_parses_and_handles_empty() {
        assert_eq!(optional_f32(b"", 0).unwrap(), None);
        assert_eq!(optional_f32(b"2.5", 0).unwrap(), Some(2.5));
        assert_eq!(optional_f32(b"-1.5", 0).unwrap(), Some(-1.5));
    }

    #[test]
    fn optional_f32_rejects_garbage() {
        match optional_f32(b"not-a-number", 2) {
            Err(DecodeError::InvalidNumber { field_index: 2 }) => {}
            other => panic!("expected InvalidNumber {{ field_index: 2 }}, got {other:?}"),
        }
    }

    #[test]
    fn coordinate_decode_northern_eastern() {
        // 4807.038 N → 48° + 07.038/60 = 48.1173° N
        let lat = optional_coordinate(b"4807.038", b"N", 2, 3, false)
            .unwrap()
            .unwrap();
        assert!((lat - 48.1173).abs() < 0.0001, "got {lat}");
        // 01131.000 E → 11° + 31/60 = 11.51667° E
        let lon = optional_coordinate(b"01131.000", b"E", 4, 5, true)
            .unwrap()
            .unwrap();
        assert!((lon - 11.51667).abs() < 0.0001, "got {lon}");
    }

    #[test]
    fn coordinate_decode_southern_western_is_negative() {
        let lat = optional_coordinate(b"4807.038", b"S", 2, 3, false)
            .unwrap()
            .unwrap();
        assert!(lat < 0.0);
        let lon = optional_coordinate(b"01131.000", b"W", 4, 5, true)
            .unwrap()
            .unwrap();
        assert!(lon < 0.0);
    }

    #[test]
    fn coordinate_empty_fields_decode_to_none() {
        assert_eq!(optional_coordinate(b"", b"", 2, 3, false).unwrap(), None);
    }

    #[test]
    fn coordinate_rejects_invalid_hemisphere() {
        match optional_coordinate(b"4807.038", b"X", 2, 3, false) {
            Err(DecodeError::InvalidHemisphere { field_index: 3 }) => {}
            other => panic!("expected InvalidHemisphere, got {other:?}"),
        }
    }

    #[test]
    fn coordinate_rejects_out_of_range() {
        // 95° would be latitude > 90°
        match optional_coordinate(b"9500.000", b"N", 2, 3, false) {
            Err(DecodeError::OutOfRange { .. }) => {}
            other => panic!("expected OutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn signed_ew_signs_and_handles_empty() {
        assert_eq!(optional_signed_ew(b"", b"", 1, 2).unwrap(), None);
        assert_eq!(optional_signed_ew(b"12.6", b"E", 1, 2).unwrap(), Some(12.6));
        assert_eq!(optional_signed_ew(b"12.6", b"W", 1, 2).unwrap(), Some(-12.6));
    }

    #[test]
    fn signed_ew_rejects_half_empty_and_bad_dir() {
        match optional_signed_ew(b"12.6", b"", 1, 2) {
            Err(DecodeError::InvalidHemisphere { field_index: 2 }) => {}
            other => panic!("expected InvalidHemisphere 2, got {other:?}"),
        }
        match optional_signed_ew(b"12.6", b"X", 1, 2) {
            Err(DecodeError::InvalidHemisphere { field_index: 2 }) => {}
            other => panic!("expected InvalidHemisphere 2, got {other:?}"),
        }
    }

    #[test]
    fn optional_string_parses_and_handles_empty() {
        assert_eq!(optional_string(b"", 10).unwrap(), None);
        assert_eq!(optional_string(b"TGT1", 10).unwrap(), Some("TGT1".to_string()));
        assert!(optional_string(&[0xFF, 0xFE], 10).is_err());
    }
}
