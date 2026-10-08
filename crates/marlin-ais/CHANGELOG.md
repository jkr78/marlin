# Changelog

All notable changes to `marlin-ais` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-10-08

### Added

- Re-exports of `marlin_nmea_envelope::{OneShot, Parser, Streaming}`,
  beside the existing `RawSentence` re-export, so a caller needs no
  direct dependency on the envelope crate to build a source.

- `marlin_ais::sentinel` — public wire codes for "not available" and
  "this value or higher" (position, SOG, COG, heading, ROT, altitude,
  dimensions, timestamp) so consumers stop hardcoding them.
- `is_auxiliary_craft_mmsi` — whether an MMSI is a `98MIDxxxx` auxiliary
  craft (gpsd / USCG convention, ADR-0002).
- `code()` on `NavStatus`, `ManeuverIndicator` and `EpfdType` returns the
  wire code the variant was decoded from.
- `Dimensions` implements `Default` (all fields `None`).
- `RateOfTurn` and `TurnDirection` (re-exported from the crate root):
  the decoded Type 1/2/3 rate-of-turn field.
- `Type24BExtent` (re-exported from the crate root): what the 30-bit
  extent field of a Type 24 Part B holds.
- `StaticDataB24B.epfd` — the EPFD type at bits 162–165, which the
  decoder used to drop.
- Type 9 (standard SAR aircraft position report) decodes to
  `SarAircraftPositionReport` via `AisMessageBody::Type9` instead of
  landing in `Other` (ITU-R M.1371-5 Annex 8 §3.7, Table 59). Altitude
  and speed over ground are whole-unit `Option<u16>` (4095 / 1023 not
  available → `None`; 4094 m and 1022 kn over-range kept, ADR-0001);
  `AltitudeSensor::{Gnss, Barometric}` names the sensor bit; DTE,
  assigned and RAIM flags and the 20-bit radio status are exposed.
  `decode_sar_aircraft_position_report`, `AltitudeSensor`,
  `SarAircraftPositionReport` and `SAR_AIRCRAFT_POSITION_REPORT_BITS`
  are re-exported from the crate root.
- Type 21 (aid-to-navigation report) decodes to `AidToNavigationReport`
  via `AisMessageBody::Type21` instead of landing in `Other` (ITU-R
  M.1371-5 Annex 8 §3.19, Tables 73 and 74). The crate's first
  variable-length decoder: the 20-character name joins the optional
  extension (`min(14, (total_bits − 272) / 6)` characters, alignment
  spare ignored) and is trimmed of trailing `@` / spaces into
  `name: Option<String>`; an embedded `@` is kept. Payloads over 360 bits
  are tolerated, under 272 are `PayloadTooShort`. `AtonType` names all
  32 Table 74 codes with `code()`, `is_fixed()` (5–19) and
  `is_floating()` (20–31); `dimensions` reuses `Dimensions` (all-`None`
  for virtual AtoN); the off-position, virtual, assigned and RAIM flags
  and the raw 8-bit `aton_status` are exposed.
  `decode_aid_to_navigation_report`, `AidToNavigationReport`, `AtonType`
  and `AID_TO_NAVIGATION_REPORT_BITS` are re-exported from the crate root.

### Fixed

- `AisReassembler` now looks up continuation fragments by
  `(channel, sequential_id)`, as the first fragment already was. Two
  multi-sentence messages sharing a sequential id on channels A and B
  used to collide: the second channel's continuation matched the first
  channel's partial, raised a channel mismatch, and both messages were
  lost. Both now reassemble. A continuation fragment with no partial on
  its own key is still `ReassemblyOutOfOrder`.
- The ITU citations in the rustdoc and in this changelog were wrong.
  The `§5.3.x` numbers pointed at Annex 2 transport clauses or at
  nothing; they now name the Annex 8 section and table (ITU-R
  M.1371-5). The 6-bit armor citation now names IEC 61162-1, which
  defines it.
- The crate docs and README described the typed decoders and
  reassembly as still to come. They now list the supported message
  types.

