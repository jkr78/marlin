//! Decoded-field state type shared by the marlin decoders.
//!
//! Every wire field that the sender can put in a non-value state decodes to
//! one [`FieldState`]: a value, an over-range bound, not available, a sender
//! error carrying its raw code, or invalid. A field the wire cannot put in
//! any other state keeps its bare type. The decoder crates re-export the
//! four items here flat at their roots; this crate holds no decoding of its
//! own and depends on nothing.
//!
//! `#![no_std]` without `alloc`, unlike the decoder crates: the type holds
//! no heap data of its own, and `FieldState<String>` works because the
//! caller supplies the `T`.
//!
//! # Example
//! ```
//! use marlin_field::{FieldState, Kind};
//!
//! let speed: FieldState<f32> = FieldState::AtLeast(102.2);
//! assert_eq!(speed.value(), None);
//! assert_eq!(speed.value_or_bound(), Some(102.2));
//! assert_eq!(speed.kind(), Kind::AtLeast);
//! assert_eq!(speed.kind().name(), "at_least");
//! ```
#![no_std]

#[cfg(test)]
extern crate std;

/// The undecoded wire integer a sentinel state retains: the raw code.
///
/// `i64` holds every wire width in the suite: AIS fields up to 30 bits, the
/// ST 0601 `i32::MIN` sender-error code, NMEA 0183 bytes and digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawCode(pub i64);

/// Why a field is invalid: the decoder could not give the wire field a
/// meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Invalid {
    /// A number inside the field's width that the specification leaves
    /// undefined, such as a reserved enum code or a latitude beyond ±90°
    /// by any code other than not available. Carries the raw code.
    Undefined(RawCode),
    /// No wire integer of the field's width could be read: unparsable
    /// 0183 text, a wrong-length KLV tag.
    Unparsable,
}

/// A field state with the value erased, obtained from [`FieldState::kind`].
///
/// The two error states keep their payload so a log line can show the raw
/// code; a loop that counts states across fields of different types keys
/// on [`name`](Self::name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The field holds a value ([`FieldState::Value`]).
    Value,
    /// The field is over-range and holds a bound ([`FieldState::AtLeast`]).
    AtLeast,
    /// The field is not available ([`FieldState::NotAvailable`]).
    NotAvailable,
    /// The field is a sender error; carries the raw code
    /// ([`FieldState::SenderError`]).
    SenderError(RawCode),
    /// The field is invalid; carries why ([`FieldState::Invalid`]).
    Invalid(Invalid),
}

impl Kind {
    /// The stable cross-language tag for this state: `value`, `at_least`,
    /// `not_available`, `sender_error` or `invalid`.
    ///
    /// This is the one table of state names; the Python bindings print the
    /// same strings, so logs from both languages agree.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::AtLeast => "at_least",
            Self::NotAvailable => "not_available",
            Self::SenderError(_) => "sender_error",
            Self::Invalid(_) => "invalid",
        }
    }
}

/// The decoded outcome of one wire field: the field state.
///
/// Exhaustive on purpose, so a `match` that misses a state fails to
/// compile. The derives are bounded on `T` by the derive itself, so
/// `FieldState<f64>` has `PartialEq` but no `Eq` or `Hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldState<T> {
    /// The wire gave the field a meaning and the decoder accepted it. A
    /// status inside a status-carrying field (AIS timestamp 61–63, rate of
    /// turn ±127) is a value of the field's own enum, not a separate state.
    Value(T),
    /// Over-range: the true value is at or beyond this bound, which is
    /// already in engineering units (speed 102.2 kn or more). The bound is
    /// not a value; [`value`](Self::value) returns `None` for it.
    AtLeast(T),
    /// The sender supplied no value: a sentinel code, an empty 0183 field,
    /// an omitted KLV tag, or a payload that ends before the field. Not an
    /// error.
    NotAvailable,
    /// The sender knows it has no usable value, such as the ST 0601
    /// `i32::MIN` code on a signed tag. Carries the raw code.
    SenderError(RawCode),
    /// The decoder could not give the wire field a meaning; see [`Invalid`]
    /// for which way.
    Invalid(Invalid),
}

