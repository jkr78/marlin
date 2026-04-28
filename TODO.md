# Marlin — TODO / Status

Living document tracking per-crate completeness, open questions, and
the pre-release checklist. Update as work progresses. Checkboxes
track deliverables (not conversational state).

---

## Crate status at a glance

| Crate | State | Tests |
| --- | --- | --- |
| `marlin-nmea-envelope` | ✅ **Feature-complete** | 52 unit · 9 golden · 4 doctest |
| `marlin-nmea-0183`     | ✅ **Feature-complete** | 78 unit · 4 doctest |
| `marlin-ais`           | ✅ **Feature-complete for v0.1** — all spec message types, reassembly with clock-based timeout, `AisFragmentParser` wrapper, fuzz coverage | 137 unit · 1 doctest |
| `marlin-py` (Python bindings) | ✅ **Feature-complete for v0.1** | 145 pytest |

**Rust workspace total: 285 tests pass, `just ci` clean. Python bindings: 145 pytest pass, mypy --strict clean (30 source files).**

---

## `marlin-nmea-envelope`

### Done

- [x] `SentenceSource` trait with GAT for zero-copy borrows
- [x] `OneShot` (datagram) + `Streaming` (TCP/serial) — one shared nom parser core
- [x] `Parser` enum for runtime dispatch without `Box<dyn>`
- [x] TAG block recognition (`\...*hh\`) with advisory checksum (PRD decision 7)
- [x] `$P…` proprietary detection → `talker = None`
- [x] `pub fn parse(&[u8])` convenience entry point
- [x] `#![no_std]` with `extern crate alloc`
- [x] `tracing` feature gate (off by default)
- [x] Full rustdoc, README
- [x] cargo-fuzz target (60 s smoke run: 3.97 M execs, 0 panics)
- [x] 7 golden-file fixtures under `tests/fixtures/`

### Remaining (non-blocking polish)

- [ ] Owned `RawSentence::to_owned()` → `RawSentence<'static>` (PRD §4.5 future work)
- [ ] `criterion` benchmark suite (PRD §P4; nice-to-have)
- [ ] Dedicated `no_std` compile-test CI job
- [ ] Optional `serde` feature behind a flag (PRD §D3; post-v1.0)
- [ ] `arbitrary` derive for `RawSentence` (helps structure-aware fuzzing of higher crates)

---

## `marlin-nmea-0183`

### Done

- [x] `Nmea0183Message` enum (non_exhaustive) with `Unknown(RawSentence)` variant
- [x] `DecodeError` (non_exhaustive, thiserror, per-field-index reporting)
- [x] **GGA** — fix quality, satellites, HDOP, altitude, geoid, DGPS fields
- [x] **VTG** — pre-2.3 + 2.3+ forms; `VtgMode` with every recognized indicator
- [x] **HDT** — true heading
- [x] **PSXN** — install-configured 6-slot layout (`PsxnLayout`), `PsxnSlot` variants including TSS sine-encoded roll/pitch, `FromStr` for legacy `"rphx1"` strings
- [x] **PRDID** — two dialect structs (`PitchRollHeading`, `RollPitchHeading`) + `PrdidDialect::Unknown` strict default emitting `PrdidData::Raw`
- [x] `UtcTime` with ms resolution
- [x] Coordinate decode (ddmm.mmmm + hemisphere → signed decimal degrees)
- [x] `DecodeOptions` with `with_psxn_layout` / `with_prdid_dialect` builders
- [x] `decode` (default options) + `decode_with` (explicit options)
- [x] `Nmea0183Parser<P>` generic wrapper + `Parser` enum
- [x] `Nmea0183Error` unified error
- [x] Per-sentence `decode_*` functions public (extension points)
- [x] Full rustdoc, README

### Remaining (non-blocking)

- [ ] Additional NMEA sentences: RMC, GLL, GSA, GSV, ZDA, DBT, MWV (PRD §11)
- [ ] Golden-file fixtures from real receivers (synthetic today)
- [ ] Example program under `examples/` showing log-file replay

---

## `marlin-ais`

### Done

- [x] `AisError` (non_exhaustive, thiserror) — variants for all upcoming milestones; only envelope/armor/wrapper variants actively emitted
- [x] `armor::decode` + `armor::decode_char` — ASCII-to-6-bit alphabet per ITU-R M.1371-5 §8.2.4
- [x] `BitReader<'a>` — `u(n)`, `i(n)` (two's-complement at field width), `b()`, `string(chars)` (AIS Table 47), `remaining()`
- [x] Past-end reads yield saturating zeros (panic-free contract, PRD §T5)
- [x] `AivdmHeader` + `parse_aivdm_wrapper` — fragment count, sequential id, channel, payload, fill bits; `is_own_ship` distinguishes `!AIVDM` from `!AIVDO`
- [x] **Type 1/2/3** — `PositionReportA` + `NavStatus` + `ManeuverIndicator` + ROT sign-preserved decode + lat/lon/COG/heading sentinels → `None`
- [x] **Type 5** — `StaticAndVoyageA` + `AisVersion` + `Eta` + per-sub-field sentinels, 424-bit payload
- [x] **Type 18** — `PositionReportB` with Class B capability flags
- [x] **Type 19** — `ExtendedPositionReportB` (Class B extended position report with static tail: name, ship type, dimensions, EPFD). 312 bits, ITU-R M.1371-5 §5.3.19
- [x] **Type 24A / 24B** — `StaticDataB24A` + `StaticDataB24B` + `decode_static_data_b` dispatcher (routes on part-number field)
- [x] Shared `Dimensions` + `EpfdType` + `trim_ais_string` helpers
- [x] **`AisMessage` wrapper + `AisMessageBody` enum + top-level `decode_message` / `decode`** — `AisMessage { is_own_ship, body }` (PRD §A7 wrapper-struct shape); bit-level `decode_message(bits, total_bits, is_own_ship)` primitive; `decode(&RawSentence)` single-fragment convenience; routes Type 1/2/3/5/18/19/24A/24B to typed variants, everything else (reserved Type 24 parts and unknown msg_type values) to `Other { msg_type, raw_payload, total_bits }`
- [x] **Multi-sentence reassembly** (`AisReassembler`, PRD §A5) — per-channel per-sequential-id fragment buffers; in-order enforcement; channel-mismatch detection; bounded-slots eviction (`DEFAULT_MAX_PARTIALS = 16`) plus optional clock-based TTL via `with_timeout_ms`/`feed_fragment_at`/`tick(now_ms)` (caller owns the clock — keeps sans-I/O + `no_std`); `VecDeque<AisError>` pending-queue so multiple simultaneous evictions each surface one `ReassemblyTimeout`
- [x] **`AisFragmentParser<P>` generic wrapper + `Parser` enum** — mirrors `Nmea0183Parser` pattern; composes envelope → `parse_aivdm_wrapper` → `AisReassembler` → `armor::decode` → `decode_message` into a single `feed`/`next_message` loop; surfaces reassembly timeouts between fragments. `next_message_at(now_ms)` variant drives the reassembler clock for time-based expiry
- [x] **cargo-fuzz targets** (PRD §F1) — `ais_armor`, `ais_bit_reader`, `ais_parser`. 15 s smoke runs each: 9 M / 1.5 M / 1.25 M executions, zero panics. `just fuzz-smoke-all` and `just fuzz-release` wrap up the set

### Remaining (non-blocking)

- [ ] Golden-file fixtures from real AIS feeds (aishub / marinetraffic public samples)
- [ ] Example program decoding an AIVDM log

---

## Open API design questions

### Resolved

- [x] **Proprietary sentence address parsing (`$PSXN`, `$PRDID`).** `RawSentence::talker: Option<[u8; 2]>`; for `$P…`, `talker = None`, `sentence_type` carries the full address including `P`.
- [x] **PSXN — install-configured layout, not fake subtypes.** `PsxnLayout` + `PsxnSlot` with TSS sine-encoded variants; `FromStr` for legacy Python config strings.
- [x] **PRDID — two dialects + strict-default.** `PrdidDialect::Unknown` (default) emits `PrdidData::Raw`.

### Open

- [ ] **Checksum enforcement policy.**
  Today's parser is strict. Legacy devices emit `*00` as a "checksum disabled" sentinel or omit the checksum entirely. Options:
  - **A**. Stay strict by default; add a `Parser::lax()` constructor that accepts `*00` as "unverified" (`checksum_ok = false`).
  - **B**. Accept `*00` specifically as a disable sentinel.
  - **C**. Status quo: strict, reject all malformed.
  `RawSentence::checksum_ok` already exists to support (A) non-breaking. Not urgent — matters at TCMS integration.

- [ ] **`Error::Truncated` activation.**
  Variant defined but never emitted. Current behavior: partial `OneShot` buffers return `None`. For explicit "datagram truncated" signalling, add `flush()` on `OneShot`. Low priority; UDP callers handle by timeout today.

---

## Documentation / infra

- [x] Workspace `README.md`
- [x] Per-crate `README.md` (envelope, nmea-0183, ais)
- [x] `justfile` for common recipes
- [x] GitHub Actions CI (build + test + clippy + fmt + doc + MSRV 1.82 + 30 s fuzz smoke)
- [x] Per-crate `CHANGELOG.md` for the Rust crates (envelope, nmea-0183, ais), starting at 0.1.0 (PRD §10.2). The Python binding's CHANGELOG already lives at `bindings/python/CHANGELOG.md`.
- [x] `docs.rs` metadata (`package.metadata.docs.rs`) for feature-aware docs at publish time

---

## `marlin-py` (Python bindings)

### Done

#### Core surface

- [x] Scaffold: PyO3 0.27 + maturin, `cdylib` named `_core`
- [x] Error hierarchy: `MarlinError` → `{Envelope,Decode,Ais,Reassembly}Error`
- [x] Envelope: `RawSentence`, `OneShotParser`, `StreamingParser`, `parse()`
- [x] NMEA typed: `Nmea0183Parser`, `Gga/Vtg/Hdt/Psxn/Prdid/Unknown`,
      enums, `DecodeOptions`, per-sentence extension-point functions
- [x] AIS typed: `AisParser` (three clock modes), `AisMessage`, all
      message variants, `BitReader`

#### Ergonomics

- [x] Context manager support (`__enter__` / `__exit__`) on every parser
      (`with OneShotParser() as p:`, `with StreamingParser() as p:`,
      `with Nmea0183Parser.streaming() as p:`, `with AisParser.streaming() as p:`)
- [x] Async iterator helpers in `marlin.aio`: `aiter_sentences`,
      `aiter_nmea_messages`, `aiter_ais_messages` for `asyncio.StreamReader`
- [x] `@dataclass`-style frozen mirrors in `marlin.dataclasses` with
      `to_dataclass(msg)` dispatcher — JSON / msgspec / dataclasses-asdict
      friendly. Covers all typed runtime classes: envelope RawSentence, NMEA
      Gga/Vtg/Hdt/Psxn/Prdid/Unknown, AIS message variants, and AisMessage
      wrapper. Enums serialize as integer values.

#### Quality + tooling

- [x] `.pyi` stubs, `py.typed`, mypy --strict CI gate
- [x] pytest unit + golden round-trip + hypothesis panic-freedom
- [x] CI: wheels for Linux x86_64/aarch64, macOS universal2, Windows x86_64

#### Documentation + examples

- [x] Six example programs (PRD §10 deliverable 7 + stdin reader + live AIS dashboard)
- [x] `bindings/python/GUIDE.md` — usage guide covering streaming, asyncio
      integration, per-protocol filtering, context managers, and dataclass
      serialization
- [x] `bindings/python/CHANGELOG.md` following Keep-a-Changelog format

### Deferred (post-v0.1)

- [ ] PyPI publish (name reservation, release workflow)
- [ ] JSON / msgspec helper submodule
- [ ] Structure-aware fuzzing integration (once Rust crates gain
      `arbitrary::Arbitrary` derives)

---

## Pre-release checklist (v0.1.0 → published)

Must complete before publishing to crates.io:

- [ ] One CPU-hour fuzz run on each of `envelope`, `ais_armor`, `ais_bit_reader`, `ais_parser`, zero findings (PRD §F2). Run: `just fuzz-release`. **Pending — gates the v0.1.0 tag push.**
- [ ] Replace synthetic fixtures with ≥ 5 real captures per sentence / message type (PRD §G3). Document source of each in fixtures README
- [ ] **Validate PSXN decoder against real captures.** Implementation matches legacy Python semantics but hasn't been cross-checked against a live Seapath/MRU feed. Candidates: `$PSXN,10,...` with known roll/pitch/heave; `$PSXN,11,...` quality-0 variant; any TCMS source using an `sqh` layout
- [ ] **Validate PRDID dialects against real captures.** Dialect orderings come from public integration guides and TSS/Teledyne convention reading. Need live samples from each hardware type
- [ ] Validate AIS decoders against real AIVDM captures — specifically lat/lon sign handling (PRD §A4) and 27/28-bit signed coordinate edge cases
- [x] MSRV double-check: whole workspace compiles on 1.82 (verified 2026-04-28)
- [x] Curate fuzz corpus down to a small regression suite (PRD §F3); commit it. Lives at `fuzz/seeds/<target>/`; auto-bootstrapped on every `just fuzz`
- [ ] Resolve remaining open API questions (checksum policy, `Error::Truncated`) so we don't ship with breaking changes imminent
- [ ] Reserve final crate names on crates.io (currently `marlin-*`)
- [x] Tag `v0.1.0`, draft release notes from commit history. **Tag created locally, not pushed; awaiting fuzz-release pass.**

---

## Out of scope for v0.1.0

Tracked here so we don't forget:

- AIS message types beyond 1/2/3/5/18/19/24A/24B (PRD §11)
- AIS Type 24 part-A/part-B pairing (higher-layer concern; PRD §A6)
- Additional NMEA sentences (RMC, GLL, GSA, GSV, ZDA, DBT, MWV, etc. — PRD §11)
- NMEA 2000 (entirely separate protocol; would be a new crate)
- Encoding / serialization (v0.1 is read-only)
- WASM target verification
- `no_std` embedded profile verification on bare-metal target
