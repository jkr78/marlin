//! Fuzz target for the KLV encoder.
//!
//! Builds an `St0601` from arbitrary bytes, every field in any of the five
//! field states with arbitrary `f64` payloads, plus unknown entries on tags
//! the crate does not type, and encodes it.
//!
//! Contract: **no panic on any input.** A set the wire cannot carry surfaces
//! as `marlin_klv::EncodeError::*` — acceptable. And whenever `encode(&set)`
//! is `Ok(b)`, `decode(&b)` is `Ok` and `encode(&decode(&b)) == b` byte for
//! byte: the encoder's output is a fixed point. (`decode(&encode(&set)) ==
//! set` cannot hold for a generated set: a value quantises to its wire count,
//! and an unparsable field with no bytes in `unknown` re-decodes as not
//! available.)
//!
//! Run:
//! ```sh
//! cargo +nightly fuzz run klv_encode
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use marlin_klv::{FieldState, Invalid, RawCode, St0601};

/// A cursor over the fuzz input; exhausted input reads as zero.
struct Cursor<'a> {
    data: &'a [u8],
}

impl Cursor<'_> {
    fn u8(&mut self) -> u8 {
        let (first, rest) = self
            .data
            .split_first()
            .map_or((0, &[][..]), |(b, r)| (*b, r));
        self.data = rest;
        first
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.u8(), self.u8()])
    }

    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(std::array::from_fn(|_| self.u8()))
    }

    /// An arbitrary `f64` (NaN and infinities included), or one inside a
    /// plausible engineering band so the value paths get exercised too.
    fn f64(&mut self) -> f64 {
        let (lo, hi) = match self.u8() % 5 {
            0 => return f64::from_le_bytes(std::array::from_fn(|_| self.u8())),
            1 => (-180.0, 180.0),
            2 => (-900.0, 19_000.0),
            3 => (0.0, 5_000_000.0),
            _ => (0.0, 255.0),
        };
        lo + (hi - lo) * f64::from(self.u16()) / 65535.0
    }

    /// One of the five field states with a payload drawn by `payload`.
    fn state<T>(&mut self, payload: impl FnOnce(&mut Self) -> T) -> FieldState<T> {
        match self.u8() % 7 {
            0 | 1 => FieldState::Value(payload(self)),
            2 => FieldState::AtLeast(payload(self)),
            3 => FieldState::NotAvailable,
            4 => FieldState::SenderError(RawCode(self.code())),
            5 => FieldState::Invalid(Invalid::Unparsable),
            _ => FieldState::Invalid(Invalid::Undefined(RawCode(self.code()))),
        }
    }

    /// A raw code: often one of the two ST 0601 sentinels, so the
    /// sender-error emission path is reached.
    fn code(&mut self) -> i64 {
        match self.u8() % 3 {
            0 => i16::MIN.into(),
            1 => i32::MIN.into(),
            _ => self.i64(),
        }
    }

    fn set(&mut self) -> St0601 {
        let mut set = St0601::new(u64::from_le_bytes(std::array::from_fn(|_| self.u8())));
        set.version = self.state(Self::u8);
        for field in [
            &mut set.platform_heading_degrees,
            &mut set.platform_pitch_degrees,
            &mut set.platform_roll_degrees,
            &mut set.platform_true_airspeed_mps,
            &mut set.sensor_latitude_degrees,
            &mut set.sensor_longitude_degrees,
            &mut set.sensor_true_altitude_meters,
            &mut set.sensor_horizontal_fov_degrees,
            &mut set.sensor_vertical_fov_degrees,
            &mut set.sensor_relative_azimuth_degrees,
            &mut set.sensor_relative_elevation_degrees,
            &mut set.sensor_relative_roll_degrees,
            &mut set.slant_range_meters,
            &mut set.target_width_meters,
            &mut set.frame_center_latitude_degrees,
            &mut set.frame_center_longitude_degrees,
            &mut set.frame_center_elevation_meters,
            &mut set.target_location_latitude_degrees,
            &mut set.target_location_longitude_degrees,
            &mut set.target_location_elevation_meters,
        ] {
            *field = self.state(Self::f64);
        }
        for _ in 0..self.u8() % 4 {
            let tag = self.u8();
            let len = usize::from(self.u8() % 8);
            let value: Vec<u8> = (0..len).map(|_| self.u8()).collect();
            // Tag 1 is framing and the typed tags would collide with their fields.
            if tag != 1 && marlin_klv::tag_name(tag).is_none() {
                set.unknown.push((tag, value));
            }
        }
        set
    }
}

fuzz_target!(|data: &[u8]| {
    let set = Cursor { data }.set();
    let mut wire = Vec::new();
    if marlin_klv::encode(&set, &mut wire).is_err() {
        assert!(wire.is_empty(), "out is untouched on Err");
        return;
    }
    let decoded = marlin_klv::decode(&wire).expect("encoder output decodes");
    let mut again = Vec::new();
    marlin_klv::encode(&decoded, &mut again).expect("decoded set re-encodes");
    assert_eq!(again, wire, "encode(decode(b)) == b");
});