impl<T> FieldState<T> {
    /// The value, if the field holds one. An over-range bound is not a
    /// value; use [`value_or_bound`](Self::value_or_bound) to accept it.
    #[must_use]
    pub fn value(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    /// The value or the over-range bound, if the field holds either: for
    /// a client that takes "102.2 kn or more" as 102.2.
    #[must_use]
    pub fn value_or_bound(self) -> Option<T> {
        match self {
            Self::Value(v) | Self::AtLeast(v) => Some(v),
            _ => None,
        }
    }

    /// The state with the value erased.
    #[must_use]
    pub fn kind(&self) -> Kind {
        match self {
            Self::Value(_) => Kind::Value,
            Self::AtLeast(_) => Kind::AtLeast,
            Self::NotAvailable => Kind::NotAvailable,
            Self::SenderError(raw) => Kind::SenderError(*raw),
            Self::Invalid(why) => Kind::Invalid(*why),
        }
    }

    /// Applies `f` to the value or the bound; every other state passes
    /// through unchanged.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> FieldState<U> {
        match self {
            Self::Value(v) => FieldState::Value(f(v)),
            Self::AtLeast(v) => FieldState::AtLeast(f(v)),
            Self::NotAvailable => FieldState::NotAvailable,
            Self::SenderError(raw) => FieldState::SenderError(raw),
            Self::Invalid(why) => FieldState::Invalid(why),
        }
    }

    /// Borrows the value or the bound, so a string field can be read from
    /// a borrowed message without cloning it.
    #[must_use]
    pub fn as_ref(&self) -> FieldState<&T> {
        match self {
            Self::Value(v) => FieldState::Value(v),
            Self::AtLeast(v) => FieldState::AtLeast(v),
            Self::NotAvailable => FieldState::NotAvailable,
            Self::SenderError(raw) => FieldState::SenderError(*raw),
            Self::Invalid(why) => FieldState::Invalid(*why),
        }
    }

    /// Mutably borrows the value or the bound.
    #[must_use]
    pub fn as_mut(&mut self) -> FieldState<&mut T> {
        match self {
            Self::Value(v) => FieldState::Value(v),
            Self::AtLeast(v) => FieldState::AtLeast(v),
            Self::NotAvailable => FieldState::NotAvailable,
            Self::SenderError(raw) => FieldState::SenderError(*raw),
            Self::Invalid(why) => FieldState::Invalid(*why),
        }
    }
}

