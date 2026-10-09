//! Fuzz target for the NMEA 0183 typed decoders.
//!
//! Feeds arbitrary bytes through `marlin_nmea_envelope::parse` and, for
//! every sentence the envelope accepts, through `marlin_nmea_0183::decode`
//! with the default options.
//!
//! Contract: **no panic on any input**, and a decode fails only for a
//! structural reason. A field's value never fails the sentence, so the
//! only `Nmea0183DecodeError` the typed layer may return is `NotEnoughFields`.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run nmea_0183_decode
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_nmea_0183::Nmea0183DecodeError;

fuzz_target!(|data: &[u8]| {
    let Ok(raw) = marlin_nmea_envelope::parse(data) else {
        return;
    };
    match marlin_nmea_0183::decode(&raw) {
        Ok(_) | Err(Nmea0183DecodeError::NotEnoughFields { .. }) => {}
        Err(other) => panic!("decode failed for a non-structural reason: {other:?}"),
    }
});
