# marlin-fuzz

Fuzz targets for the marlin suite. **Requires nightly Rust.**

## Prerequisites

```sh
cargo install cargo-fuzz          # one-time
rustup toolchain install nightly  # one-time
```

## Run

```sh
# Short smoke test on a single target:
cargo +nightly fuzz run envelope       -- -max_total_time=60
cargo +nightly fuzz run ais_armor      -- -max_total_time=60
cargo +nightly fuzz run ais_bit_reader -- -max_total_time=60
cargo +nightly fuzz run ais_parser     -- -max_total_time=60

# Via just:
just fuzz envelope 60
just fuzz-smoke-all            # 30 s each target (~ 2 min total)
just fuzz-release              # one CPU-hour per target (PRD §F2)
```

## Targets

| Name             | Fuzzes                                                   |
| ---------------- | -------------------------------------------------------- |
| `envelope`       | `Streaming::feed` + `next_sentence` loop                 |
| `ais_armor`      | `armor::decode(payload, fill_bits)`                      |
| `ais_bit_reader` | `BitReader::{u,i,b,string}` over arbitrary packed bytes  |
| `ais_parser`     | Full `AisFragmentParser<Streaming>` pipeline (envelope → AIVDM wrapper → reassembler → armor → typed decode) |

Targets for `marlin-nmea-0183` land when the crate needs dedicated
coverage beyond what's exercised via `envelope` + the typed NMEA
decoders' unit tests (PRD §8.3).

## What is being fuzzed

The contract: **no panic on any input**. Fuzz targets feed arbitrary bytes
into the parser and drain every available sentence. Successful parses and
errors are both acceptable outcomes; the only failure mode is a crash
(panic, UB, OOM).

## Workspace relationship

The `fuzz/` directory is **excluded** from the main workspace (see the
`exclude = ["fuzz"]` entry in the root `Cargo.toml`). This is because
cargo-fuzz injects nightly-only sanitizer flags, which would infect the
stable-toolchain build if the fuzz crate were a workspace member.
