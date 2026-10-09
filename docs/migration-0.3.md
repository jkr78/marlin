# Migrating to 0.3.0

0.3.0 changes the type of nearly every decoded field. A field the wire
can put in a non-value state is a `FieldState<T>` from the new
`marlin-field` crate, re-exported flat at the root of `marlin-nmea-0183`,
`marlin-ais` and `marlin-klv` (ADR-0008). The Python bindings carry the
same field as `marlin.field.FieldState` with one variant class per state
(ADR-0009). Every public error type carries its crate's prefix
(ADR-0010). The terms used below are the "Field state" entries of
[`GLOSSARY.md`](../GLOSSARY.md).

The one-line migration where an `Option<T>` was read before is
`.value()`, which is `Some` only for the `Value` state. Everything else
in this guide is for the sites that want to tell the other four states
apart.

```rust
pub enum FieldState<T> {
    Value(T),              // the wire gave it a meaning, the decoder accepted it
    AtLeast(T),            // over-range: the true value is at or beyond this bound
    NotAvailable,          // the sender supplied no value
    SenderError(RawCode),  // the sender knows it has no usable value
    Invalid(Invalid),      // the decoder could not give the wire field a meaning
}
pub enum Invalid { Undefined(RawCode), Unparsable }
pub struct RawCode(pub i64);
```

```python
from marlin.field import FieldState

FieldState.Value(10.2)         # .kind == "value"
FieldState.AtLeast(102.2)      # .kind == "at_least"
FieldState.NotAvailable()      # .kind == "not_available"
FieldState.SenderError(-32768) # .kind == "sender_error"
FieldState.Invalid(13)         # .kind == "invalid"; code is None when no
                               # wire integer could be read
```

A field's value never fails the message any more. A decode returns an
error for a structural reason only: framing, the checksum, a layout the
decoder does not expect, or the one mandatory field in the suite, the
ST 0601 Tag 2 timestamp.

## Numeric field

Every `Option<f32>`, `Option<f64>`, `Option<u8>`, `Option<u16>` and
`Option<u32>` field is a `FieldState` of the same payload. An over-range
code, which used to pass through as the value it names, is
`AtLeast(bound)` with the bound in engineering units: vessel speed
102.2 kn, Type 9 speed 1022 kn and altitude 4094 m, draught 25.5 m,
dimensions 511 m and 63 m.

```rust
// 0.2.0
if let Some(sog) = report.speed_over_ground {
    println!("{sog} kn"); // 102.2 may be the over-range code
}
```

```rust
// 0.3.0
use marlin_ais::FieldState;

match report.speed_over_ground {
    FieldState::Value(sog) => println!("{sog} kn"),
    FieldState::AtLeast(bound) => println!("{bound} kn or more"),
    FieldState::NotAvailable => {}
    FieldState::SenderError(code) => println!("sender error {code:?}"),
    FieldState::Invalid(why) => println!("invalid: {why:?}"),
}

// Where an Option<f32> was read before:
let sog: Option<f32> = report.speed_over_ground.value();
// Where a bound is as good as a value:
let sog_or_bound: Option<f32> = report.speed_over_ground.value_or_bound();
```

```python
# 0.2.0
if body.speed_over_ground is not None:
    print(body.speed_over_ground)
```

```python
# 0.3.0
from marlin.field import FieldState

match body.speed_over_ground:
    case FieldState.Value(sog):
        print(f"{sog} kn")
    case FieldState.AtLeast(bound):
        print(f"{bound} kn or more")
    case FieldState.NotAvailable():
        pass
    case FieldState.SenderError(code) | FieldState.Invalid(code):
        print(f"no speed: {code}")

# Where `is not None` was tested before:
if body.speed_over_ground.value is not None:
    print(body.speed_over_ground.value)
```

The same applies to every 0183 numeric field (`hdop`, `altitude_m`,
`speed_knots`, `utc`, ...) and to the KLV scaled tags (next sections).

## Enum field

A bare enum field is `FieldState<E>`, and the enum loses the members that
were states in disguise. The not-available code decodes to
`NotAvailable`; a reserved or undefined code decodes to
`Invalid(Undefined(RawCode(code)))` with the wire integer.

| Enum | Removed | Replaced by |
| --- | --- | --- |
| `NavStatus` | `NotDefined`, `Reserved(u8)` | `NotAvailable`, `Invalid(Undefined(RawCode(9..=13)))` |
| `ManeuverIndicator` | `NotAvailable`, `Reserved` | `NotAvailable`, `Invalid(Undefined(RawCode(3)))` |
| `EpfdType` | `Undefined`, `Reserved(u8)` | `NotAvailable`, `Invalid(Undefined(RawCode(9..=14)))` |
| `GgaFixQuality` | `Other(u8)`; `Invalid` renamed `NoFix` | `Invalid(Undefined(RawCode(digit)))`; `NoFix` is a value the sender reported |
| the seven 0183 letter-code enums | `Other(u8)` | `Invalid(Undefined(RawCode(byte)))` |

