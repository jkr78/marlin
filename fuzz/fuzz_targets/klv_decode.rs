//! Fuzz target for the KLV decoder.
//!
//! Feeds arbitrary bytes through `marlin_klv::decode` and
//! `marlin_klv::precision_timestamp`.
//!
//! Contract: **no panic on any input.** Malformed bytes surface as
//! `marlin_klv::Error::*` — all acceptable.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run klv_decode
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = marlin_klv::decode(data);
    let _ = marlin_klv::precision_timestamp(data);
});
