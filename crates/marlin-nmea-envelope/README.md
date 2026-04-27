# marlin-nmea-envelope

Sans-I/O NMEA 0183 envelope parser. Foundation crate of the `marlin` suite.

## What this crate does

- Locates NMEA 0183 sentence boundaries in a byte feed
- Verifies the XOR checksum (bytes between `$`/`!` and `*`)
- Splits the payload into fields (empty fields preserved as empty slices)
- Recognizes NMEA 4.10 TAG block prefixes (`\...*hh\`) and captures
  their raw content bytes
- Exposes a unified [`SentenceSource`] trait with two implementations:
  - [`OneShot`] — one complete sentence per `feed` (for UDP datagrams)
  - [`Streaming`] — buffered scanning for TCP-style byte streams
- Plus a [`Parser`] enum for zero-cost runtime mode selection

## What this crate does not do

- **No I/O.** The caller drives bytes in and parsed sentences out.
- **No typed decoding** of specific sentence types. That is the job of
  `marlin-nmea-0183` and `marlin-ais`.

## Quickstart

### UDP-datagram mode (one sentence per `feed`, no terminator required)

```rust
use marlin_nmea_envelope::{OneShot, SentenceSource};

let mut parser = OneShot::new();
parser.feed(b"$GPGGA,123519*77"); // checksum 0x77 = XOR of "GPGGA,123519"
let sentence = parser.next_sentence().unwrap().unwrap();
assert_eq!(sentence.sentence_type, "GGA");
assert!(sentence.checksum_ok);
```

### TCP-stream mode (multiple sentences per feed, terminators consumed)

```rust
use marlin_nmea_envelope::{Streaming, SentenceSource};

let mut parser = Streaming::new();
parser.feed(b"$GPGGA,123519*77\r\n$GPGGA,123519*77\r\n");
while let Some(result) = parser.next_sentence() {
    let sentence = result.expect("parse error");
    // ...
}
```

### Runtime dispatch (config-driven mode selection)

```rust
use marlin_nmea_envelope::Parser;

let mut parser = if transport_is_udp {
    Parser::one_shot()
} else {
    Parser::streaming()
};
// Identical call shape for both modes.
parser.feed(&bytes);
while let Some(Ok(s)) = parser.next_sentence() { ... }
```

## Features

| Flag | Default | Purpose |
| --- | --- | --- |
| `tracing` | off | Emits `tracing` events for discarded garbage, buffer overflows, and TAG block checksum mismatches. Add a subscriber in the host application to see them. |

## Architectural commitments

- **Sans-I/O** — no sockets, no runtime, no file handles.
- **One parser core** shared by both modes.
- **`complete` nom parsers** only — no `Err::Incomplete` propagation.
- **Zero-copy** borrows; no allocation on the sentence hot path.
- **TAG checksum mismatches are advisory**, not fatal (PRD decision 7).
- **Panic-free** on all inputs; verified by cargo-fuzz.

## Minimum Supported Rust Version

1.82

## License

Licensed under either of Apache License 2.0 or MIT License at your option.

[`SentenceSource`]: https://docs.rs/marlin-nmea-envelope/latest/marlin_nmea_envelope/trait.SentenceSource.html
[`OneShot`]: https://docs.rs/marlin-nmea-envelope/latest/marlin_nmea_envelope/struct.OneShot.html
[`Streaming`]: https://docs.rs/marlin-nmea-envelope/latest/marlin_nmea_envelope/struct.Streaming.html
[`Parser`]: https://docs.rs/marlin-nmea-envelope/latest/marlin_nmea_envelope/enum.Parser.html
