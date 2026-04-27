//! Class B static data — AIS message Type 24, Parts A and B.
//!
//! Per ITU-R M.1371-5 §5.3.24, Type 24 splits into two separate AIS
//! messages (not multi-sentence fragments): Part A carries the
//! vessel name, Part B carries ship type, dimensions, and callsign.
//! The two parts share an MMSI but may arrive minutes apart.
//!
//! Per PRD §A6, v1 emits [`StaticDataB24A`] and [`StaticDataB24B`] as
//! independent messages and does **not** pair them. A higher layer
//! can pair by MMSI if desired.

use alloc::string::String;

use crate::shared_types::{dim_u6, dim_u9, trim_ais_string, Dimensions};
use crate::{AisError, BitReader};

/// Minimum valid payload size for either part (ITU-R M.1371-5 §5.3.24).
pub const STATIC_DATA_B_BITS: usize = 168;

/// Which part of Type 24 a sentence carries. Encoded in bits 38–39
/// of the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Type24Part {
    /// Part A — vessel name.
    A,
    /// Part B — ship type, vendor ID, call sign, dimensions.
    B,
    /// Reserved part codes (2 or 3); raw code preserved.
    Reserved(u8),
}

/// Decoded Type 24 Part A — vessel name only.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDataB24A {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Vessel name (up to 20 characters). `None` on all-padding.
    pub vessel_name: Option<String>,
}

/// Decoded Type 24 Part B — ship type, vendor, call sign, dimensions.
///
/// For auxiliary-craft MMSIs (98XXXXXXX pattern), the 30 bits normally
/// holding dimensions carry a mothership MMSI instead. This crate
/// returns the bytes as dimensions regardless; callers interpret based
/// on the MMSI range if they need to distinguish.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDataB24B {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53 raw value.
    pub ship_type: u8,
    /// Vendor ID (up to 7 characters). `None` on all-padding. Per
    /// ITU-R M.1371-5 §5.3.24.2 this is a composite of a 3-char
    /// vendor ID, 4-bit unit-model code, and 20-bit serial number;
    /// we surface the entire 7-char string and let callers split.
    pub vendor_id: Option<String>,
    /// Call sign (up to 7 characters). `None` on all-padding.
    pub call_sign: Option<String>,
    /// Vessel dimensions. For auxiliary craft these bits carry a
    /// mothership MMSI (see struct-level docs).
    pub dimensions: Dimensions,
}

/// Dispatch result from [`decode_static_data_b`].
#[derive(Debug, Clone, PartialEq)]
pub enum StaticDataB {
    /// Part A content.
    PartA(StaticDataB24A),
    /// Part B content.
    PartB(StaticDataB24B),
    /// Reserved part codes (2 or 3). The MMSI is preserved; the
    /// payload is otherwise opaque to this crate.
    Reserved {
        /// Preserved MMSI so callers can correlate with other messages.
        mmsi: u32,
        /// Part code as emitted on the wire.
        part_code: u8,
    },
}

/// Decode a Type 24 payload, dispatching on the 2-bit part-number
/// field to the appropriate part-specific decoder.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 168`.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b(bits: &[u8], total_bits: usize) -> Result<StaticDataB, AisError> {
    if total_bits < STATIC_DATA_B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut peek = BitReader::new(bits, total_bits);
    let _ = peek.u(6); // msg_type
    let _ = peek.u(2); // repeat
    let mmsi_preview = peek.u(30);
    let part = peek.u(2);

    match part {
        0 => decode_static_data_b_24a(bits, total_bits).map(StaticDataB::PartA),
        1 => decode_static_data_b_24b(bits, total_bits).map(StaticDataB::PartB),
        other => Ok(StaticDataB::Reserved {
            mmsi: (mmsi_preview & 0xFFFF_FFFF) as u32,
            part_code: (other & 0x03) as u8,
        }),
    }
}

/// Decode a Type 24 Part A sentence (vessel name).
///
/// The caller asserts the part by calling this function; this decoder
/// does not verify the part-number field. Use
/// [`decode_static_data_b`] if you want automatic dispatch.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b_24a(
    bits: &[u8],
    total_bits: usize,
) -> Result<StaticDataB24A, AisError> {
    if total_bits < STATIC_DATA_B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let _part = r.u(2); // part number (should be 0)
    let vessel_name = trim_ais_string(r.string(20));
    // Remaining bits are spare / padding in Part A.
    Ok(StaticDataB24A { mmsi, vessel_name })
}

