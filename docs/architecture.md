# Architecture

## marlin-field

Leaf crate holding the field state type every decoder re-exports:
`FieldState<T>`, `Kind`, `RawCode` and `Invalid`, flat at the root with
no other public item. It is `#![no_std]` **without** `alloc`, unlike the
decoder crates: the type holds no heap data of its own, and a
`FieldState<String>` works because the caller supplies the `T`. It has no
dependencies. `marlin-nmea-0183`, `marlin-ais` and `marlin-klv` depend on
it and re-export the four items from 0.3.0; `marlin-nmea-envelope` does
not depend on it, since the envelope yields raw sentences and assigns no
field a state. The decision and the per-crate consequences are ADR-0008.

### Surface

```rust
pub struct RawCode(pub i64);
pub enum Invalid { Undefined(RawCode), Unparsable }
pub enum Kind { Value, AtLeast, NotAvailable, SenderError(RawCode), Invalid(Invalid) }
pub enum FieldState<T> { Value(T), AtLeast(T), NotAvailable, SenderError(RawCode), Invalid(Invalid) }

impl Kind { fn name(&self) -> &'static str }
impl<T> FieldState<T> {
    fn value(self) -> Option<T>;            // Value only
    fn value_or_bound(self) -> Option<T>;   // Value or AtLeast
    fn kind(&self) -> Kind;
    fn map<U>(self, f: impl FnOnce(T) -> U) -> FieldState<U>;
    fn as_ref(&self) -> FieldState<&T>;
    fn as_mut(&mut self) -> FieldState<&mut T>;
}
impl<T> From<T> for FieldState<T>;          // T → Value(T)
```

All four types derive `Debug, Clone, Copy, PartialEq, Eq, Hash`, bounded
on `T` by the derive itself, so `FieldState<f64>` has `PartialEq` but no
`Eq` or `Hash`. Deliberately absent: `Default` (no fabricated value),
`#[non_exhaustive]` (clients match exhaustively), `Display` (rendering is
client presentation), `PartialOrd`/`Ord` (variant order is not a ranking),
`serde` (no consumer yet), `From<Option<T>>` (`None` would silently mean
not available where invalid was meant), predicates and `Option`
combinator duplicates (`value()` reaches them in one call), and any
decoder-side construction helper (the three decoders map code spaces
three ways).

### `Kind::name()`

The one table of state names. The Python `kind` attribute prints the
same strings, so logs from both languages agree.

| `Kind` | `name()` |
| --- | --- |
| `Value` | `value` |
| `AtLeast` | `at_least` |
| `NotAvailable` | `not_available` |
| `SenderError(_)` | `sender_error` |
| `Invalid(_)` | `invalid` |

## marlin-klv

`#![no_std]` + `alloc` crate: a MISB ST 0601 (UAS Datalink Local Set)
KLV encoder/decoder. Unlike `marlin-nmea-0183` and `marlin-ais`, it does
not depend on `marlin-nmea-envelope`: KLV is not NMEA-framed, so there's
no shared envelope layer to sit on. Its only marlin dependency is
`marlin-field`, from 0.3.0, as the `marlin-field` section above says.

### Modules

- `ber` — BER length codec (short and long form) and big-endian
  fixed-width readers.
- `scale` — MISB ST 0601 legacy linear scaling (not ST 1201 IMAPB;
  those are newer tags, out of scope today). The decode half partitions
  each signed width's count into the ST 0601 sentinel, read as a sender
  error with its raw code, or a scaled value; the encode half turns a
  range-checked engineering value back into a count.
- `tags` — macro-driven registry of the 20 scaled tags. Each
  `scaled_tags!` entry expands to that tag's encode arm, its
  decode-dispatch arm and its `TAGS` row.
- `checksum` — BCC-16 (Tag 1) running-sum checksum.
- `error` — `KlvDecodeError` (non_exhaustive, `Clone`): truncated
  input, BER length overflow, wrong local-set key, checksum mismatch or
  missing, Tag 2 absent or not 8 bytes. `KlvEncodeError`
  (non_exhaustive, `Clone`): a field state the tag cannot carry, a value
  outside the tag's range or NaN.
- `st0601` — the encode/decode orchestrator. Owns `St0601` and
  `St0601::new`, `UAS_LS_KEY`, `encode`, `decode`, `precision_timestamp`,
  and (behind the `bytes` feature) `encode_to_bytes`. Composes `ber` +
  `scale` + `tags` + `checksum`.

