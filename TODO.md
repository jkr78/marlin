# Marlin — TODO / Status

Living document tracking per-crate completeness, open questions, and
the pre-release checklist. Update as work progresses. Checkboxes
track deliverables (not conversational state).

---

## New tasks

- [ ] Report slot-full eviction under its own error, not ReassemblyTimeout
  `AisReassembler::open_partial` queues `AisError::ReassemblyTimeout` when
  the slot cap evicts the oldest partial, although no time passed. The two
  evictions are told apart only by whether `with_timeout_ms` was set. A
  separate variant (or one `ReassemblyEvicted { reason }`) would name the
  cause; `AisError` is `non_exhaustive`, so adding one is not breaking for
  matchers, but the Python `ReassemblyError` message text would change.
  Found while grilling the reassembly clock change. [ais][draft]
- [ ] Settle what "wrapper" means in the Python bindings
  The word appears 14 times in `bindings/python/src/` with two meanings. The
  four module headers ("Python wrappers for `marlin-ais`", and the same in
  `nmea.rs`, `envelope.rs`, `klv.rs`) mean binding classes, and `GLOSSARY.md`
  lists "wrapper class" as a term to avoid for that. Elsewhere "the
  `AisMessage` wrapper" means the outer message that holds a body; the
  `marlin.dataclasses` docstring and `tests/unit/test_dataclasses.py` use it
  that way too, and the glossary has no term for it. Decide the term for the
  outer message first (keep "wrapper" with a glossary entry, or pick another),
  then reword. Only the four module headers are clear misuses today, so this
  may need no more than those four lines. Found by the py-field-once ticket 03
  review. [docs][draft]
- [ ] Generate each binding class from one field list
  A field of a binding class is written 5 times in `bindings/python/src/ais.rs`
  and `src/nmea.rs`: the `#[pyclass]` struct, the `#[pyo3(signature = ...)]`
  list, the `#[new]` parameters, the `Self { ... }` body, and the `From<Rust…>`
  impl. One `macro_rules!` invocation listing each field once can generate all
  of it; tried 2026-10-03 on `SarAircraftPositionReport`, clean under the
  crate's clippy lints, same behaviour on Python 3.9 and 3.13. Defaults must be
  captured as `$($default:tt)::+`, not `$default:expr`, or the Python text
  signature shows `...`; a default like `-1` needs another form. Untried on the
  classes ADR-0003 flattens (`PositionReportA`, `StaticDataB24B`). May not be
  worth doing: the compiler already catches a field missing from the struct
  literal or the `From` impl, so this saves typing, not drift. Revisit when a
  batch of new message types arrives. Split from the card below. [py][draft]
- [ ] Give the parser iterator classes one name in stubs and runtime
  The public stubs declare `_AisIterator`, `_NmeaIterator`, and
  `_EnvelopeIterator` as what `__iter__` returns. At runtime the classes are
  `marlin.ais.PyAisIterator`, `marlin.nmea.PyNmeaIterator`, and
  `marlin.envelope.PyEnvelopeIterator`: the `#[pyclass]` attributes in
  `bindings/python/src/` set no Python name, and the package modules do not
  re-export them. `mypy.stubtest` does not report this, because the stub names
  are private and absent at runtime. May not need fixing: users iterate these
  objects and never name them, so nothing breaks today. A fix picks the
  Python-visible name in `#[pyclass(name = ...)]`, matches the stubs to it, and
  rebuilds the extension. Found by the py-field-once ticket 01 review. [py][draft]
- [x] Write each Python message field once per layer **DONE 2026-10-03**
  Candidate 1 of the 2026-10-03 architecture review; spec and tickets in
  `.scratch/py-field-once/`. A field was written about 11 times across three
  layers. What landed: the public stubs are checked against the runtime by
  `mypy.stubtest` (ticket 01, card below); `marlin.dataclasses` converts with
  one generic converter that pairs a binding class with the dataclass mirror
  of the same name, guarded by
  `bindings/python/tests/unit/test_dataclass_agreement.py`, so adding a message
  there is one dataclass and one `__all__` entry (ticket 02); docs and comments
  say "binding class" and "dataclass mirror" (ticket 03). Not done: the binding
  class still writes each field 5 times (card above), and the stubs stay
  hand-written because no stub generator fits (PyO3's is experimental and
  rejects function-declared modules, `pyo3-stub-gen` needs Python 3.10 against
  our `abi3-py39`). [py]
