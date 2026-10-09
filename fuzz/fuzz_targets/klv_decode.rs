//! Fuzz target for the KLV decoder.
//!
//! Feeds arbitrary bytes through `marlin_klv::decode` and
//! `marlin_klv::precision_timestamp`.
//!
//! Contract: **no panic on any input.** Malformed bytes surface as
//! `marlin_klv::Error::*` — all acceptable. And for every set that decodes,
//! `decode(&encode(&set)) == set`: every state `decode` produces re-encodes,
//! and re-decodes to the same set.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run klv_decode
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = marlin_klv::precision_timestamp(data);
    let Ok(set) = marlin_klv::decode(data) else {
        return;
    };
    let mut wire = Vec::new();
    marlin_klv::encode(&set, &mut wire).expect("every decoded set re-encodes");
    let again = marlin_klv::decode(&wire).expect("re-encoded set decodes");
    assert_eq!(again, set, "decode(encode(set)) == set");
});