/// Decode a Type 24 Part B sentence.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b_24b(
    bits: &[u8],
    total_bits: usize,
) -> Result<StaticDataB24B, AisError> {
    if total_bits < STATIC_DATA_B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let _part = r.u(2); // part number (should be 1)
    let ship_type = (r.u(8) & 0xFF) as u8;
    let vendor_id = trim_ais_string(r.string(7));
    let call_sign = trim_ais_string(r.string(7));
    let dimensions = Dimensions {
        to_bow_m: dim_u9(r.u(9)),
        to_stern_m: dim_u9(r.u(9)),
        to_port_m: dim_u6(r.u(6)),
        to_starboard_m: dim_u6(r.u(6)),
    };
    // Remaining bits (spare + possibly EPFD) ignored — not normative
    // for the common-case Part B layout.
    Ok(StaticDataB24B {
        mmsi,
        ship_type,
        vendor_id,
        call_sign,
        dimensions,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::cast_possible_truncation)]
mod tests {
    use super::*;
    use crate::testing::BitWriter;

    fn write_ais_str(w: &mut BitWriter, s: &[u8], chars: usize) {
        for i in 0..chars {
            let c = s.get(i).copied().unwrap_or(b'@');
            let v = if c >= 64 { c - 64 } else { c };
            w.u(6, u64::from(v));
        }
    }

    fn build_part_a(mmsi: u32, name: &[u8]) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24); // msg_type = 24
        w.u(2, 0); // repeat
        w.u(30, u64::from(mmsi));
        w.u(2, 0); // part A
        write_ais_str(&mut w, name, 20);
        // Pad to 168 bits total — 8 bits of spare.
        w.u(8, 0);
        w.finish()
    }

    #[allow(clippy::too_many_arguments)] // test fixture constructor
    fn build_part_b(
        mmsi: u32,
        ship_type: u8,
        vendor: &[u8],
        callsign: &[u8],
        bow: u16,
        stern: u16,
        port: u8,
        starboard: u8,
    ) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, u64::from(mmsi));
        w.u(2, 1); // part B
        w.u(8, u64::from(ship_type));
        write_ais_str(&mut w, vendor, 7);
        write_ais_str(&mut w, callsign, 7);
        w.u(9, u64::from(bow));
        w.u(9, u64::from(stern));
        w.u(6, u64::from(port));
        w.u(6, u64::from(starboard));
        // Spare: pad to 168.
        w.u(6, 0);
        w.finish()
    }

    #[test]
    fn part_a_decodes_vessel_name() {
        let (bits, total) = build_part_a(123_456_789, b"MY VESSEL");
        let msg = decode_static_data_b_24a(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.vessel_name.as_deref(), Some("MY VESSEL"));
    }

    #[test]
    fn part_b_decodes_all_fields() {
        let (bits, total) = build_part_b(123_456_789, 37, b"VND1234", b"CS001", 30, 10, 5, 3);
        let msg = decode_static_data_b_24b(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.ship_type, 37);
        assert_eq!(msg.vendor_id.as_deref(), Some("VND1234"));
        assert_eq!(msg.call_sign.as_deref(), Some("CS001"));
        assert_eq!(msg.dimensions.to_bow_m, Some(30));
        assert_eq!(msg.dimensions.to_stern_m, Some(10));
        assert_eq!(msg.dimensions.to_port_m, Some(5));
        assert_eq!(msg.dimensions.to_starboard_m, Some(3));
    }

    #[test]
    fn dispatcher_routes_part_a() {
        let (bits, total) = build_part_a(999, b"TESTNAME");
        match decode_static_data_b(&bits, total).unwrap() {
            StaticDataB::PartA(a) => {
                assert_eq!(a.mmsi, 999);
                assert_eq!(a.vessel_name.as_deref(), Some("TESTNAME"));
            }
            other => panic!("expected PartA, got {other:?}"),
        }
    }

    #[test]
    fn dispatcher_routes_part_b() {
        let (bits, total) = build_part_b(1, 70, b"V", b"C", 1, 1, 1, 1);
        match decode_static_data_b(&bits, total).unwrap() {
            StaticDataB::PartB(b) => assert_eq!(b.ship_type, 70),
            other => panic!("expected PartB, got {other:?}"),
        }
    }

    #[test]
    fn dispatcher_surfaces_reserved_part_code() {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, 42);
        w.u(2, 2); // reserved
                   // Pad the remaining 128 bits in two 64-bit writes — BitWriter
                   // can't shift a u64 by 127 in a single u(128, ...) call.
        w.u(64, 0);
        w.u(64, 0);
        let (bits, total) = w.finish();
        match decode_static_data_b(&bits, total).unwrap() {
            StaticDataB::Reserved { mmsi, part_code } => {
                assert_eq!(mmsi, 42);
                assert_eq!(part_code, 2);
            }
            other => panic!("expected Reserved, got {other:?}"),
        }
    }

    #[test]
    fn part_a_all_padding_name_is_none() {
        let (bits, total) = build_part_a(1, b"");
        let msg = decode_static_data_b_24a(&bits, total).unwrap();
        assert_eq!(msg.vessel_name, None);
    }

    #[test]
    fn too_short_payload_rejected() {
        let buf = [0u8; 10];
        match decode_static_data_b(&buf, 100) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }
}