/// A bare `T` is the value state, so a hand-built set is written
/// `set.sensor_latitude_degrees = 60.1768.into()`.
impl<T> From<T> for FieldState<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use std::string::String;

    use super::*;

    #[test]
    fn value_is_some_only_for_the_value_state() {
        assert_eq!(FieldState::Value(7_u8).value(), Some(7));
        assert_eq!(FieldState::AtLeast(7_u8).value(), None);
        assert_eq!(FieldState::<u8>::NotAvailable.value(), None);
        assert_eq!(FieldState::<u8>::SenderError(RawCode(-1)).value(), None);
        assert_eq!(FieldState::<u8>::Invalid(Invalid::Unparsable).value(), None);
    }

    #[test]
    fn value_or_bound_is_some_for_value_and_over_range() {
        assert_eq!(FieldState::Value(7_u8).value_or_bound(), Some(7));
        assert_eq!(FieldState::AtLeast(7_u8).value_or_bound(), Some(7));
        assert_eq!(FieldState::<u8>::NotAvailable.value_or_bound(), None);
        assert_eq!(
            FieldState::<u8>::SenderError(RawCode(-1)).value_or_bound(),
            None
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Unparsable).value_or_bound(),
            None
        );
    }

    #[test]
    fn kind_erases_the_value_and_keeps_the_raw_code() {
        assert_eq!(FieldState::Value(7_u8).kind(), Kind::Value);
        assert_eq!(FieldState::AtLeast(7_u8).kind(), Kind::AtLeast);
        assert_eq!(FieldState::<u8>::NotAvailable.kind(), Kind::NotAvailable);
        assert_eq!(
            FieldState::<u8>::SenderError(RawCode(-1)).kind(),
            Kind::SenderError(RawCode(-1))
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Undefined(RawCode(91))).kind(),
            Kind::Invalid(Invalid::Undefined(RawCode(91)))
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Unparsable).kind(),
            Kind::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn map_applies_to_value_and_bound_and_keeps_the_other_states() {
        let double = |v: u8| u16::from(v) * 2;
        assert_eq!(
            FieldState::Value(7_u8).map(double),
            FieldState::Value(14_u16)
        );
        assert_eq!(
            FieldState::AtLeast(7_u8).map(double),
            FieldState::AtLeast(14_u16)
        );
        assert_eq!(
            FieldState::<u8>::NotAvailable.map(double),
            FieldState::NotAvailable
        );
        assert_eq!(
            FieldState::<u8>::SenderError(RawCode(-1)).map(double),
            FieldState::SenderError(RawCode(-1))
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Unparsable).map(double),
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn as_ref_borrows_the_value_and_bound() {
        let value = FieldState::Value(7_u8);
        assert_eq!(value.as_ref(), FieldState::Value(&7));
        let bound = FieldState::AtLeast(7_u8);
        assert_eq!(bound.as_ref(), FieldState::AtLeast(&7));
        assert_eq!(
            FieldState::<u8>::NotAvailable.as_ref(),
            FieldState::NotAvailable
        );
        assert_eq!(
            FieldState::<u8>::SenderError(RawCode(-1)).as_ref(),
            FieldState::SenderError(RawCode(-1))
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Unparsable).as_ref(),
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn as_ref_reads_a_string_field_without_taking_it() {
        let name = FieldState::Value(String::from("EVER GIVEN"));
        assert_eq!(
            name.as_ref().value().map(String::as_str),
            Some("EVER GIVEN")
        );
        assert_eq!(name, FieldState::Value(String::from("EVER GIVEN")));
    }

    #[test]
    fn as_mut_writes_through_to_the_value_and_bound() {
        let mut value = FieldState::Value(7_u8);
        if let FieldState::Value(v) = value.as_mut() {
            *v = 8;
        }
        assert_eq!(value, FieldState::Value(8));
        let mut bound = FieldState::AtLeast(7_u8);
        if let FieldState::AtLeast(v) = bound.as_mut() {
            *v = 8;
        }
        assert_eq!(bound, FieldState::AtLeast(8));
        assert_eq!(
            FieldState::<u8>::NotAvailable.as_mut(),
            FieldState::NotAvailable
        );
        assert_eq!(
            FieldState::<u8>::SenderError(RawCode(-1)).as_mut(),
            FieldState::SenderError(RawCode(-1))
        );
        assert_eq!(
            FieldState::<u8>::Invalid(Invalid::Unparsable).as_mut(),
            FieldState::Invalid(Invalid::Unparsable)
        );
    }

    #[test]
    fn from_lifts_a_bare_value_into_the_value_state() {
        let state: FieldState<f64> = 60.1768.into();
        assert_eq!(state, FieldState::Value(60.1768));
    }

    #[test]
    fn kind_name_is_the_stable_snake_case_tag() {
        let table = [
            (Kind::Value, "value"),
            (Kind::AtLeast, "at_least"),
            (Kind::NotAvailable, "not_available"),
            (Kind::SenderError(RawCode(-1)), "sender_error"),
            (Kind::Invalid(Invalid::Undefined(RawCode(91))), "invalid"),
            (Kind::Invalid(Invalid::Unparsable), "invalid"),
        ];
        for (kind, name) in table {
            assert_eq!(kind.name(), name, "{kind:?}");
        }
    }

    /// Compile-pass pin for the derive bounds: `FieldState<f64>` compares
    /// with `PartialEq` alone, and an `Eq + Hash` payload lifts both.
    #[test]
    fn derives_follow_the_payload_bounds() {
        fn eq_and_hash<T: Eq + core::hash::Hash>(_: &T) {}
        let float = FieldState::Value(1.5_f64);
        assert_eq!(float, FieldState::Value(1.5_f64));
        assert_ne!(float, FieldState::AtLeast(1.5_f64));
        eq_and_hash(&FieldState::Value(7_u8));
        eq_and_hash(&Kind::Invalid(Invalid::Undefined(RawCode(91))));
    }
}
