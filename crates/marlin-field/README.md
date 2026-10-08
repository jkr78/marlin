# marlin-field

Decoded-field state type shared by the `marlin` decoders.

## What this crate does

- Defines [`FieldState<T>`], the decoded outcome of one wire field: a
  value, an over-range bound, not available, a sender error with its raw
  code, or invalid
- Defines [`Kind`], the state with the value erased, and `Kind::name()`,
  the stable snake-case tag shared with the Python bindings
- Defines [`RawCode`] and [`Invalid`], the payloads of the two error states

`marlin-nmea-0183`, `marlin-ais` and `marlin-klv` re-export these four
items flat at their roots from 0.3.0, so code written against one decoder
reads fields from the others. `marlin-nmea-envelope` does not use it.

## What this crate does not do

- **No decoding.** Which wire code maps to which state is each decoder's
  rule; this crate holds only the type.
- **No `Default`, `Display`, ordering or `serde`.** A state is matched
  exhaustively or read through `value()`, `value_or_bound()` and `kind()`;
  how it renders is the client's call.

## Quickstart

```rust
use marlin_field::{FieldState, Invalid, Kind, RawCode};

fn describe(speed: FieldState<f32>) -> &'static str {
    match speed {
        FieldState::Value(_) => "measured",
        FieldState::AtLeast(_) => "at or beyond the bound",
        FieldState::NotAvailable => "not available",
        FieldState::SenderError(RawCode(_)) => "sender error",
        FieldState::Invalid(Invalid::Undefined(_)) => "undefined code",
        FieldState::Invalid(Invalid::Unparsable) => "unparsable",
    }
}

let speed = FieldState::AtLeast(102.2_f32);
assert_eq!(speed.value(), None);           // a bound is not a value
assert_eq!(speed.value_or_bound(), Some(102.2));
assert_eq!(speed.kind(), Kind::AtLeast);
assert_eq!(speed.kind().name(), "at_least");
assert_eq!(describe(speed), "at or beyond the bound");

let lat: FieldState<f64> = 60.1768.into(); // a bare value is Value
assert_eq!(lat, FieldState::Value(60.1768));
```

## Architectural commitments

- **`#![no_std]` without `alloc`.** The type holds no heap data; a
  `FieldState<String>` works because the caller supplies the `T`.
- **Exhaustive.** No `#[non_exhaustive]`: a `match` that misses a state
  fails to compile.
- **No dependencies.** The leaf every decoder can sit on.

## Minimum Supported Rust Version

1.82

## License

Licensed under either of Apache License 2.0 or MIT License at your option.

[`FieldState<T>`]: https://docs.rs/marlin-field/latest/marlin_field/enum.FieldState.html
[`Kind`]: https://docs.rs/marlin-field/latest/marlin_field/enum.Kind.html
[`RawCode`]: https://docs.rs/marlin-field/latest/marlin_field/struct.RawCode.html
[`Invalid`]: https://docs.rs/marlin-field/latest/marlin_field/enum.Invalid.html
