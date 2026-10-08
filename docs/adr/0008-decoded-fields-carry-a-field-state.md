# Decoded fields carry a field state

Supersedes [ADR-0001](0001-sentinel-representation.md) from 0.3.0.

Through 0.2.0 the three decoders had three policies for a wire field that
holds no measurement. marlin-ais mapped a not-available code to `None`,
passed an over-range code through as the value it names and published the
codes as constants (ADR-0001). marlin-nmea-0183 mapped an empty field to
`None` and failed the whole sentence on a field it could not parse.
marlin-klv kept the raw wire integer and returned `None` from the accessor
for the ST 0601 error indicator. A client that wanted to know whether a
field was sent as null, sent as an error, rejected by the decoder, or
measured had to consult sentinel constants, catch sentence errors, or read
raw counts, and the answer differed per crate.

From 0.3.0 every wire field the sender can put in a non-value state is a
`FieldState<T>` from the new `marlin-field` crate, re-exported flat at the
root of each decoder crate:

```rust
pub enum FieldState<T> {
    Value(T),                // the wire gave it a meaning, the decoder accepted it
    AtLeast(T),              // over-range: the true value is at or beyond this bound
    NotAvailable,            // the sender supplied no value
    SenderError(RawCode),    // a sentinel meaning the sender knows it has no usable value
    Invalid(Invalid),        // the decoder could not give the wire value a meaning
}
pub enum Invalid { Undefined(RawCode), Unparsable }
pub struct RawCode(pub i64);
```

The five states are the ones `GLOSSARY.md` defines under "Field state". A
field the wire cannot put in any state but value (a one-bit flag, an opaque
bit block, the MMSI) is a plain field and keeps its bare type. The raw wire
integer survives only inside the two error states; `AtLeast` carries the
bound alone, because an encoder derives the over-range code from the field
definition. The type is exhaustive, has no `Default`, no `Display`, no
ordering and no `serde`; `value()` and `value_or_bound()` reach `Option<T>`
in one call and `kind()` gives a value-erased `Kind` for loops and for the
Python `kind` tag through `Kind::name()`. The crate is `#![no_std]` without
`alloc`; the decoders depend on it, the envelope does not.

**A field's value never fails the message.** A decode returns an error only
for framing, checksum, layout (too few 0183 fields, an AIS payload below
its floor, KLV framing) or a mandatory field absent, of which the suite has
exactly one: ST 0601 Tag 2. The sentence checksum is the envelope's; the
typed crates never re-check it and carry no policy knob.

The per-crate consequences that cost something, and why they were paid:

- **Statuses inside a measurement are values, not states.** AIS timestamp
  61–63 and rate of turn ±127 are variants of the field's own enum
  (`Timestamp::PositioningStatus`, `RateOfTurn::NoIndicator`), so a loop
  over `Kind` counts them as `Value`. A sixth state and splitting the wire
  field into a value field plus a status field were both rejected: the
  first fixes a shape the field's enum already answers, the second lets an
  encoder be handed a contradiction.
- **0183 numeric rejections carry no raw code.** 0183 numeric text has no
  wire integer, so every parse or range failure is `Invalid(Unparsable)`;
  `Invalid::Undefined(RawCode)` is reserved for one-byte letter codes and
  the GGA quality digit, where `RawCode(9)` is the digit, not the ASCII
  byte. A paired field (magnitude plus its sign letter) takes one state and
  a half-filled pair is invalid, not not available. Lowercase code letters
  stay accepted. PSXN's derived angles are `FieldState` though computed, so
  the crate has one convention; a trig failure reads `Invalid(Unparsable)`.
- **AIS floors follow traffic, with one departure from gpsd.** Type 5 drops
  to 420 bits, where its destination is read at character granularity and
  its `dte` becomes `FieldState<bool>`, not available when bit 422 lies past
  the payload; Types 9 and 19 keep `dte: bool` because their floors cover
  the bit. The rule is that a floor lowering wraps the fields it exposes.
  Types 1/2/3 stay at 168 rather than gpsd's 163, which would make
  `radio_status` partly absent and force a state onto lower-layer data.
  Character-granularity truncation is distinct from ADR-0007 saturation:
  the partial character's bits are never read. ADR-0007 stands as the read
  primitive, but a decoder checks a field's bit range against `total_bits`
  before trusting a saturated read, so a short payload yields not available
  rather than a fabricated zero. Draught 255 becomes `AtLeast(25.5)`; it was
  a plain 25.5. The `sentinel` module leaves the public API.
- **KLV fields hold engineering units.** Every scaled ST 0601 tag is a
  `FieldState<f64>` in degrees, metres or metres per second; raw-count
  storage and the accessor pairs go, because count → `f64` → count is exact
  for every count in each tag's width. `INT_MIN` on the nine signed tags is
  the only sender-error source in the suite; an omitted tag is the only
  not-available source; `AtLeast` and `Invalid::Undefined` are unreachable.
  A known tag with the wrong wire length is `Invalid(Unparsable)` with its
  bytes kept in `unknown`, so the lossless promise survives in this form: a
  decoded packet in framing order re-encodes to its source bytes, and any
  decodable packet re-encodes to a set equal to the first decode.
- **KLV encode is fallible.** Every state decode can produce re-encodes
  (`Value` as a range-checked count, `NotAvailable` as an omitted tag,
  `SenderError` as the tag's own indicator, `Invalid(Unparsable)` as nothing
  while `unknown` carries the bytes); every other state is
  `EncodeError::Unencodable`, and a `Value` outside the tag's range or NaN is
  `EncodeError::OutOfRange`. Clamping and NaN-to-minimum are gone: a NaN
  encoded as −50° was a fabricated reading. On `Err` the output buffer is
  untouched.

## Considered options

- `Result<Option<T>, E>` per field: over-range has nowhere honest to live,
  `?` aborts a function on a bound, and `.ok().flatten()` collapses the
  bound and both error states into `None`.
- A raw newtype per field with `raw()`, `is_over_range()` and so on, as
  `aivdm` does: no `match`, predicates must be asked in the right order,
  and each newtype reinvents the state view.
- Collapsing invalid into not available, or keeping the sentence failure
  for a bad 0183 field: both hide what the sender did.
- A copy of the type per crate: three drifting definitions and no shared
  Python mapping.
- Transitional shims for `marlin_ais::sentinel` and the KLV `*_degrees()`
  getters: the old `Option` fields cannot coexist with `FieldState` fields
  of the same name, and `value()` is the one-line migration.
