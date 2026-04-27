//! Fuzz target for the envelope's streaming parser.
//!
//! Feeds arbitrary bytes into [`Streaming`] and drains every available
//! sentence. The contract being fuzzed: **no panic, ever**, regardless of
//! input. Malformed bytes must surface as [`Error`] variants.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run envelope
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_nmea_envelope::{SentenceSource, Streaming};

fuzz_target!(|data: &[u8]| {
    let mut parser = Streaming::new();

    // Split the input at arbitrary boundaries to exercise the streaming
    // buffer's fragmentation handling. Using a deterministic split based
    // on the data itself keeps the harness stateless.
    let split_at = data.len() / 2;
    let (head, tail) = data.split_at(split_at);

    parser.feed(head);
    drain(&mut parser);
    parser.feed(tail);
    drain(&mut parser);
});

fn drain(parser: &mut Streaming) {
    while let Some(result) = parser.next_sentence() {
        // We do not care whether a sentence was successfully parsed or
        // produced an Error — only that no panic escapes. Consuming the
        // borrow each iteration lets the next call re-borrow `parser`.
        let _ = result;
    }
}