- [x] Gate the Python stubs with mypy.stubtest **DONE 2026-10-03**
  `just py-type-check` and the CI type-check job run
  `python -m mypy.stubtest marlin --ignore-missing-stub` with
  `bindings/python/stubtest-allowlist.txt`. Of the 475 findings (Python 3.13,
  mypy 1.20.2), 346 went by declaring constructors as `__new__`, 55 by marking
  every binding class `@final` (which also cleared the 55 `@disjoint_base`
  findings), and 18 by making `__eq__` positional-only. Removing the `__init__`
  noise exposed one real disagreement: the stubs for `Gga`, `Vtg`, `Hdt`, and
  `Unknown` marked constructor parameters keyword-only where the runtime takes
  them positionally; the stubs now follow the runtime. The allowlist has two
  entries: `marlin.ais.ClockMode` (a two-value `Literal` alias reported as "is
  not a Union") and, on Python 3.9 only, `marlin.aio.AsyncIterator`. The second
  forces `--ignore-unused-allowlist`, so a stale entry is not reported. Ticket:
  `.scratch/py-field-once/issues/01`. [py]
- [ ] Give the supported sentence and message lists one source
  The NMEA sentence list (GGA, GLL, HDG, HDT, RMC, TLL, TTM, VTG, PSXN, PRDID)
  is written out by hand in about twelve places and the AIS type list in about
  six: root, crate, and Python READMEs, `GUIDE.md`, crate-level rustdoc, Cargo
  descriptions, Python docstrings, this file. Ticket 07 existed because GLL,
  RMC, HDG, TTM, and TLL had drifted out of most of them. Pick one home per list
  (the crate README table is the likely one), have the others link to it or
  name only the count, and decide whether a CI check should compare the list
  against the `Nmea0183Message` / `AisMessageBody` variants. Needs a short
  design pass first: Cargo descriptions and docstrings cannot link. Raised by
  the ticket 07 Standards review. [docs][draft]
- [ ] Unify the marlin-ais test payload builders on one style
  `message.rs` routing tests mix named `build_typeN(mmsi)` helpers (Types 1/2/3,
  5, 18, 24) with inline `w.u(64, 0)` padding (Types 19 and 9).
  `build_min_payload(msg_type, mmsi, total_bits)` landed with ticket 06 and the
  Type 21 routing tests use it; the other eight builders still need converting
  to it. The decoder modules mix positional builders under `too_many_arguments`
  (`build_prb`, `build_pra`, `build_part_b`) with the `Fields` struct + `Default`
  + struct-update form that `sar_aircraft_position_report.rs` (Type 9) and
  `aid_to_navigation_report.rs` (Type 21) use. Keep the `Fields` form (each
  test names only what it varies, no allow needed) and convert the siblings
  when their tests are next touched. Raised by the ticket 05 Standards review.
  [ais][ready]
- [x] Make `just py-ci` self-sufficient **DONE 2026-10-04**
  `just py-ci` failed on a fresh checkout: `maturin` and `pyright` came from
  PATH and the virtualenv had neither. Every Python recipe now runs its tools
  as `{{py}} -m <tool>` from one virtualenv, `.venv` at the repo root by
  default (the one CI and `pyrightconfig.json` use), and `just py-setup`
  creates it and installs the `dev` extra, which now includes `maturin`.
  Another interpreter: `just py_venv=/abs/path py-setup py-ci`; run that way
  on Python 3.9 and 3.13. `bindings/python/.venv` (uv, no pip) is no longer
  used by any recipe. The CI workflow installs the same `dev` extra list,
  read from `pyproject.toml`; it still runs its steps inline, not through
  `just py-ci`.
- [ ] Simplify the three-step lookup in `AisReassembler::append_to_partial`
  `position` → `get(idx)` → `get_mut(idx)` with three identical
  `else { return Err(ReassemblyOutOfOrder) }` fallbacks exists only to dodge
  `clippy::indexing_slicing`. Review of 7ba309f suggested one `iter_mut()`
  `position`-plus-`remove` reshaping. Judgement call: the current form is clippy
  clean and the fallbacks are unreachable, so re-check whether the ceremony
  still bothers anyone before doing the work.
- [ ] Add a three-way sentinel reading across all AIS types (PRD §A3 revision)
  value / not available / invalid on every numeric field, with a typed timestamp
  (codes 60–63) folded in. Requested by nexus; deferred from the Type 9/21 feature
  because it changes every field type (breaking) and revises PRD §A3, which today
  says the raw code is not exposed. ADR-0001 records the current policy. Needs its
  own grill before any code.
- [ ] Parse NMEA 4.10 TAG blocks in marlin-nmea-envelope
  `RawSentence.tag_block` is raw bytes; consumers parse `c:` (Unix seconds)
  themselves. A typed TAG block (`c:`, `s:`, `d:`, `g:`, `n:`, `r:`, `t:`) removes
  that code. Independent of AIS; own feature.
- [ ] Decode AIS Types 4 and 27, and binary Types 6 and 8
  Still routed to `Other`. Types 4 and 27 did not appear in the 2026-09-22
  Kystverket capture; Types 6 and 8 (78 messages there) have no consumer use yet.
- [ ] Decode radar sentences OSD, RSD, TTD, TLB in marlin-nmea-0183
  OSD would let an ARPA consumer take own-ship data from the radar instead of
  configuration. Follow-up to the HDG/TTM/TLL decoders.
- [ ] Track ITU-R M.1371-6 changes: Message 28, Message 9 rename, Table 72
  M.1371-6 (02/2026) adds the single-slot AtoN report (Message 28), renames
  Message 9 to "Standard aircraft station in the maritime mobile service position
  report", and renumbers the AtoN type table to Table 72 (code 2 "RACON or
  MatoN"). The crate targets M.1371-5; revisit when a consumer needs -6 semantics.
- [ ] Run the ais_parser fuzz target in the CI fuzz-smoke job
  `.github/workflows/ci.yml` runs only `just fuzz envelope 30`; `just
  fuzz-smoke-all` covers the AIS targets locally only. Add `just fuzz ais_parser
  30` (nightly job already set up).
- [ ] Remove the never-constructed Type24Part enum
  `Type24Part` is defined in `static_data_b.rs` and re-exported from `lib.rs` but
  never constructed: dispatch uses `StaticDataB::{PartA, PartB, Reserved}`.
  Removal is breaking; the 0.2.0 window (Type 9/21 release) was declined on
  2026-10-02 in favour of this card, so it waits for the next breaking release.
- [x] Fix pre-existing marlin-py stub/export gaps **DONE 2026-07-07**
  Found during the radar-sentence review.
  Two unrelated pre-existing drifts surfaced while adding HDG/TTM/TLL:
  (1) `bindings/python/python/marlin/_core.pyi` `Nmea0183Parser.next_message`
  return-type union omits `Gll` and `Rmc` (present on `__next__` and the public
  alias, missing only here). (2) `bindings/python/python/marlin/dataclasses.py`
  `__all__` omits `Gll`, `Rmc`, and `UtcDate` (classes exist and are used but
  aren't exported). Both are latent-only (no runtime break today). Small fix.
- [ ] Add a present-field enumeration view to marlin-klv St0601
  klv-inspect G2.
  Ergonomic convenience only, no wire semantics: a `present()` / `items()`
  view yielding the tags a decoded set actually carries, so consumers stop
  reflecting over `raw_*` attribute names. Deferred — not codec territory and
  the suite has no precedent for enumerating a decoded value's fields (you read
  named fields, as with `AisMessage`). Do it for ergonomics if/when wanted,
  the same way the AIS parser helpers exist for ergonomics. Once the G1 tag
  registry landed, consumers can already enumerate against the authoritative
  table instead of a naming convention.
- [ ] Add a checksum-free KLV structural reader if faulty senders appear
  klv-inspect G3.
  Only if real streams arrive with absent/bad BCC or foreign framing that
  strict `decode` rejects: a clearly-named, separate reader (Rust first, then
  a Python binding) that walks UL + BER-TLV and yields raw `(tag, value)`
  items WITHOUT verifying the checksum — never a mode of `decode`. Tension:
  this relaxes the crate's strict-by-default contract, and every other
  iterator in the suite surfaces validation errors as `Err` and recovers
  rather than surrendering the guarantee. If built, first extract the shared
  TLV walk (see the decode/precision_timestamp dedup task) so this is not a
  fourth copy, and have it report the checksum outcome rather than silently
  dropping it. Hopefully unnecessary; bosun keeps a local fallback meanwhile.
- [x] Export marlin.klv tag registry + UAS_LS_KEY (klv-inspect G1 + G4) **DONE 2026-07-07**
  Codec-owned metadata, exposed so consumers stop re-deriving it. Rust:
  `tags()` / `tag_number()` / `tag_name()` + `TagInfo`, macro-generated from
  the `scaled_tags!` table (drift-free) plus Tag 2 and Tag 65. Python:
  same three functions + `TagInfo`, and the `UAS_LS_KEY` module constant.
- [ ] Let Python callers construct marlin-klv unknown tags
  The `St0601.unknown` PyO3 getter is read-only, so Python can decode and
  round-trip unknown tags but cannot build a set with unknown tags from
  scratch. Add a setter (or an `add_unknown(tag, value)` method) if a
  Python encode-side use case needs it.
- [ ] Expand marlin-klv ST 0601 tag coverage beyond the initial 20 scaled tags
  ST 0601 defines many more tags than the initial release covers. Add
  one struct field in `st0601.rs` plus one `scaled_tags!` entry in
  `tags.rs` per tag; verify each formula against the standard, don't
  pattern-match blind.
- [ ] Add ST 1201 IMAPB float scaling to marlin-klv
  `scale.rs` implements only ST 0601 legacy linear scaling today (its
  module doc says so explicitly: "NOT ST 1201 IMAPB — those are newer
  tags, out of scope"). Needed once IMAPB-encoded tags show up in real
  streams.
- [ ] Support additional MISB local sets (ST 0903 VMTI, ST 0102 security)
  marlin-klv implements only ST 0601 (UAS Datalink LS) today. VMTI
  (target tracks) and the 0102 security metadata LS are common
  companions in real KLV streams.
- [ ] Dedupe decode/precision_timestamp item-walk in marlin-klv
  `st0601.rs::decode` and `st0601.rs::precision_timestamp` both walk
  the TLV item list with near-identical loops. A shared helper is the
  obvious cleanup, but a walker over borrowed item slices can hit a
  borrow-checker lifetime trap when a second such method tries to
  delegate to it. Prototype the extraction before committing to it.
- [ ] Real-data KLV golden captures (>=5 samples) for marlin-klv
  Same gap as the NMEA/AIS fixtures: today's KLV test vectors are
  synthetic. Need >=5 real ST 0601 streams from actual UAS datalink
  hardware/software, source documented per sample.
- [x] Add a bindings/python Rust fmt/clippy CI gate **DONE 2026-07-06**
  `bindings/python` is workspace-excluded (root `Cargo.toml`:
  `exclude = ["fuzz", "bindings/python"]`), so `just ci` never runs
  `cargo fmt --check` / `cargo clippy -D warnings` against its Rust
  source. Needs its own recipe/CI step.
- [x] Gate the fuzz crate Rust with fmt + clippy **DONE 2026-07-06**
  Same workspace-exclusion gap as the bindings crate. Added
  `just fuzz-fmt-check` / `fuzz-lint` (wired into `just ci`) plus fmt +
  clippy steps in the `ci.yml` check job. Runs on stable — nightly is
  only needed to run the fuzzer, not to lint the harnesses.
- [x] Investigate pyright errors in test_context_managers.py, test_envelope.py **DONE 2026-07-07**
  Ran project pyright (`standard` mode): found 24 errors across three files.
  Root cause of the possibly-unbound findings: parser `__exit__` stubs typed
  `-> bool` (the Rust always returns `false`), so pyright assumed exceptions
  might be suppressed → with-body vars possibly-unbound. Fixed at the root by
  typing all 8 `__exit__` stubs `-> Literal[False]`. `test_envelope.py`
  optional-access narrowed with `assert s is not None`; the intentional
  frozen-property assignment got a scoped `# pyright: ignore`. Also caught +
  fixed 11 self-introduced pyright errors in `test_radar.py` (missing
  `isinstance(msg, Ttm)` narrowing — mypy passed but pyright standard flagged
  it). Pyright now 0 errors; mypy --strict + 181 pytest still green.

---

## Crate status at a glance

| Crate | State | Tests |
| --- | --- | --- |
| `marlin-nmea-envelope` | ✅ **Feature-complete** | 52 unit · 9 golden · 4 doctest |
| `marlin-nmea-0183`     | ✅ **Feature-complete** — GGA, GLL, HDG, HDT, RMC, TLL, TTM, VTG, PSXN, PRDID | 122 unit · 4 doctest |
| `marlin-ais`           | ✅ **Feature-complete for 0.2.0** — Types 1/2/3, 5, 9, 18, 19, 21, 24A/24B, reassembly keyed on `(channel, sequential_id)` with clock-based timeout, `AisFragmentParser` wrapper, fuzz coverage | 188 unit · 1 doctest |
| `marlin-klv`           | ✅ **Feature-complete for v0.1** | 48 unit · 1 doctest |
| `marlin-py` (Python bindings) | ✅ **Feature-complete for 0.2.0** | 206 pytest |

**Rust workspace total: 429 tests pass, `just ci` clean. Python bindings: 206 pytest pass, mypy --strict clean (35 source files). Counted 2026-10-03 from `just ci` output, default features.**

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
  Covers the wire types of every crate, including the `marlin-ais` structs (not
  only the envelope). nexus maps into its own model by hand and does not need it
  (noted 2026-10-02).
- [ ] `arbitrary` derive for `RawSentence` (helps structure-aware fuzzing of higher crates)

---

## `marlin-nmea-0183`

### Done

- [x] `Nmea0183Message` enum (non_exhaustive) with `Unknown(RawSentence)` variant
- [x] `DecodeError` (non_exhaustive, thiserror, per-field-index reporting)
- [x] **GGA** — fix quality, satellites, HDOP, altitude, geoid, DGPS fields
- [x] **VTG** — pre-2.3 + 2.3+ forms; `VtgMode` with every recognized indicator
- [x] **HDT** — true heading
- [x] **HDG** — magnetic heading, deviation and variation as signed degrees (`E` positive) (added v0.1.4 2026-07-07)
- [x] **TTM** — radar/ARPA tracked target, `AngleReference` / `DistanceUnits` / `AcquisitionType` / `TargetStatus` enums, optional NMEA 3.0 trailing fields (added v0.1.4 2026-07-07)
- [x] **TLL** — radar/ARPA target position with optional trailing fields (added v0.1.4 2026-07-07)
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

- [x] **RMC** — recommended minimum (UTC + date + position + speed + course + magnetic variation), pre-2.3 / 2.3+ / 4.10+ forms (added v0.1.1 2026-05-08)
- [x] **GLL** — geographic position with validity status and optional 2.3+ mode (added v0.1.1 2026-05-08)
- [ ] Additional NMEA sentences: GSA, GSV, ZDA, DBT, MWV (PRD §11)
- [ ] Golden-file fixtures from real receivers (synthetic today)
- [ ] Example program under `examples/` showing log-file replay

---

## `marlin-ais`

### Done

- [x] `AisError` (non_exhaustive, thiserror) — envelope/armor/wrapper/payload variants plus the two reassembly variants `ReassemblyOutOfOrder` and `ReassemblyTimeout`, all emitted. `UnknownMessageType` (never emitted; unrouted types go to `Other`) and `ReassemblyChannelMismatch` (unreachable once partials are keyed on channel) removed in 0.2.0
- [x] `armor::decode` + `armor::decode_char` — ASCII-to-6-bit alphabet per IEC 61162-1
- [x] `BitReader<'a>` — `u(n)`, `i(n)` (two's-complement at field width), `b()`, `string(chars)` (AIS Table 47), `remaining()`
- [x] Past-end reads yield saturating zeros (panic-free contract, PRD §T5)
- [x] `AivdmHeader` + `parse_aivdm_wrapper` — fragment count, sequential id, channel, payload, fill bits; `is_own_ship` distinguishes `!AIVDM` from `!AIVDO`
- [x] **Type 1/2/3** — `PositionReportA` + `NavStatus` + `ManeuverIndicator` + `rate_of_turn: Option<RateOfTurn>` (a measured rate, or `NoIndicator(TurnDirection)` for ±127; 0.2.0 fix: ±127 used to decode as ±720 °/min) + lat/lon/COG/heading sentinels → `None`
- [x] **Type 5** — `StaticAndVoyageA` + `AisVersion` + `Eta` + per-sub-field sentinels, 424-bit payload
- [x] **Type 9** — `SarAircraftPositionReport` + `AltitudeSensor`: altitude and SOG as whole-unit `Option<u16>`, 168 bits, ITU-R M.1371-5 Annex 8 §3.7, Table 59 (0.2.0)
- [x] **Type 18** — `PositionReportB` with Class B capability flags
- [x] **Type 19** — `ExtendedPositionReportB` (Class B extended position report with static tail: name, ship type, dimensions, EPFD). 312 bits, ITU-R M.1371-5 Annex 8 §3.17, Table 71
- [x] **Type 21** — `AidToNavigationReport` + `AtonType` (32 codes, Table 74): variable length 272–360 bits, name joined with its optional extension, ITU-R M.1371-5 Annex 8 §3.19, Table 73 (0.2.0)
- [x] **Type 24A / 24B** — `StaticDataB24A` + `StaticDataB24B` + `decode_static_data_b` dispatcher (routes on part-number field). Part B `extent: Type24BExtent` is dimensions, or the mother-ship MMSI for an auxiliary craft (`98MIDxxxx`, ADR-0002), and `epfd` is decoded (0.2.0)
- [x] Shared `Dimensions` + `EpfdType` + `trim_ais_string` helpers; public `sentinel` wire codes and `is_auxiliary_craft_mmsi` (0.2.0, ADR-0001)
- [x] **`AisMessage` wrapper + `AisMessageBody` enum + top-level `decode_message` / `decode`** — `AisMessage { is_own_ship, body }` (PRD §A7 wrapper-struct shape); bit-level `decode_message(bits, total_bits, is_own_ship)` primitive; `decode(&RawSentence)` single-fragment convenience; routes Type 1/2/3/5/9/18/19/21/24A/24B to typed variants, everything else (reserved Type 24 parts and unknown msg_type values) to `Other { msg_type, raw_payload, total_bits }`
- [x] **Multi-sentence reassembly** (`AisReassembler`, PRD §A5) — fragment buffers keyed on `(channel, sequential_id)` for every lookup (0.2.0 fix: continuation fragments used to match on sequential id alone, so the same id live on A and B lost both messages); in-order enforcement; bounded-slots eviction (`DEFAULT_MAX_PARTIALS = 16`) plus optional clock-based TTL via `with_timeout_ms`/`tick(now_ms)` (caller owns the clock and the reassembler stores the last tick, ADR-0004 — keeps sans-I/O + `no_std`); `VecDeque<AisError>` pending-queue so multiple simultaneous evictions each surface one `ReassemblyTimeout`
- [x] **`AisFragmentParser<P>` generic wrapper + `Parser` enum** — mirrors `Nmea0183Parser` pattern; composes envelope → `parse_aivdm_wrapper` → `AisReassembler` → `armor::decode` → `decode_message` into a single `feed`/`next_message` loop; surfaces reassembly timeouts between fragments. `tick(now_ms)` passthrough drives the reassembler clock for time-based expiry
- [x] **cargo-fuzz targets** (PRD §F1) — `ais_armor`, `ais_bit_reader`, `ais_parser`. 15 s smoke runs each: 9 M / 1.5 M / 1.25 M executions, zero panics. `just fuzz-smoke-all` and `just fuzz-release` wrap up the set

### Remaining (non-blocking)

- [ ] Golden-file fixtures from real AIS feeds (aishub / marinetraffic public samples)
  Source material kept outside the repo: `~/devel/j/captures/kystverket-2026-10-02.nmea`
  (Kystverket open feed, 203 s, 9 047 lines, 174 single-sentence Type 21, 0 Type 9;
  NLOD licence, attribution required). The 2026-09-22 capture (465 Type 21) is
  gone. The Type 9/21 feature ships gpsd BSD vectors only (2026-10-02).
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
- [x] NMEA typed: `Nmea0183Parser`, `Gga/Gll/Hdg/Hdt/Rmc/Tll/Ttm/Vtg/Psxn/Prdid/Unknown`,
      enums, `DecodeOptions`, per-sentence extension-point functions
- [x] AIS typed: `AisParser` (three clock modes), `AisMessage`, all
      message variants (Types 9 and 21 added in 0.2.0; field-level sum
      types flattened per ADR-0003), `BitReader`

#### Ergonomics

- [x] Context manager support (`__enter__` / `__exit__`) on every parser
      (`with OneShotParser() as p:`, `with StreamingParser() as p:`,
      `with Nmea0183Parser.streaming() as p:`, `with AisParser.streaming() as p:`)
- [x] Async iterator helpers in `marlin.aio`: `aiter_sentences`,
      `aiter_nmea_messages`, `aiter_ais_messages` for `asyncio.StreamReader`
- [x] `@dataclass`-style frozen mirrors in `marlin.dataclasses` with
      `to_dataclass(msg)` dispatcher — JSON / msgspec / dataclasses-asdict
      friendly. Covers all typed runtime classes: envelope RawSentence, NMEA
      Gga/Gll/Hdg/Hdt/Rmc/Tll/Ttm/Vtg/Psxn/Prdid/Unknown, AIS message
      variants (Types 9 and 21 included), and AisMessage
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

- [x] PyPI publish (name reservation, release workflow). Distribution name: `marlin-py` (the bare `marlin` was taken by an unrelated project). Published 2026-04-28: [marlin-py 0.1.0 on PyPI](https://pypi.org/project/marlin-py/). Trusted Publishing via GitHub OIDC; future tag pushes auto-publish through `.github/workflows/python-bindings.yml`
- [ ] JSON / msgspec helper submodule
- [ ] Structure-aware fuzzing integration (once Rust crates gain
      `arbitrary::Arbitrary` derives)

---

## Pre-release checklist (v0.1.0 → published)

Must complete before publishing to crates.io:

- [x] One CPU-hour fuzz run on each of `envelope`, `ais_armor`, `ais_bit_reader`, `ais_parser`, zero findings (PRD §F2). Completed 2026-04-28: 2.69 B total executions across the four targets (envelope 306 M, ais_armor 1.79 B, ais_bit_reader 341 M, ais_parser 251 M), zero panics, zero sanitizer hits, zero artifacts. Re-run with `just fuzz-release` before any future tag push.
- [ ] Replace synthetic fixtures with ≥ 5 real captures per sentence / message type (PRD §G3). Document source of each in fixtures README
- [ ] **Validate PSXN decoder against real captures.** Implementation matches legacy Python semantics but hasn't been cross-checked against a live Seapath/MRU feed. Candidates: `$PSXN,10,...` with known roll/pitch/heave; `$PSXN,11,...` quality-0 variant; any TCMS source using an `sqh` layout
- [ ] **Validate PRDID dialects against real captures.** Dialect orderings come from public integration guides and TSS/Teledyne convention reading. Need live samples from each hardware type
- [ ] Validate AIS decoders against real AIVDM captures — specifically lat/lon sign handling (PRD §A4) and 27/28-bit signed coordinate edge cases
- [x] MSRV double-check: whole workspace compiles on 1.82 (verified 2026-04-28)
- [x] Curate fuzz corpus down to a small regression suite (PRD §F3); commit it. Lives at `fuzz/seeds/<target>/`; auto-bootstrapped on every `just fuzz`
- [ ] Resolve remaining open API questions (checksum policy, `Error::Truncated`) so we don't ship with breaking changes imminent
- [x] Reserve final crate names on crates.io. Published 2026-04-28: [marlin-nmea-envelope 0.1.0](https://crates.io/crates/marlin-nmea-envelope), [marlin-nmea-0183 0.1.0](https://crates.io/crates/marlin-nmea-0183), [marlin-ais 0.1.0](https://crates.io/crates/marlin-ais)
- [x] Tag `v0.1.0`, draft release notes from commit history. **Tag created locally, not pushed; awaiting fuzz-release pass.**

---

## Out of scope for v0.1.0

Tracked here so we don't forget:

- AIS message types beyond 1/2/3/5/18/19/24A/24B (PRD §11)
- AIS Type 24 part-A/part-B pairing (higher-layer concern; PRD §A6)
- Additional NMEA sentences (GSA, GSV, ZDA, DBT, MWV, etc. — PRD §11)
- NMEA 2000 (entirely separate protocol; would be a new crate)
- Encoding / serialization (v0.1 is read-only)
- WASM target verification
- `no_std` embedded profile verification on bare-metal target