```rust
// 0.2.0
use marlin_ais::NavStatus;

match report.navigation_status {
    NavStatus::NotDefined => {}
    NavStatus::Reserved(code) => println!("reserved {code}"),
    status => println!("{status:?}"),
}
```

```rust
// 0.3.0
use marlin_ais::{FieldState, Invalid, RawCode};

match report.navigation_status {
    FieldState::Value(status) => println!("{status:?}"),
    FieldState::NotAvailable => {}
    FieldState::Invalid(Invalid::Undefined(RawCode(code))) => println!("reserved {code}"),
    _ => {}
}
```

```python
# 0.2.0
from marlin.ais import NavStatus

if body.navigation_status != NavStatus.NOT_DEFINED:
    print(body.navigation_status)
```

```python
# 0.3.0
match body.navigation_status:
    case FieldState.Value(status):
        print(status)
    case FieldState.Invalid(code):
        print(f"reserved {code}")
    case _:
        pass
```

The Python enums lose the same members: `NavStatus.NOT_DEFINED`,
`ManeuverIndicator.NOT_AVAILABLE`, `ManeuverIndicator.RESERVED`,
`EpfdType.UNDEFINED`, and the catch-all `UNKNOWN` member of
`TargetStatus`, `AngleReference`, `DistanceUnits` and `AcquisitionType`;
`GgaFixQuality.INVALID` is `NO_FIX`. `PrdidDialect.UNKNOWN` stays: it is
a decode option, not a decoded field.

## Status-carrying field

A numeric field whose code space also carries statuses is a `FieldState`
of the field's own enum, so a status is a value and only the
not-available code is a state. The AIS timestamp was a bare `u8`; it is
`FieldState<Timestamp>`, where 0..=59 is `Timestamp::Second` and 61..=63
is `Timestamp::PositioningStatus`. Rate of turn was
`Option<RateOfTurn>`; it is `FieldState<RateOfTurn>`.

```rust
// 0.2.0
if report.timestamp <= 59 {
    println!("fix at second {}", report.timestamp);
}
if let Some(RateOfTurn::DegPerMin(rate)) = report.rate_of_turn {
    println!("{rate} deg/min");
}
```

```rust
// 0.3.0
use marlin_ais::{FieldState, RateOfTurn, Timestamp};

match report.timestamp {
    FieldState::Value(Timestamp::Second(second)) => println!("fix at second {second}"),
    FieldState::Value(Timestamp::PositioningStatus(status)) => println!("{status:?}"),
    FieldState::NotAvailable => {} // code 60
    _ => {}
}
if let FieldState::Value(RateOfTurn::DegPerMin(rate)) = report.rate_of_turn {
    println!("{rate} deg/min");
}
```

In Python the sibling attributes that flattened these enums are gone:
`turn_direction` beside `rate_of_turn`, and `dimensions` plus
`mothership_mmsi` beside the new `StaticDataB24B.extent`. Each is one
sum type with a variant class per Rust variant.

```python
# 0.2.0
if body.turn_direction is not None:
    print(f"turning {body.turn_direction}, no indicator")
elif body.rate_of_turn is not None:
    print(f"{body.rate_of_turn} deg/min")
```

```python
# 0.3.0
from marlin.ais import RateOfTurn, Timestamp

match body.rate_of_turn:
    case FieldState.Value(RateOfTurn.DegPerMin(rate)):
        print(f"{rate} deg/min")
    case FieldState.Value(RateOfTurn.NoIndicator(direction)):
        print(f"turning {direction}, no indicator")
    case _:
        pass

match body.timestamp:
    case FieldState.Value(Timestamp.Second(second)):
        print(f"fix at second {second}")
    case FieldState.Value(Timestamp.PositioningStatus(status)):
        print(status)
    case _:
        pass
```

## String field

`Option<String>` is `FieldState<String>`: all padding, or an empty 0183
field, is `NotAvailable`; 0183 text that is not UTF-8 is
`Invalid(Unparsable)`. `value()` takes the state by value, so a borrow
goes through `as_ref()`.

```rust
// 0.2.0
if let Some(name) = report.vessel_name.as_deref() {
    println!("{name}");
}
```

```rust
// 0.3.0
if let Some(name) = report.vessel_name.as_ref().value() {
    println!("{name}");
}
// or
let name: Option<&str> = report.vessel_name.as_ref().value().map(String::as_str);
```

```python
# 0.2.0
if body.vessel_name:
    print(body.vessel_name)
```

