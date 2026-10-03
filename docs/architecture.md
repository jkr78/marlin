# Architecture

## marlin-klv

Standalone `#![no_std]` + `alloc` leaf crate: a MISB ST 0601 (UAS
Datalink Local Set) KLV encoder/decoder. Unlike `marlin-nmea-0183` and
`marlin-ais`, it does not depend on `marlin-nmea-envelope`: KLV is not
NMEA-framed, so there's no shared envelope layer to sit on.

### Modules

- `ber` — BER length codec (short and long form) and big-endian
  fixed-width readers.
- `scale` — MISB ST 0601 legacy linear int↔f64 scaling (not ST 1201
  IMAPB; those are newer tags, out of scope today).
- `tags` — macro-driven registry of the 20 scaled tags. Each
  `scaled_tags!` entry expands to that tag's encode arm, decode-dispatch
  arm, and getter/setter accessor pair.
- `checksum` — BCC-16 (Tag 1) running-sum checksum.
- `error` — `Error` (non_exhaustive, `Clone`): truncated input, BER
  length overflow, checksum mismatch, wrong local-set key.
- `st0601` — the encode/decode orchestrator. Owns `St0601`,
  `UAS_LS_KEY`, `encode`, `decode`, `precision_timestamp`, and (behind
  the `bytes` feature) `encode_to_bytes`. Composes `ber` + `scale` +
  `tags` + `checksum`.

### Data model

`St0601` stores raw wire integers per tag (`Option<u16>`,
`Option<i32>`, and so on), not engineering units. Each scaled tag gets
an accessor pair (e.g. `sensor_latitude_degrees()` /
`set_sensor_latitude_degrees()`) that converts to and from degrees or
meters. Getters return `None` when the tag is absent, or when the wire
value is the ST 0601 sentinel (`i16::MIN` / `i32::MIN`). Setters clamp
the input to the tag's valid range before conversion, so encoding can
never emit a sentinel value.

Unrecognized tags round-trip verbatim through
`St0601::unknown: Vec<(u8, Vec<u8>)>`, preserved in wire order.

### Invariants

- **Byte-exact round-trip.** `decode(encode(set)) == set` for this
  crate's own output. `encode` reproduces the source bytes for any
  packet it can decode, including a known tag with an unexpected wire
  length (falls back to `unknown` rather than erroring).
- **Tolerant decode.** A malformed known tag doesn't fail the whole
  decode; it lands in `unknown` instead. Tag 2 (precision timestamp) is
  the one exception — mandatory, so a malformed Tag 2 fails the whole
  decode.
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
  boolean and 6-bit-ASCII string fields (ITU-R M.1371-5 Annex 8 §3,
  Table 47). A read past the end returns zeros and never panics.
- `reassembly` — `AisReassembler` joins multi-sentence payloads.
- `message` — `AisMessage { is_own_ship, body }`, the `AisMessageBody`
  enum, and the dispatcher: `decode_message` for a bit buffer, `decode`
  for a single-fragment `RawSentence`.
- `parser` — `AisFragmentParser<P>` runs the whole pipeline over any
  envelope `SentenceSource`. The `Parser` enum picks one-shot or
  streaming at runtime.
- `shared_types` — `Dimensions`, `EpfdType`, `is_auxiliary_craft_mmsi`,
  the public `sentinel` constants, and the crate-private readers for
  longitude, latitude, COG, heading and SOG that the position decoders
  share.
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

### Sentinel policy

AIS fields reserve wire codes for "not available" (SOG 1023, altitude
4095, longitude 181°) and for "this value or higher" (SOG 1022,
altitude 4094 m, dimensions 511 m and 63 m). The decoders map a
not-available code to `None` and pass an over-range code through as the
value it names. A decoded field never carries the raw code. The codes
are public constants in `marlin_ais::sentinel`, so a consumer that
needs to tell an over-range value from a measurement compares against
them.

Rate of turn is the exception. ±127 means "turning at more than 5° per
30 s, no turn indicator", which is a status and not a rate, so
`PositionReportA.rate_of_turn` is an `Option<RateOfTurn>` enum.

See [ADR-0001](adr/0001-sentinel-representation.md).

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
- Expiry by age is opt-in: `with_timeout_ms` sets the limit, and the
  caller supplies monotonic milliseconds through `feed_fragment_at`,
  `tick`, or `AisFragmentParser::next_message_at`. The crate never
  reads a clock.

### Python mirror policy

`marlin.ais` exposes one class per message struct (Types 1, 2 and 3
share `PositionReportA`) plus a `type_tag` string on `AisMessage` that
names the variant. Coded enums (`NavStatus`,
`EpfdType`, `AtonType`) are int-backed classes, and
`marlin.dataclasses` stores them as `int`.

A Rust enum that carries data in a field position is flattened into
sibling optional attributes:

| Rust field | Python attributes |
| --- | --- |
| `rate_of_turn: Option<RateOfTurn>` | `rate_of_turn`, `turn_direction` |
| `extent: Type24BExtent` | `dimensions`, `mothership_mmsi` |

Parser output sets at most one attribute of each pair. The constructors
do not validate hand-built instances. See
[ADR-0003](adr/0003-python-mirrors-flatten-sum-types.md).
