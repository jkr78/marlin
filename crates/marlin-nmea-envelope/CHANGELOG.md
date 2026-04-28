# Changelog

All notable changes to `marlin-nmea-envelope` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Initial release of `marlin-nmea-envelope` — the sans-I/O framing layer
  for the Marlin suite. Verifies checksums, splits fields, locates
  sentence boundaries, and recognizes optional NMEA 4.10 TAG blocks
- `SentenceSource` trait with a generic associated type for zero-copy
  `RawSentence<'a>` borrows from caller buffers
- Two sources behind one shared nom parser core:
  - `OneShot` for datagram transports (UDP)
  - `Streaming` for byte-stream transports (TCP, serial)
- `Parser` runtime-dispatch enum that avoids `Box<dyn SentenceSource>`
- TAG block parsing with advisory-only checksum (PRD decision 7); the
  wrapped sentence still surfaces with its own checksum status
- Proprietary-sentence detection: `$P…` sets `talker = None` and packs
  the full address into `sentence_type`
- `pub fn parse(&[u8])` one-shot entry point for callers who do not need
  the `SentenceSource` machinery
- `#![no_std]` with `extern crate alloc`; no transitive `std` dependency
- `tracing` feature gate (off by default) for structured parse-error logs
- Full rustdoc and README
- Seven golden-file fixtures under `tests/fixtures/`
- `cargo-fuzz` target (`envelope`); 60-second smoke run reached 3.97 M
  executions with zero panics