```python
# 0.3.0
if (name := body.vessel_name.value) is not None:
    print(name)
```

`if body.vessel_name:` still parses and is always true: every field
state is truthy, `NotAvailable()` included. See the trap below.

## Paired 0183 field

Latitude with its `N`/`S` letter, longitude with `E`/`W`, and the HDG
and RMC `E`/`W` magnitudes are paired fields: two wire fields, one
state. Both empty is `NotAvailable`; exactly one empty, or a magnitude
the decoder cannot read, is `Invalid(Unparsable)`; a letter outside the
pair is `Invalid(Undefined(RawCode(byte)))`. In 0.2.0 the half-filled
and unreadable cases failed the whole sentence.

```rust
// 0.2.0
use marlin_nmea_0183::{decode_gga, DecodeError};

match decode_gga(&raw) {
    Ok(gga) => println!("{:?}", gga.latitude_deg),
    Err(DecodeError::InvalidNumber { .. }) => println!("bad latitude"),
    Err(err) => println!("{err}"),
}
```

```rust
// 0.3.0
use marlin_nmea_0183::{decode_gga, FieldState, Invalid};

match decode_gga(&raw) {
    Ok(gga) => match gga.latitude_deg {
        FieldState::Value(lat) => println!("{lat}"),
        FieldState::Invalid(Invalid::Unparsable) => println!("bad latitude"),
        other => println!("{other:?}"),
    },
    Err(err) => println!("{err}"), // only NotEnoughFields
}
```

`Nmea0183DecodeError` keeps `NotEnoughFields` alone; the arms for
`InvalidNumber`, `OutOfRange`, `InvalidHemisphere`, `InvalidUtf8` and
`InvalidUtcTime` go. `decode` on a known sentence type with enough fields
never returns `Err`.

```python
# 0.2.0
try:
    gga = decode_gga(raw)
except DecodeError:
    gga = None  # a bad field or a short sentence
```

```python
# 0.3.0
try:
    gga = decode_gga(raw)  # DecodeError only for a short sentence
except DecodeError:
    gga = None
else:
    lat = gga.latitude_deg.value  # None for a bad or empty pair
```

## KLV wire count to engineering unit

`St0601` holds engineering units: every scaled tag is a `FieldState<f64>`
named as the former getter (`sensor_latitude_degrees`,
`slant_range_meters`, `platform_true_airspeed_mps`), and `version` is a
`FieldState<u8>`. The wire-count fields, the twenty getter/setter pairs
and `Default` are gone. `St0601::new(timestamp_us)` builds a set with
every optional field `NotAvailable`. An omitted tag is `NotAvailable`;
the ST 0601 sentinel on a signed tag (`i16::MIN`, `i32::MIN`) is
`SenderError(RawCode)` with the raw code, where the getter used to
return `None`; a known tag with the wrong wire length is
`Invalid(Unparsable)` with its bytes kept in `unknown`.

```rust
// 0.2.0
let mut set = St0601 { timestamp_us: 1_700_000_000_000_000, ..Default::default() };
set.set_sensor_latitude_degrees(60.1768);
let lat: Option<f64> = set.sensor_latitude_degrees();
let raw: Option<i32> = set.sensor_latitude;
```

```rust
// 0.3.0
use marlin_klv::{FieldState, RawCode, St0601};

let mut set = St0601::new(1_700_000_000_000_000);
set.sensor_latitude_degrees = FieldState::Value(60.1768); // or 60.1768.into()
let lat: Option<f64> = set.sensor_latitude_degrees.value();

// The wire count survives only inside the sender-error state:
set.platform_pitch_degrees = FieldState::SenderError(RawCode(i16::MIN.into()));
```

```python
# 0.2.0
s = St0601()
s.timestamp_us = 1_700_000_000_000_000
s.sensor_latitude_degrees = 60.1768
lat = s.sensor_latitude_degrees      # float | None
raw = s.raw_sensor_latitude          # int | None
```

```python
# 0.3.0
from marlin.klv import St0601

s = St0601(timestamp_us=1_700_000_000_000_000)  # timestamp_us is required
s.sensor_latitude_degrees = 60.1768              # Value; None is NotAvailable()
lat = s.sensor_latitude_degrees.value            # float | None
s.platform_pitch_degrees = FieldState.SenderError(-32768)
```

## KLV fallible encode