### Changed (BREAKING)

- The `Parser` source-mode enum is removed. `AisFragmentParser<P>`
  now covers that case by wrapping `marlin_nmea_envelope::Parser`,
  which implements `SentenceSource` (ADR-0005), and the whole wrapper
  API (`with_reassembler`, `reassembler()`, `inner()`, `tick`) is
  available in that form. Replacements: `Parser::one_shot()` →
  `AisFragmentParser::new(OneShot::new())`; `Parser::streaming()` →
  `AisFragmentParser::new(Streaming::new())`;
  `Parser::streaming_with_capacity(n)` →
  `AisFragmentParser::new(Streaming::with_capacity(n))`. For a source
  mode chosen at runtime, pass `Parser::one_shot()` or
  `Parser::streaming()` from the envelope crate as the source.

- `PositionReportA.rate_of_turn` is `Option<RateOfTurn>` instead of
  `Option<f32>`. Raw `0..=±126` decode to `RateOfTurn::DegPerMin(f32)` as
  before; raw `±127` now decode to `RateOfTurn::NoIndicator(TurnDirection)`
  ("turning right/left at more than 5° per 30 s, no turn indicator",
  ITU-R M.1371-5 Table 48) instead of a fabricated ±720 °/min. `None`
  still means the `−128` not-available sentinel (ADR-0001).
- `StaticDataB24B.dimensions: Dimensions` is replaced by
  `extent: Type24BExtent`. For an auxiliary-craft MMSI (`98MIDxxxx`,
  `is_auxiliary_craft_mmsi`) the 30 bits are the mother ship's MMSI and
  decode to `MothershipMmsi(u32)` verbatim; they used to be misread as
  dimensions (gpsd / USCG convention, ADR-0002). Every other MMSI decodes
  to `Dimensions(..)` exactly as before.
- `AisError::ReassemblyChannelMismatch` removed: with channel-keyed
  lookups the branch is unreachable. Match on `ReassemblyOutOfOrder`
  instead.
- `AisError::UnknownMessageType` removed: it was never emitted; unrouted
  message types surface as `AisMessageBody::Other`.
- `Type24Part` removed: it was never constructed. `decode_static_data_b`
  already names the part by variant, `StaticDataB::{PartA, PartB,
  Reserved}`, and `Reserved { part_code }` carries the raw code.
- The reassembly clock is reassembler state (ADR-0004).
  `AisReassembler::tick(now_ms)` now also stores `now_ms`, and
  `feed_fragment` stamps the partial it touches with the last ticked
  time; `feed_fragment_at` is removed. `AisFragmentParser` and `Parser`
  gain `tick(now_ms)` and lose `next_message_at`: replace
  `p.next_message_at(t)` with `p.tick(t); p.next_message()`. A partial
  opened before the first tick has no stamp and is retired only by the
  slot cap, as unstamped partials were before.

## [0.1.4] - 2026-07-07

No behavioral changes. Lockstep version bump with the workspace release that
adds HDG/TTM/TLL sentence decoders to `marlin-nmea-0183`.

## [0.1.3] - 2026-07-07

No behavioral changes. Lockstep version bump with the workspace release that
extends `marlin-klv` with a tag-registry inspection API. Dropped the stale
hardcoded `html_root_url` doc attribute (docs.rs sets the root automatically).

## [0.1.2] - 2026-07-06

No behavioral changes. The version bump tracks the workspace release that
adds the new `marlin-klv` crate (MISB ST 0601 KLV encoder/decoder).

## [0.1.1] - 2026-05-08

### Fixed

- **Type 24 Part A decoder rejected every spec-canonical 160-bit
  payload** with `AisError::PayloadTooShort`. The previous floor of
  168 bits was Part B's size; ITU-R M.1371-5 Annex 8 §3.22, Table 78
  specifies Part A as 6 + 2 + 30 + 2 + 120 = 160 bits exactly. Real-world transmitters
  emit Part A at 160 bits with no padding; v0.1.0 silently dropped all
  such frames. Reported via the Python bindings against a 161-sentence
  batch from a public AIS feed.

