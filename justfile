# Marlin — common task runner.
#
# Run `just` with no args to list recipes. All recipes are thin wrappers
# around cargo so that normal `cargo`-level commands still work; this is
# just a cheat sheet with the right args pre-filled.

# Default fuzz duration in seconds. Override on the CLI: `just fuzz 300`.
fuzz_time := "60"

# List available recipes.
default:
    @just --list

# ---------------------------------------------------------------------------
# Build / test / lint
# ---------------------------------------------------------------------------

# Build the whole workspace (all targets).
build:
    cargo build --workspace --all-targets

# Run the full test suite (unit + integration + doctests).
test:
    cargo test --workspace --all-targets
    cargo test --workspace --doc

# Run tests with the `tracing` feature enabled so the feature-gated paths
# get exercised. The compile cost is small; worth including locally.
test-tracing:
    cargo test -p marlin-nmea-envelope --features tracing

# Run clippy across all targets with warnings promoted to errors. This is
# the same gate the CI uses.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Check formatting without modifying files.
fmt-check:
    cargo fmt --all --check

# Format in place.
fmt:
    cargo fmt --all

# Build rustdoc with warnings promoted to errors. Matches CI.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Open the rendered docs in the browser (local dev convenience).
doc-open:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --open

# Run everything CI runs, in order. Use before pushing.
ci: fmt-check build test lint doc

# ---------------------------------------------------------------------------
# Fuzzing (requires nightly + cargo-fuzz)
# ---------------------------------------------------------------------------

# Build all fuzz targets. Catches compile errors without actually
# fuzzing. Useful after API changes.
fuzz-build:
    cd fuzz && cargo +nightly fuzz build

# Run a single fuzz target. `target` is one of `envelope`, `ais_armor`,
# `ais_bit_reader`, `ais_parser`. Duration is in seconds (default 60).
# Usage: `just fuzz envelope 300`, `just fuzz ais_parser`.
fuzz target="envelope" duration=fuzz_time:
    cd fuzz && cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}} -print_final_stats=1

# Smoke-test every target for 30 s each. Roughly what CI does per push.
fuzz-smoke-all:
    @just fuzz envelope 30
    @just fuzz ais_armor 30
    @just fuzz ais_bit_reader 30
    @just fuzz ais_parser 30

# Long-form fuzz run (one CPU-hour per target) — required before a
# release per PRD §F2.
fuzz-release:
    @just fuzz envelope 3600
    @just fuzz ais_armor 3600
    @just fuzz ais_bit_reader 3600
    @just fuzz ais_parser 3600

# List the current fuzz corpus size (number of inputs libfuzzer has kept)
# for the given target.
fuzz-corpus-size target="envelope":
    @ls fuzz/corpus/{{target}} 2>/dev/null | wc -l

# Delete every fuzz corpus and any crash artifacts. Use when starting
# fresh after large parser changes would invalidate accumulated coverage.
fuzz-clean:
    rm -rf fuzz/corpus fuzz/artifacts

# ---------------------------------------------------------------------------
# Release helpers
# ---------------------------------------------------------------------------

# Produce a release build. Uses the `lto = "thin"` profile from Cargo.toml.
release:
    cargo build --workspace --release

# --- Python bindings ---

py-dev:
    cd bindings/python && maturin develop --release

py-build:
    cd bindings/python && maturin build --release

py-test: py-dev
    cd bindings/python && python -m pytest tests/ -v

py-type-check:
    cd bindings/python && python -m mypy --strict .

py-golden-regenerate: py-dev
    cd bindings/python && MARLIN_REGENERATE_GOLDENS=1 python -m pytest tests/golden -v

py-ci: py-dev py-test py-type-check
