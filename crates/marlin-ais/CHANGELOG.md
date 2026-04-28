# Changelog

All notable changes to `marlin-ais` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Initial release of `marlin-ais` — typed sans-I/O decoders for AIS
  (AIVDM/AIVDO) messages, built on `marlin-nmea-envelope`
- `AisError` (non-exhaustive, `thiserror`-derived) covers envelope,
  armor, wrapper, and reassembly failure modes
- 6-bit ASCII armor codec per ITU-R M.1371-5 §8.2.4
  (`armor::decode`, `armor::decode_char`)
- `BitReader<'a>` with width-aware unsigned, two's-complement signed,
  boolean, and AIS-Table-47 string readers; past-end reads yield
  saturating zeros (panic-free contract, PRD §T5)
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
    312 bits, ITU-R M.1371-5 §5.3.19
  - Type 24A / 24B: `StaticDataB24A`, `StaticDataB24B`, and a
    `decode_static_data_b` dispatcher that routes on the part-number field
- Shared `Dimensions`, `EpfdType`, and `trim_ais_string` helpers
- `AisMessage` wrapper carrying `is_own_ship` plus an `AisMessageBody`
  enum. Top-level `decode_message(bits, total_bits, is_own_ship)`
  primitive and `decode(&RawSentence)` single-fragment convenience.
  Unrouted types surface as `Other { msg_type, raw_payload, total_bits }`
- `AisReassembler` (PRD §A5) — per-channel per-sequential-id buffers,
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
