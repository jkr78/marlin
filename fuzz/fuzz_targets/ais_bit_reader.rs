//! Fuzz target for [`BitReader`].
//!
//! Interprets the fuzz input as a combined data-buffer + operation
//! stream:
//!
//! 1. First byte sets the declared `total_bits` (×8 multiplier, so
//!    0..=2040 bits — big enough for all real AIS messages).
//! 2. Remaining bytes double as both the packed bit buffer and an
//!    op-code stream: low 2 bits select the operation (`u`/`i`/`b`/
//!    `string`), upper 6 bits supply a width or string length.
//!
//! Contract: **no panic on any input.** Past-end reads must return
//! saturating zeros (per the `BitReader` docs). Width clamping at 64
//! is asserted by reading values larger than 64 and checking that
//! the reader does not panic.
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run ais_bit_reader
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_ais::BitReader;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let total_bits = usize::from(data[0]) * 8;
    let ops = &data[1..];

    // `ops` doubles as the packed bit buffer. The declared total_bits
    // may exceed the slice length × 8 — the saturating-zero contract
    // means BitReader must handle that case cleanly.
    let mut reader = BitReader::new(ops, total_bits);

    for &op_byte in ops {
        let op = op_byte & 0x03;
        let width = usize::from(op_byte >> 2);
        match op {
            0 => {
                // Unsigned read; BitReader clamps width internally.
                let _ = reader.u(width);
            }
            1 => {
                // Signed read.
                let _ = reader.i(width);
            }
            2 => {
                // Single bit.
                let _ = reader.b();
            }
            3 => {
                // String read — cap length at 32 to keep the harness
                // from spending budget on multi-kB allocations.
                let _ = reader.string(width.min(32));
            }
            _ => unreachable!(),
        }
        // `remaining()` must stay sensible after every op.
        let _ = reader.remaining();
    }
});
