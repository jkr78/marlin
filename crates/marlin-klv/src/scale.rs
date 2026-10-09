//! MISB ST 0601 legacy linear scaling (NOT ST 1201 IMAPB — those are newer tags,
//! out of scope; any tag beyond the current table must be re-researched, not
//! pattern-matched).
//!
//! Decode half: each signed width partitions its wire count into the ST 0601 sentinel
//! (`i16::MIN` = 0x8000, `i32::MIN` = 0x80000000), a sender error carrying the raw
//! code, or a scaled value; the unsigned widths scale straight to a value. Encode half:
//! `encode` range-checks the engineering value before calling a `units_to_*` function,
//! so the clamp inside each one is only a cast guard and the sentinel is never produced
//! by scaling.

use marlin_field::{FieldState, RawCode};

/// Unsigned full-range map: `0..=65535` → `0.0..=span`.
pub(crate) fn u16_to_units(raw: u16, span: f64) -> f64 {
    f64::from(raw) * span / 65535.0
}

/// Unsigned full-range map: `0..=u32::MAX` → `0.0..=span`.
pub(crate) fn u32_to_units(raw: u32, span: f64) -> f64 {
    f64::from(raw) * span / 4_294_967_295.0
}

/// Signed symmetric partition: `i16::MIN` is the sentinel, every other count
/// maps `-32767..=32767` → `±half_span`.
pub(crate) fn i16_field(raw: i16, half_span: f64) -> FieldState<f64> {
    if raw == i16::MIN {
        FieldState::SenderError(RawCode(raw.into()))
    } else {
        FieldState::Value(f64::from(raw) * half_span / 32767.0)
    }
}

/// Signed symmetric partition: `i32::MIN` is the sentinel, every other count
/// maps `-2147483647..=2147483647` → `±half_span`.
pub(crate) fn i32_field(raw: i32, half_span: f64) -> FieldState<f64> {
    if raw == i32::MIN {
        FieldState::SenderError(RawCode(raw.into()))
    } else {
        FieldState::Value(f64::from(raw) * half_span / 2_147_483_647.0)
    }
}

/// Offset map (altitude family): `0..=65535` → `offset ..= offset+span`.
pub(crate) fn u16_offset_to_units(raw: u16, span: f64, offset: f64) -> f64 {
    f64::from(raw) * span / 65535.0 + offset
}

/// Whether `v` lies in `lo..=hi`. NaN is never in range.
pub(crate) fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    (lo..=hi).contains(&v)
}

// All encoders below clamp into the target integer's range before `libm::round`, so the
// final `as` cast is provably in range and sign-correct. The caller has already
// range-checked the engineering value; the clamp is a cast guard, not a policy.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u16(v: f64, span: f64) -> u16 {
    libm::round(v.clamp(0.0, span) / span * 65535.0) as u16
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u32(v: f64, span: f64) -> u32 {
    libm::round(v.clamp(0.0, span) / span * 4_294_967_295.0) as u32
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn units_to_i16(v: f64, half_span: f64) -> i16 {
    libm::round(v.clamp(-half_span, half_span) / half_span * 32767.0) as i16
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn units_to_i32(v: f64, half_span: f64) -> i32 {
    libm::round(v.clamp(-half_span, half_span) / half_span * 2_147_483_647.0) as i32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u16_offset(v: f64, span: f64, offset: f64) -> u16 {
    libm::round((v.clamp(offset, offset + span) - offset) / span * 65535.0) as u16
}

/// Identity m/s → u8 (Tag 8).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u8(v: f64) -> u8 {
    libm::round(v.clamp(0.0, 255.0)) as u8
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn signed_sentinel_is_a_sender_error_with_the_raw_code() {
        assert_eq!(
            i16_field(i16::MIN, 50.0),
            FieldState::SenderError(RawCode(-32768))
        );
        assert_eq!(
            i32_field(i32::MIN, 90.0),
            FieldState::SenderError(RawCode(-2147483648))
        );
    }

    #[test]
    fn signed_extremes_map_to_half_span() {
        assert_eq!(i16_field(32767, 50.0), FieldState::Value(50.0));
        assert_eq!(i16_field(-32767, 50.0), FieldState::Value(-50.0));
        assert_eq!(i32_field(2_147_483_647, 90.0), FieldState::Value(90.0));
    }

    #[test]
    fn range_check_is_inclusive_and_rejects_nan() {
        assert!(in_range(90.0, -90.0, 90.0));
        assert!(in_range(-90.0, -90.0, 90.0));
        assert!(!in_range(90.0001, -90.0, 90.0));
        assert!(!in_range(f64::NAN, -90.0, 90.0));
    }

    #[test]
    fn cast_guard_never_emits_the_sentinel() {
        assert_eq!(
            units_to_i16(-50.0, 50.0),
            -32767,
            "range minimum, NOT -32768"
        );
        assert_eq!(units_to_i16(50.0, 50.0), 32767);
        assert_eq!(units_to_i32(-90.0, 90.0), -2_147_483_647);
        assert_eq!(units_to_i32(90.0, 90.0), 2_147_483_647);
        assert_eq!(units_to_u16(360.0, 360.0), 65535);
        assert_eq!(units_to_u32(360.0, 360.0), u32::MAX);
        assert_eq!(units_to_u8(255.0), 255);
    }

    #[test]
    fn altitude_offset_round_trips() {
        let raw = units_to_u16_offset(0.0, 19900.0, -900.0);
        let back = u16_offset_to_units(raw, 19900.0, -900.0);
        assert!(
            (back - 0.0).abs() < 0.5,
            "0 m round-trips within LSB (~0.3 m), got {back}"
        );
    }

    #[test]
    fn heading_kat_from_vector_1() {
        // 0x71c2 = 29122 → 159.97436484321355° (klvdata expected value)
        let deg = u16_to_units(0x71c2, 360.0);
        assert!((deg - 159.97436484321355).abs() < 1e-9, "got {deg}");
    }
}