### Changed (BREAKING)

- `STATIC_DATA_B_BITS` (the wrong-for-Part-A constant) is replaced by
  two part-specific constants: `STATIC_DATA_B_24A_BITS = 160` and
  `STATIC_DATA_B_24B_BITS = 168`. The dispatcher and per-part decoders
  now use the correct minimum for each part. Callers that referenced
  the public re-export at `marlin_ais::STATIC_DATA_B_BITS` must switch
  to one of the two new names.
- `AisError::PayloadTooShort` error message is now "payload too short
  for the chosen decoder" (was "fill-bits count exceeds payload size",
  which only described the armor-decoder case and was misleading when
  the variant surfaced from a per-type decoder).

## [0.1.0] - 2026-04-28

### Added

- Initial release of `marlin-ais` — typed sans-I/O decoders for AIS
  (AIVDM/AIVDO) messages, built on `marlin-nmea-envelope`
- `AisError` (non-exhaustive, `thiserror`-derived) covers envelope,
  armor, wrapper, and reassembly failure modes
- 6-bit ASCII armor codec per IEC 61162-1
  (`armor::decode`, `armor::decode_char`)
- `BitReader<'a>` with width-aware unsigned, two's-complement signed,
  boolean, and AIS-Table-47 string readers; past-end reads yield
  saturating zeros (panic-free contract)
- `AivdmHeader` and `parse_aivdm_wrapper` — fragment count, sequential
  id, channel, payload, fill bits; `is_own_ship` distinguishes `!AIVDM`
  from `!AIVDO`
- Typed decoders for the v0.1 message set:
  - Type 1/2/3: `PositionReportA` with `NavStatus`, `ManeuverIndicator`,
    sign-preserved ROT, and lat/lon/COG/heading sentinels surfaced as
    `None`
  - Type 5: `StaticAndVoyageA` with `AisVersion`, `Eta`, and per-sub-field
    sentinels (424-bit payload)
  - Type 18: `PositionReportB` with Class B capability flags
  - Type 19: `ExtendedPositionReportB`, the Class B extended position
    report with the static tail (name, ship type, dimensions, EPFD).
    312 bits, ITU-R M.1371-5 Annex 8 §3.17, Table 71
  - Type 24A / 24B: `StaticDataB24A`, `StaticDataB24B`, and a
    `decode_static_data_b` dispatcher that routes on the part-number field
- Shared `Dimensions`, `EpfdType`, and `trim_ais_string` helpers
- `AisMessage` wrapper carrying `is_own_ship` plus an `AisMessageBody`
  enum. Top-level `decode_message(bits, total_bits, is_own_ship)`
  primitive and `decode(&RawSentence)` single-fragment convenience.
  Unrouted types surface as `Other { msg_type, raw_payload, total_bits }`
- `AisReassembler` — per-channel per-sequential-id buffers,
  in-order enforcement, channel-mismatch detection, bounded-slot eviction
  (`DEFAULT_MAX_PARTIALS = 16`), and optional clock-based TTL via
  `with_timeout_ms` / `feed_fragment_at` / `tick(now_ms)`. The caller
  owns the clock so the crate stays sans-I/O and `no_std`. A
  `VecDeque<AisError>` pending queue surfaces one `ReassemblyTimeout` per
  evicted fragment when several expire at once
- `AisFragmentParser<P>` generic wrapper plus a runtime-dispatch `Parser`
  enum. Composes envelope → `parse_aivdm_wrapper` → `AisReassembler` →
  `armor::decode` → `decode_message` into one `feed` / `next_message`
  loop; `next_message_at(now_ms)` drives the reassembler clock for
  time-based expiry
- `cargo-fuzz` targets: `ais_armor`, `ais_bit_reader`, `ais_parser`.
  15-second smoke runs reached 9 M / 1.5 M / 1.25 M executions with zero
  panics. `just fuzz-smoke-all` and `just fuzz-release` wrap the set
