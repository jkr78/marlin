//! Fuzz target for the AIS ASCII armor decoder.
//!
//! Contract: `armor::decode` must never panic, integer-overflow, or
//! allocate unboundedly, regardless of input. Malformed bytes surface
//! as `AisError::InvalidArmorChar` / `InvalidFillBits` /
//! `PayloadTooShort` / `PayloadTooLong` — all of which are fine.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run ais_armor
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_ais::armor;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // First byte drives `fill_bits`. We let it take any value 0..=255
    // so the `InvalidFillBits` path (values > 5) is exercised too.
    let fill_bits = data[0];
    let payload = &data[1..];
    let _ = armor::decode(payload, fill_bits);
});