### Data model

`St0601` holds engineering units. Every scaled tag is a `FieldState<f64>`
in degrees, metres or metres per second, named for its unit
(`sensor_latitude_degrees`, `slant_range_meters`,
`platform_true_airspeed_mps`); `version` is a `FieldState<u8>`;
`timestamp_us` is a plain `u64`, the one mandatory field. No wire counts
are stored: count → `f64` → count is exact for every count in each
tag's width, so nothing is lost. `St0601::new(timestamp_us)` is the
constructor, with every optional field `NotAvailable` and `unknown`
empty; there is no `Default`, and a field is assigned directly
(`set.sensor_latitude_degrees = 60.1768.into()`).

The states a tag can be in:

- An omitted tag is `NotAvailable`, the only not-available source.
- The ST 0601 sentinel on a signed tag (`i16::MIN` on Tags 6 and 7,
  `i32::MIN` on Tags 13, 14, 19, 23, 24, 40 and 41) is
  `SenderError(RawCode)` with the raw code. This is the only
  sender-error source in the suite, and the only place a wire count
  survives: a client that wants counts calls `encode`.
- A known tag with the wrong wire length (Tag 65 included) is
  `Invalid(Unparsable)` on its field, and its bytes are kept in
  `unknown`. When the same tag also occurs with the right length, the
  readable occurrence keeps the field (last readable wins) and the
  wrong-length bytes still ride in `unknown`.
- `AtLeast` and `Invalid::Undefined` are unreachable on every ST 0601
  tag.

Unrecognized tags round-trip verbatim through
`St0601::unknown: Vec<(u8, Vec<u8>)>`, preserved in wire order.
`unknown` is unvalidated: a typed tag pushed there by hand is emitted
twice and last-wins on re-decode.

### Invariants

- **Round-trip.** `decode(encode(set)) == set` for every set `decode`
  produces (fuzz-asserted on `klv_decode`), and `encode(decode(b)) == b`
  for every decodable packet `b` whose items already sit in the framing
  order below, a known tag with an unexpected wire length included. A
  packet in any other item order decodes to the same set but re-encodes
  in framing order, so its bytes differ. A hand-built set is not
  covered: `Value` quantises to the tag's resolution.
- **A field's value never fails the set.** `decode` fails only for a
  structural reason: framing (`BadKey`, `Truncated`, `LengthOverflow`),
  the checksum (`BadChecksum`, `MissingChecksum`) or the mandatory Tag 2
  (`BadTimestamp`). A checksum mismatch outranks a bad Tag 2: the
  timestamp is settled after the checksum is verified.
- **Fallible encode.** Every state `decode` can produce re-encodes: a
  `Value` as its range-checked count, `NotAvailable` as an omitted tag,
  `SenderError` as the tag's own sentinel, `Invalid(Unparsable)` as
  nothing for the typed tag while `unknown` carries the bytes. Every
  other state (`AtLeast`, `Invalid(Undefined)`, a `SenderError` whose
  code is not the tag's own sentinel or that sits on an unsigned tag or
  Tag 65) is `KlvEncodeError::Unencodable`. A `Value` outside the tag's
  engineering range, inclusive at both ends and checked before
  rounding, or NaN, is `KlvEncodeError::OutOfRange`; nothing is pulled
  into range. On `Err` the output buffer is untouched.
- **Framing order.** Tag 2 first, then Tag 65 (version) if present,
  then the scaled tags in ascending tag order, then preserved unknown
  tags in original order, then Tag 1 (checksum) last.

### no_std rounding

Encoding rounds engineering values to the nearest wire count with
`libm::round` (round-half-away-from-zero), since `f64::round` isn't
available in `core` under `no_std`. `libm` is a plain dependency, not
feature-gated.

### Optional `bytes` feature

`encode_to_bytes` (behind the `bytes` feature) encodes directly into a
`bytes::Bytes`, for callers that hand ownership downstream instead of
taking a `Vec<u8>`.

## marlin-ais

`#![no_std]` + `alloc` crate that decodes AIS messages carried in
`!AIVDM` / `!AIVDO` sentences. It sits on `marlin-nmea-envelope`, which
frames the sentence and checks the checksum. It does not depend on
`marlin-nmea-0183`.

```text
bytes → marlin-nmea-envelope → RawSentence → parse_aivdm_wrapper
      → AisReassembler → armor::decode → decode_message → AisMessage
```