`encode` and `encode_to_bytes` return `Result<_, KlvEncodeError>`. A
value outside its tag's range, or NaN, is `OutOfRange { tag }`, where it
used to be clamped (NaN to the range minimum). A state the wire cannot
carry (`AtLeast`, `Invalid(Undefined(_))`, a `SenderError` whose code is
not the tag's own sentinel, any `SenderError` on an unsigned tag) is
`Unencodable { tag, kind }`. On `Err` the output buffer is untouched.

```rust
// 0.2.0
set.set_sensor_latitude_degrees(91.0); // clamped to 90.0
marlin_klv::encode(&set, &mut out).unwrap();
```

```rust
// 0.3.0
use marlin_klv::KlvEncodeError;

set.sensor_latitude_degrees = FieldState::Value(91.0);
match marlin_klv::encode(&set, &mut out) {
    Ok(()) => {}
    Err(KlvEncodeError::OutOfRange { tag }) => println!("tag {tag} out of range"),
    Err(KlvEncodeError::Unencodable { tag, kind }) => {
        println!("tag {tag} cannot carry {}", kind.name());
    }
    Err(err) => println!("{err}"),
}
```

```python
# 0.2.0
s.sensor_latitude_degrees = 91.0  # clamped to 90.0
wire = encode(s)
```

```python
# 0.3.0
from marlin.klv import KlvEncodeError, encode

s.sensor_latitude_degrees = 91.0
try:
    wire = encode(s)
except KlvEncodeError as err:
    print(err.variant, err.tag, err.kind)  # out_of_range 13 None
```

`KlvEncodeError` subclasses `KlvError`, so an existing `except KlvError`
around `encode` still catches it.

## Python read and write

Every field-state attribute reads a `FieldState[T]`. The base class
gives `kind` (the snake-case state tag, the same strings as the Rust
`Kind::name()`), `value` (`None` unless the state is `Value`) and
`value_or_bound` (`None` unless `Value` or `AtLeast`).

```python
state = body.speed_over_ground
state.kind              # "value" | "at_least" | "not_available" | "sender_error" | "invalid"
state.value             # float | None
state.value_or_bound    # float | None
isinstance(state, FieldState.AtLeast)

match state.kind:       # the same strings the Rust Kind::name() returns
    case "value" | "at_least":
        print(state.value_or_bound)
    case "not_available":
        pass
    case "sender_error" | "invalid":
        print("unusable")
```

**The trap.** A field state has no `__bool__`, so it is always truthy:
`if body.speed_over_ground:` is true for `NotAvailable()` as well. A
falsy not-available state would have made `Value(0.0)` indistinguishable
from it. Port `if msg.field:` and `if msg.field is not None:` to
`if msg.field.value is not None:` or to a state check.

Constructors and `St0601` setters accept `FieldState[T] | T | None`: a
bare value becomes `Value`, `None` becomes `NotAvailable()`, a
`FieldState` passes through. A payload of the wrong type raises
`TypeError`; an integer outside the field's width raises
`OverflowError`. Every field-state keyword defaults to `NotAvailable()`.

```python
from marlin.ais import PositionReportA

report = PositionReportA(mmsi=123_456_789, true_heading=240)
report.true_heading               # FieldState.Value(240)
report.speed_over_ground          # FieldState.NotAvailable()
```

The dataclass mirrors in `marlin.dataclasses` carry a field as one of
five generic frozen dataclasses `Value`, `AtLeast`, `NotAvailable`,
`SenderError` and `Invalid`, each with a `kind` literal, so `asdict`
yields `{"true_heading": {"kind": "value", "value": 240}}`; an enum
payload is stored as its integer value.

The Python floor is 3.10 (`requires-python >= 3.10`, `abi3-py310`
wheels), so every example above can use `match`.

## Error type names

Every public error type carries its crate's prefix, with an operation
word where a crate has more than one (ADR-0010). The variants are
unchanged except where a section above says so.

| Crate | 0.2.0 | 0.3.0 |
| --- | --- | --- |
| `marlin-nmea-envelope` | `Error` | `EnvelopeError` |
| `marlin-nmea-0183` | `DecodeError` | `Nmea0183DecodeError` |
| `marlin-nmea-0183` | `PsxnLayoutParseError` | `ParsePsxnLayoutError` |
| `marlin-nmea-0183` | `Nmea0183Error` | unchanged |
| `marlin-ais` | `AisError` | unchanged |
| `marlin-klv` | `Error` | `KlvDecodeError` |
| `marlin-klv` | (new) | `KlvEncodeError` |

`KlvDecodeError` gains `MissingChecksum` (Tag 1 absent or not 2 bytes)
and `BadTimestamp` (Tag 2 absent or not 8 bytes); a `match` on it
already needs a wildcard arm, since the enum is `#[non_exhaustive]`. The
Python exception names are unchanged.

## Also gone

- `marlin_ais::sentinel` and its constants, `SOG_OVER_RANGE_KN`
  included: the state a field is in is on the field, so there is no
  code to compare against.
- `Default` on `Dimensions`, `Eta` and `St0601`: no fabricated set.
