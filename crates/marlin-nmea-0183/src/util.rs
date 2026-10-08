//! Field-level producers shared by the sentence decoders.
//!
//! Private module: each producer maps one wire field (or one paired
//! field) to a [`FieldState`] and never fails the sentence. Every
//! producer reads an empty field as [`FieldState::NotAvailable`].

use alloc::string::{String, ToString};
use core::ops::Neg;
use core::str::FromStr;

use marlin_field::{FieldState, Invalid, RawCode};

/// Decode a numeric field through `T`'s [`FromStr`].
///
/// Empty → `NotAvailable`; text that is not NMEA numeric text, or a
/// parse failure → `Invalid(Unparsable)` (numeric text has no wire
/// integer to carry); else `Value`.
pub(crate) fn number<T: FromStr>(bytes: &[u8]) -> FieldState<T> {
    if bytes.is_empty() {
        return FieldState::NotAvailable;
    }
    numeric_text(bytes)
        .and_then(|s| s.parse::<T>().ok())
        .map_or(FieldState::Invalid(Invalid::Unparsable), FieldState::Value)
}

/// The field as NMEA numeric text: an optional leading sign, then ASCII
/// digits and decimal points only. `None` for anything else, so the
/// spellings Rust's [`FromStr`] would accept but the standard does not
/// (`nan`, `inf`, `1e5`) never become a value.
fn numeric_text(bytes: &[u8]) -> Option<&str> {
    let digits = match bytes {
        [b'+' | b'-', rest @ ..] => rest,
        _ => bytes,
    };
    if digits.iter().all(|b| b.is_ascii_digit() || *b == b'.') {
        core::str::from_utf8(bytes).ok()
    } else {
        None
    }
}

/// Decode a one-byte letter code through `from_byte`, which names the
/// bytes the enum defines (case-insensitively).
///
/// Empty → `NotAvailable`; one named byte → `Value`; one unnamed byte →
/// `Invalid(Undefined(RawCode(byte)))`; two or more bytes →
/// `Invalid(Unparsable)`.
pub(crate) fn code<E>(bytes: &[u8], from_byte: fn(u8) -> Option<E>) -> FieldState<E> {
    match bytes {
        [] => FieldState::NotAvailable,
        &[byte] => from_byte(byte).map_or(
            FieldState::Invalid(Invalid::Undefined(RawCode(byte.into()))),
            FieldState::Value,
        ),
        _ => FieldState::Invalid(Invalid::Unparsable),
    }
}

/// Decode a text field into an owned `String`.
///
/// Empty → `NotAvailable`; non-UTF-8 → `Invalid(Unparsable)`; else
/// `Value`. Real NMEA text is ASCII.
pub(crate) fn text(bytes: &[u8]) -> FieldState<String> {
    if bytes.is_empty() {
        return FieldState::NotAvailable;
    }
    core::str::from_utf8(bytes).map_or(FieldState::Invalid(Invalid::Unparsable), |s| {
        FieldState::Value(s.to_string())
    })
}

/// Decode a paired field: a magnitude field and its sign-letter field
/// into one signed value.
///
/// `positive` and `negative` are the two letters the pair defines
/// (`N`/`S`, `E`/`W`), matched case-insensitively; `magnitude_of` turns
/// the magnitude text into the unsigned value, `None` when it cannot.
///
/// Both empty → `NotAvailable`; exactly one empty → `Invalid(Unparsable)`;
/// a magnitude the decoder cannot read → `Invalid(Unparsable)`; one-byte
/// letter outside the pair → `Invalid(Undefined(RawCode(byte)))`; letter of
/// two or more bytes → `Invalid(Unparsable)`; else `Value(signed)`.
pub(crate) fn paired<T: Neg<Output = T>>(
    magnitude_bytes: &[u8],
    sign_bytes: &[u8],
    positive: u8,
    negative: u8,
    magnitude_of: fn(&str) -> Option<T>,
) -> FieldState<T> {
    let negate = match (magnitude_bytes.is_empty(), sign_bytes) {
        (true, []) => return FieldState::NotAvailable,
        (false, &[letter]) if letter.eq_ignore_ascii_case(&positive) => false,
        (false, &[letter]) if letter.eq_ignore_ascii_case(&negative) => true,
        (false, &[letter]) => {
            return FieldState::Invalid(Invalid::Undefined(RawCode(letter.into())))
        }
        // A half-filled pair, or a letter of two or more bytes.
        _ => return FieldState::Invalid(Invalid::Unparsable),
    };
    let magnitude = numeric_text(magnitude_bytes).and_then(magnitude_of);
    match magnitude {
        Some(m) if negate => FieldState::Value(-m),
        Some(m) => FieldState::Value(m),
        None => FieldState::Invalid(Invalid::Unparsable),
    }
}

/// Decode a latitude paired field (`ddmm.mmmm` with `N`/`S`) into signed
/// decimal degrees, north positive. A magnitude beyond 90° is
/// `Invalid(Unparsable)`.
pub(crate) fn latitude(value_bytes: &[u8], hemi_bytes: &[u8]) -> FieldState<f64> {
    paired(value_bytes, hemi_bytes, b'N', b'S', |s| {
        degrees_from_dm(s).filter(|d| (0.0..=90.0).contains(d))
    })
}

/// Decode a longitude paired field (`dddmm.mmmm` with `E`/`W`) into
/// signed decimal degrees, east positive. A magnitude beyond 180° is
/// `Invalid(Unparsable)`.
pub(crate) fn longitude(value_bytes: &[u8], hemi_bytes: &[u8]) -> FieldState<f64> {
    paired(value_bytes, hemi_bytes, b'E', b'W', |s| {
        degrees_from_dm(s).filter(|d| (0.0..=180.0).contains(d))
    })
}

/// Decode a degrees magnitude paired with an `E`/`W` direction into a
/// signed `f32` (`E` positive, `W` negative): HDG deviation and
/// variation, RMC magnetic variation.
pub(crate) fn signed_ew(magnitude_bytes: &[u8], dir_bytes: &[u8]) -> FieldState<f32> {
    paired(magnitude_bytes, dir_bytes, b'E', b'W', |s| s.parse().ok())
}

/// The bytes of an optional trailing field: empty when the sentence is
/// too short to carry it, which decodes like an empty field.
pub(crate) fn optional<'a>(fields: &[&'a [u8]], index: usize) -> &'a [u8] {
    fields.get(index).copied().unwrap_or_default()
}

/// Decode the radar reference-target flag.
///
/// Empty → `Value(false)`, the standard's encoding of "not the reference
/// target", never `NotAvailable`; `R`/`r` → `Value(true)`; other one
/// byte → `Invalid(Undefined(RawCode(byte)))`; two or more bytes →
/// `Invalid(Unparsable)`.
pub(crate) fn reference_target(bytes: &[u8]) -> FieldState<bool> {
    match bytes {
        [] => FieldState::Value(false),
        &[byte] if byte.eq_ignore_ascii_case(&b'R') => FieldState::Value(true),
        &[byte] => FieldState::Invalid(Invalid::Undefined(RawCode(byte.into()))),
        _ => FieldState::Invalid(Invalid::Unparsable),
    }
}

/// `ddmm.mmmm` (or `dddmm.mmmm`) text to unsigned decimal degrees.
fn degrees_from_dm(s: &str) -> Option<f64> {
    let num: f64 = s.parse().ok()?;
    if num < 0.0 {
        return None;
    }
    let degrees_int = (num / 100.0).trunc();
    let minutes = num - (degrees_int * 100.0);
    Some(degrees_int + (minutes / 60.0))
}