### Modules

- `aivdm` — `parse_aivdm_wrapper` reads the wrapper fields into
  `AivdmHeader`: fragment count and number, sequential id, channel,
  armored payload, fill bits, and `is_own_ship` (`!AIVDO`).
- `armor` — the 6-bit ASCII armor codec (IEC 61162-1). `decode` turns
  payload characters into a packed bit buffer plus a bit count with the
  fill bits removed.
- `bit_reader` — `BitReader` reads unsigned, two's-complement signed,
  boolean, and 6-bit-ASCII string fields (ITU-R M.1371-5 Annex 8 §3,
  Table 47). A read past the end returns zeros and never panics.
- `reassembly` — `AisReassembler` joins multi-sentence payloads.
- `message` — `AisMessage { is_own_ship, body }`, the `AisMessageBody`
  enum, and the dispatcher: `decode_message` for a bit buffer, `decode`
  for a single-fragment `RawSentence`.
- `parser` — `AisFragmentParser<P>` runs the whole pipeline over any
  envelope `SentenceSource`. For a source mode chosen at runtime it
  wraps the envelope's `Parser` enum, which the crate re-exports; the
  crate has no mode enum of its own (ADR-0005).
- `shared_types` — `Dimensions`, `EpfdType`, `Timestamp`,
  `PositioningStatus`, `is_auxiliary_craft_mmsi`, and the crate-private
  code-space partitions the decoders share: a `Partition<T>` table per
  numeric field (longitude, latitude, COG, heading, SOG, ship type,
  dimensions) and `coded` for the enum fields and the timestamp. The
  not-available and over-range codes are crate-private constants.
- `error` — `AisError` (`non_exhaustive`).
- One module per message layout, each with a public `decode_*` function
  and a minimum-bit-count constant:

| Module | Types | Struct | ITU-R M.1371-5 Annex 8 |
| --- | --- | --- | --- |
| `position_report_a` | 1, 2, 3 | `PositionReportA` | §3.1, Table 48 |
| `static_voyage_a` | 5 | `StaticAndVoyageA` | §3.3, Table 52 |
| `sar_aircraft_position_report` | 9 | `SarAircraftPositionReport` | §3.7, Table 59 |
| `position_report_b` | 18 | `PositionReportB` | §3.16, Table 70 |
| `extended_position_report_b` | 19 | `ExtendedPositionReportB` | §3.17, Table 71 |
| `aid_to_navigation_report` | 21 | `AidToNavigationReport`, `AtonType` | §3.19, Tables 73 and 74 |
| `static_data_b` | 24 | `StaticDataB24A`, `StaticDataB24B` | §3.22, Tables 78 and 79 |

Any other message type becomes `AisMessageBody::Other { msg_type,
raw_payload, total_bits }`, so a caller can decode it with `BitReader`.

### Field state

Every field the wire can put in a non-value state is a `FieldState<T>`
(ADR-0008). Each numeric field has one code-space partition, applied
in this order: the not-available code is `NotAvailable`; the
over-range code, where the field has one, is `AtLeast(bound)` with the
bound in engineering units (SOG 1022 → 102.2 kn, Type 9 SOG 1022 kn
and altitude 4094 m, dimensions 511 m and 63 m, draught 255 → 25.5 m);
a code inside the defined range is a `Value`; any other code inside
the field's width is `Invalid(Undefined(RawCode))` with the wire
integer as the raw code (latitude and longitude in 1/10 000 minute).
`SenderError` and `Invalid::Unparsable` are unreachable in AIS. A raw
code never appears outside the invalid state, and the wire codes are
not public: the state a field is in is on the field.

Enum fields follow the same rule: `NavStatus`, `ManeuverIndicator` and
`EpfdType` lost their not-available and reserved members, which are
the `NotAvailable` and invalid states on the field. Strings are
`FieldState<String>`, `NotAvailable` when all padding. `ship_type` is
`FieldState<u8>` with 0 as not available and every other code a value;
the crate claims no knowledge of Table 53.

A status-carrying field is a `FieldState` of its own enum, so a status
is a value: timestamp 61–63 is `Timestamp::PositioningStatus` and rate
of turn ±127 ("turning at more than 5° per 30 s, no turn indicator")
is `RateOfTurn::NoIndicator`; only 60 and −128 are `NotAvailable`.

