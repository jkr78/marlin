//! Fuzz target for the full AIS parser pipeline.
//!
//! Feeds arbitrary bytes through `AisFragmentParser<Streaming>` —
//! exercising the envelope parser, AIVDM wrapper, reassembler, armor
//! decoder, and typed message dispatcher in one harness. This is the
//! integration-shaped counterpart to the unit-level `ais_armor` and
//! `ais_bit_reader` targets.
//!
//! Contract: **no panic on any input.** Malformed bytes surface as
//! `AisError::*` variants — all acceptable.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run ais_parser
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_ais::{AisFragmentParser, AisReassembler};
use marlin_nmea_envelope::Streaming;

fuzz_target!(|data: &[u8]| {
    // Small reassembler cap — overflow eviction becomes easier to
    // exercise in a short fuzz iteration.
    let inner = Streaming::new();
    let reasm = AisReassembler::with_max_partials(4);
    let mut parser = AisFragmentParser::with_reassembler(inner, reasm);

    // Split the input at its midpoint to exercise the streaming
    // buffer boundary — most parser bugs that escape unit tests live
    // on the cross-feed path.
    let split_at = data.len() / 2;
    let (head, tail) = data.split_at(split_at);

    parser.feed(head);
    drain(&mut parser);
    parser.feed(tail);
    drain(&mut parser);
});

fn drain(parser: &mut AisFragmentParser<Streaming>) {
    while let Some(result) = parser.next_message() {
        // Either Ok(msg) or Err(_) is fine; only panics fail the fuzz.
        let _ = result;
    }
}
