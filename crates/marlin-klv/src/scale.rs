//! MISB ST 0601 legacy linear scaling (NOT ST 1201 IMAPB — those are newer tags,
//! out of scope; any tag beyond the current table must be re-researched, not pattern-matched).
//!
//! Sentinel policy: on DECODE, the reserved error indicators (`i16::MIN` = 0x8000,
//! `i32::MIN` = 0x80000000) yield `None` from accessors. On ENCODE, inputs are clamped to
//! the tag's valid range BEFORE conversion, so the sentinel can never be emitted
//! (e.g. −50.0° clamps to count −32767, never −32768). NaN clamps to the range minimum.

/// Unsigned full-range map: `0..=65535` → `0.0..=span`.
pub(crate) fn u16_to_units(raw: u16, span: f64) -> f64 {
    f64::from(raw) * span / 65535.0
}

/// Unsigned full-range map: `0..=u32::MAX` → `0.0..=span`.
pub(crate) fn u32_to_units(raw: u32, span: f64) -> f64 {
    f64::from(raw) * span / 4_294_967_295.0
}

/// Signed symmetric map: `-32767..=32767` → `±half_span`; `i16::MIN` is the sentinel.
pub(crate) fn i16_to_units(raw: i16, half_span: f64) -> Option<f64> {
    if raw == i16::MIN {
        None
    } else {
        Some(f64::from(raw) * half_span / 32767.0)
    }
}

/// Signed symmetric map: `-2147483647..=2147483647` → `±half_span`; `i32::MIN` is the sentinel.
pub(crate) fn i32_to_units(raw: i32, half_span: f64) -> Option<f64> {
    if raw == i32::MIN {
        None
    } else {
        Some(f64::from(raw) * half_span / 2_147_483_647.0)
    }
}

/// Offset map (altitude family): `0..=65535` → `offset ..= offset+span`.
pub(crate) fn u16_offset_to_units(raw: u16, span: f64, offset: f64) -> f64 {
    f64::from(raw) * span / 65535.0 + offset
}

// All encoders below clamp into the target integer's range before `libm::round`, so the
// final `as` cast is provably in range and sign-correct.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u16(v: f64, span: f64) -> u16 {
    libm::round(nan_to(v, 0.0).clamp(0.0, span) / span * 65535.0) as u16
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u32(v: f64, span: f64) -> u32 {
    libm::round(nan_to(v, 0.0).clamp(0.0, span) / span * 4_294_967_295.0) as u32
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn units_to_i16(v: f64, half_span: f64) -> i16 {
    libm::round(nan_to(v, -half_span).clamp(-half_span, half_span) / half_span * 32767.0) as i16
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn units_to_i32(v: f64, half_span: f64) -> i32 {
    libm::round(nan_to(v, -half_span).clamp(-half_span, half_span) / half_span * 2_147_483_647.0)
        as i32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u16_offset(v: f64, span: f64, offset: f64) -> u16 {
    libm::round((nan_to(v, offset).clamp(offset, offset + span) - offset) / span * 65535.0) as u16
}

/// Identity m/s → u8 (Tag 8). Clamp to 0..=255, NaN → 0.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn units_to_u8(v: f64) -> u8 {
    libm::round(nan_to(v, 0.0).clamp(0.0, 255.0)) as u8
}

fn nan_to(v: f64, fallback: f64) -> f64 {
    if v.is_nan() { fallback } else { v }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn signed_sentinel_decodes_to_none() {
        assert_eq!(i16_to_units(i16::MIN, 50.0), None);
        assert_eq!(i32_to_units(i32::MIN, 90.0), None);
    }

    #[test]
    fn signed_extremes_map_to_half_span() {
        assert_eq!(i16_to_units(32767, 50.0), Some(50.0));
        assert_eq!(i16_to_units(-32767, 50.0), Some(-50.0));
        assert_eq!(i32_to_units(2_147_483_647, 90.0), Some(90.0));
    }

    #[test]
    fn encode_clamps_and_never_emits_sentinel() {
        assert_eq!(units_to_i16(-999.0, 50.0), -32767, "clamped min, NOT -32768");
        assert_eq!(units_to_i16(999.0, 50.0), 32767);
        assert_eq!(units_to_i32(-999.0, 90.0), -2_147_483_647);
        assert_eq!(units_to_i32(999.0, 90.0), 2_147_483_647);
        assert_eq!(units_to_u16(-5.0, 360.0), 0);
        assert_eq!(units_to_u16(999.0, 360.0), 65535);
        assert_eq!(units_to_u32(-1.0, 360.0), 0);
        assert_eq!(units_to_u32(9999.0, 360.0), u32::MAX);
        assert_eq!(units_to_u8(-5.0), 0);
        assert_eq!(units_to_u8(9999.0), 255);
    }

    #[test]
    fn nan_clamps_to_range_minimum() {
        assert_eq!(units_to_u16(f64::NAN, 360.0), 0);
        assert_eq!(units_to_i16(f64::NAN, 50.0), -32767);
        assert_eq!(units_to_u16_offset(f64::NAN, 19900.0, -900.0), 0);
    }

    #[test]
    fn altitude_offset_round_trips() {
        let raw = units_to_u16_offset(0.0, 19900.0, -900.0);
        let back = u16_offset_to_units(raw, 19900.0, -900.0);
        assert!((back - 0.0).abs() < 0.5, "0 m round-trips within LSB (~0.3 m), got {back}");
    }

    #[test]
    fn heading_kat_from_vector_1() {
        // 0x71c2 = 29122 → 159.97436484321355° (klvdata expected value)
        let deg = u16_to_units(0x71c2, 360.0);
        assert!((deg - 159.97436484321355).abs() < 1e-9, "got {deg}");
    }
}