Plain fields keep their bare types: every one-bit flag, `mmsi`,
`radio_status`, `aton_status`, `AisVersion`, `AtonType`,
`AltitudeSensor` and `Type24BExtent::MothershipMmsi`.

A field whose bits lie wholly or partly past `total_bits` is
`NotAvailable`: a decoder checks the bit range before trusting an
ADR-0007 saturated read, so a short payload never yields a fabricated
zero. Floors follow traffic seen in the wild: Type 5 accepts 420 and
422 bits, reading the destination at character granularity and `dte`
as `FieldState<bool>`; Types 9 and 19 keep `dte: bool` because their
floors cover the bit. Types 1/2/3 stay at 168 bits, the one departure
from gpsd, so `radio_status` never lies partly past the payload.

See [ADR-0008](adr/0008-decoded-fields-carry-a-field-state.md).

### Type 24 Part B extent

The 30-bit field after the call sign holds ship dimensions, except on
an auxiliary craft (MMSI `98MIDxxxx`), where it holds the mother ship's
MMSI. `StaticDataB24B.extent` is a `Type24BExtent` enum chosen by
`is_auxiliary_craft_mmsi`. See
[ADR-0002](adr/0002-type24b-extent-by-mmsi-prefix.md).

Parts A and B are separate messages. The crate does not pair them; a
consumer pairs by MMSI.

### Reassembly

`AisReassembler` keys each open partial on `(channel, sequential_id)`
for every lookup, so the same sequential id can be live on channels A
and B at once. A single-fragment sentence returns immediately and
touches no state.

- Fragments must arrive in order. A fragment that is out of order, or a
  continuation with no partial on its key, yields
  `AisError::ReassemblyOutOfOrder` and drops the partial.
- At most `DEFAULT_MAX_PARTIALS` (16) partials are open. One more
  evicts the oldest and queues `AisError::ReassemblyTimeout`.
- Expiry by age is opt-in. `with_timeout_ms` sets the limit. The crate
  never reads a clock, so the caller advances it with `tick(now_ms)`
  (also on `AisFragmentParser`); the reassembler keeps that
  time and stamps each fragment with it (ADR-0004). A partial opened
  before the first tick has no stamp and can only be evicted by the
  slot cap.

### Python binding class policy

`marlin.ais` exposes one binding class per message struct (Types 1, 2,
and 3 share `PositionReportA`) plus a `type_tag` string on `AisMessage`
that names the variant. Coded enums (`NavStatus`, `EpfdType`, `AtonType`,
`PositioningStatus`) are int-backed binding classes, and the dataclass
mirrors in `marlin.dataclasses` store them as `int`.

A field state is `marlin.field.FieldState`, a frozen class with five
nested variant classes (`FieldState.Value(value)`,
`FieldState.AtLeast(bound)`, `FieldState.NotAvailable()`,
`FieldState.SenderError(code)`, `FieldState.Invalid(code)`) matched by
`isinstance` or `match`. The base class carries `kind` (the Rust
`Kind::name()` string, so both languages share one table), `value` and
`value_or_bound`. It has no `__bool__`: every state is truthy.

A Rust enum that carries data in a field position is a sum type of the
same shape, one variant class per Rust variant nested under the type's
class:

| Rust field | Python attribute |
| --- | --- |
| `rate_of_turn: FieldState<RateOfTurn>` | `rate_of_turn: FieldState[RateOfTurn]`, `RateOfTurn.DegPerMin` / `RateOfTurn.NoIndicator` |
| `timestamp: FieldState<Timestamp>` | `timestamp: FieldState[Timestamp]`, `Timestamp.Second` / `Timestamp.PositioningStatus` |
| `extent: Type24BExtent` | `extent: Type24BExtent`, `Type24BExtent.Dimensions` / `Type24BExtent.MothershipMmsi` |

Getters return `FieldState[T]` only. Constructors and `St0601` setters
accept `FieldState[T] | T | None`, coercing a bare value to `Value` and
`None` to `NotAvailable()`; the stubs type the two sides asymmetrically.
The constructors do not validate hand-built instances beyond the
payload's Rust type. The mirrors carry a field as one of five generic
frozen dataclasses with a `kind` literal, and a sum type's variants as
dataclasses nested under a namespace class of the type's name;
`to_dataclass` finds a mirror by the binding class's qualified name.
See
[ADR-0009](adr/0009-python-bindings-carry-sum-types-as-variant-classes.md).
