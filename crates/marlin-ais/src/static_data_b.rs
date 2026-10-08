//! Class B static data — AIS message Type 24, Parts A and B.
//!
//! Per ITU-R M.1371-5 Annex 8 §3.22, Type 24 splits into two separate AIS
//! messages (not multi-sentence fragments): Part A carries the
//! vessel name, Part B carries ship type, vendor ID, call sign,
//! dimensions (or a mother-ship MMSI, see [`Type24BExtent`]) and EPFD
//! type. The two parts share an MMSI but may arrive minutes apart.
//!
//! This crate emits [`StaticDataB24A`] and [`StaticDataB24B`] as
//! independent messages and does **not** pair them. A higher layer
//! can pair by MMSI if desired.

use alloc::string::String;

use marlin_field::FieldState;

use crate::shared_types::{
    is_auxiliary_craft_mmsi, read_dimensions, read_epfd, trim_ais_string, Dimensions, EpfdType,
    SHIP_TYPE,
};
use crate::{AisError, BitReader};

/// Spec-canonical bit count for Type 24 Part A (ITU-R M.1371-5 Annex 8
/// §3.22, Table 78):
/// 6 `msg_type` + 2 `repeat` + 30 `mmsi` + 2 `part` + 120 `name` = **160**.
pub const STATIC_DATA_B_24A_BITS: usize = 160;

/// Spec-canonical bit count for Type 24 Part B (ITU-R M.1371-5 Annex 8
/// §3.22, Table 79):
/// 40-bit header + 8 `ship_type` + 42 `vendor_id` + 42 `callsign` +
/// 30 `dimensions` + 4 `EPFD` + 2 spare = **168**.
pub const STATIC_DATA_B_24B_BITS: usize = 168;

/// Decoded Type 24 Part A — vessel name only.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDataB24A {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Vessel name (up to 20 characters). Not available on all-padding.
    pub vessel_name: FieldState<String>,
}

/// What the 30-bit extent field of a Type 24 Part B holds, decided by
/// the MMSI prefix (ADR-0002).
///
/// This crate decodes wire fields without interpreting them; this field
/// is the single exception. For an auxiliary craft (`98MIDxxxx`, see
/// [`is_auxiliary_craft_mmsi`]) the bits hold the mother ship's MMSI;
/// read as dimensions they were wrong for every such craft and the
/// caller could not repair them without re-packing the bits. The rule
/// is the USCG MMSI-format convention as gpsd implements it;
/// ITU-R M.1371-5 Table 79 does not state it.
///
/// Both arms keep the 30 bits recoverable: `MothershipMmsi` is a plain
/// field holding them verbatim and `Dimensions` carries a field state
/// per member. Exhaustive: the discriminator is a fixed MMSI-prefix
/// rule, so no third arm can appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type24BExtent {
    /// Dimensions A/B/C/D, for every MMSI that is not an auxiliary craft.
    Dimensions(Dimensions),
    /// MMSI of the mother ship, for auxiliary-craft MMSIs.
    MothershipMmsi(u32),
}

/// Decoded Type 24 Part B — ship type, vendor, call sign, extent
/// (dimensions or mother-ship MMSI) and EPFD type.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDataB24B {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53 raw code. Not
    /// available on `0`; every other code is a value.
    pub ship_type: FieldState<u8>,
    /// Vendor ID (up to 7 characters). Not available on all-padding. Per
    /// ITU-R M.1371-5 Annex 8 §3.22, Table 79A this is a composite of a
    /// 3-char vendor ID, 4-bit unit-model code, and 20-bit serial number;
    /// we surface the entire 7-char string and let callers split.
    pub vendor_id: FieldState<String>,
    /// Call sign (up to 7 characters). Not available on all-padding.
    pub call_sign: FieldState<String>,
    /// Vessel dimensions, or the mother ship's MMSI for an auxiliary
    /// craft. See [`Type24BExtent`].
    pub extent: Type24BExtent,
    /// Electronic position-fixing device type (bits 162–165). Not
    /// available on `0`; invalid with the raw code on the reserved codes
    /// `9..=14`.
    pub epfd: FieldState<EpfdType>,
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
/// field (bits 38–39) to the appropriate part-specific decoder.
///
/// The dispatcher uses Part A's smaller minimum (160 bits) as its own
/// floor; the per-part decoder it dispatches to enforces its own
/// stricter minimum (168 for Part B).
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 160`, or any error
/// from the dispatched per-part decoder.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b(bits: &[u8], total_bits: usize) -> Result<StaticDataB, AisError> {
    if total_bits < STATIC_DATA_B_24A_BITS {
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
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 160`.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b_24a(
    bits: &[u8],
    total_bits: usize,
) -> Result<StaticDataB24A, AisError> {
    if total_bits < STATIC_DATA_B_24A_BITS {
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
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 168`.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_static_data_b_24b(
    bits: &[u8],
    total_bits: usize,
) -> Result<StaticDataB24B, AisError> {
    if total_bits < STATIC_DATA_B_24B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let _part = r.u(2); // part number (should be 1)
    let ship_type = SHIP_TYPE.read(r.u(8));
    let vendor_id = trim_ais_string(r.string(7));
    let call_sign = trim_ais_string(r.string(7));
    let extent = if is_auxiliary_craft_mmsi(mmsi) {
        Type24BExtent::MothershipMmsi((r.u(30) & 0xFFFF_FFFF) as u32)
    } else {
        Type24BExtent::Dimensions(read_dimensions(&mut r))
    };
    let epfd = read_epfd(&mut r);
    // Trailing 2 bits are spare.
    Ok(StaticDataB24B {
        mmsi,
        ship_type,
        vendor_id,
        call_sign,
        extent,
        epfd,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::testing::{armor_encode, text, undefined, write_ais_str, BitWriter};

    fn build_part_a(mmsi: u32, name: &[u8]) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24); // msg_type = 24
        w.u(2, 0); // repeat
        w.u(30, u64::from(mmsi));
        w.u(2, 0); // part A
        write_ais_str(&mut w, name, 20);
        // Total: 6 + 2 + 30 + 2 + 120 = 160 bits per ITU-R M.1371-5 Annex 8 §3.22, Table 78.
        w.finish()
    }

    /// The 30-bit extent field packed as dimensions A/B/C/D in wire
    /// order (9 + 9 + 6 + 6 bits).
    fn dimension_bits(bow: u16, stern: u16, port: u8, starboard: u8) -> u64 {
        (u64::from(bow) << 21)
            | (u64::from(stern) << 12)
            | (u64::from(port) << 6)
            | u64::from(starboard)
    }

    /// Every Table 79 field a test may want to vary. `extent_bits` is the
    /// raw 30-bit field: dimensions via [`dimension_bits`] or a
    /// mother-ship MMSI verbatim. The spare bits are zero.
    struct Fields {
        mmsi: u32,
        ship_type: u8,
        vendor: &'static [u8],
        callsign: &'static [u8],
        extent_bits: u64,
        epfd: u8,
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                mmsi: 123_456_789,
                ship_type: 37,
                vendor: b"VND1234",
                callsign: b"CS001",
                extent_bits: dimension_bits(30, 10, 5, 3),
                epfd: 0,
            }
        }
    }

    fn build_part_b(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, u64::from(f.mmsi));
        w.u(2, 1); // part B
        w.u(8, u64::from(f.ship_type));
        write_ais_str(&mut w, f.vendor, 7);
        write_ais_str(&mut w, f.callsign, 7);
        w.u(30, f.extent_bits);
        w.u(4, u64::from(f.epfd));
        w.u(2, 0); // spare; total 168
        w.finish()
    }

    fn decode_b(f: &Fields) -> StaticDataB24B {
        let (bits, total) = build_part_b(f);
        decode_static_data_b_24b(&bits, total).unwrap()
    }

    #[test]
    fn part_a_decodes_vessel_name() {
        let (bits, total) = build_part_a(123_456_789, b"MY VESSEL");
        let msg = decode_static_data_b_24a(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.vessel_name, text("MY VESSEL"));
    }

    #[test]
    fn part_b_decodes_all_fields() {
        let msg = decode_b(&Fields::default());
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.ship_type, FieldState::Value(37));
        assert_eq!(msg.vendor_id, text("VND1234"));
        assert_eq!(msg.call_sign, text("CS001"));
        assert_eq!(
            msg.extent,
            Type24BExtent::Dimensions(Dimensions {
                to_bow_m: FieldState::Value(30),
                to_stern_m: FieldState::Value(10),
                to_port_m: FieldState::Value(5),
                to_starboard_m: FieldState::Value(3),
            })
        );
        assert_eq!(msg.epfd, FieldState::NotAvailable);
    }

    #[test]
    fn part_b_every_not_available_code_decodes_to_not_available() {
        let msg = decode_b(&Fields {
            ship_type: 0,
            vendor: b"",
            callsign: b"",
            extent_bits: 0,
            epfd: 0,
            ..Fields::default()
        });
        assert_eq!(msg.ship_type, FieldState::NotAvailable);
        assert_eq!(msg.vendor_id, FieldState::NotAvailable);
        assert_eq!(msg.call_sign, FieldState::NotAvailable);
        assert_eq!(
            msg.extent,
            Type24BExtent::Dimensions(Dimensions {
                to_bow_m: FieldState::NotAvailable,
                to_stern_m: FieldState::NotAvailable,
                to_port_m: FieldState::NotAvailable,
                to_starboard_m: FieldState::NotAvailable,
            })
        );
        assert_eq!(msg.epfd, FieldState::NotAvailable);
    }

    #[test]
    fn part_b_ship_type_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (255, FieldState::Value(255)),
        ] {
            let msg = decode_b(&Fields {
                ship_type: raw,
                ..Fields::default()
            });
            assert_eq!(msg.ship_type, expected, "raw {raw}");
        }
    }

    #[test]
    fn part_b_dimension_rows() {
        let msg = decode_b(&Fields {
            extent_bits: dimension_bits(511, 510, 63, 62),
            ..Fields::default()
        });
        assert_eq!(
            msg.extent,
            Type24BExtent::Dimensions(Dimensions {
                to_bow_m: FieldState::AtLeast(511),
                to_stern_m: FieldState::Value(510),
                to_port_m: FieldState::AtLeast(63),
                to_starboard_m: FieldState::Value(62),
            })
        );
        let msg = decode_b(&Fields {
            extent_bits: dimension_bits(1, 0, 1, 0),
            ..Fields::default()
        });
        assert_eq!(
            msg.extent,
            Type24BExtent::Dimensions(Dimensions {
                to_bow_m: FieldState::Value(1),
                to_stern_m: FieldState::NotAvailable,
                to_port_m: FieldState::Value(1),
                to_starboard_m: FieldState::NotAvailable,
            })
        );
    }

    #[test]
    fn part_b_epfd_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(EpfdType::Gps)),
            (8, FieldState::Value(EpfdType::Galileo)),
            (9, FieldState::Invalid(undefined(9))),
            (14, FieldState::Invalid(undefined(14))),
            (15, FieldState::Value(EpfdType::InternalGnss)),
        ] {
            let msg = decode_b(&Fields {
                epfd: raw,
                ..Fields::default()
            });
            assert_eq!(msg.epfd, expected, "raw {raw}");
        }
    }

    /// ADR-0002: for a `98MIDxxxx` auxiliary-craft MMSI the 30 extent
    /// bits are the mother ship's MMSI, surfaced verbatim.
    #[test]
    fn part_b_auxiliary_craft_mmsi_yields_mothership_mmsi() {
        let msg = decode_b(&Fields {
            mmsi: 987_654_321,
            extent_bits: 211_000_123,
            ..Fields::default()
        });
        assert_eq!(msg.extent, Type24BExtent::MothershipMmsi(211_000_123));
    }

    /// The branch is decided by the MMSI prefix alone: the same 30 bits
    /// under a non-auxiliary MMSI read as dimensions.
    #[test]
    fn part_b_non_auxiliary_mmsi_reads_same_bits_as_dimensions() {
        let msg = decode_b(&Fields {
            extent_bits: 211_000_123,
            ..Fields::default()
        });
        assert!(matches!(msg.extent, Type24BExtent::Dimensions(_)));
    }

    /// Pins the armored form of an auxiliary-craft Part B so the Python
    /// unit test `test_static_data_b24b_mothership_mmsi` (bindings) can
    /// decode the same bits without a Python-side bit packer. 168 bits
    /// armor to exactly 28 characters with zero fill bits.
    #[test]
    fn part_b_auxiliary_craft_payload_armors_to_known_string() {
        let (bits, total) = build_part_b(&Fields {
            mmsi: 987_654_321,
            extent_bits: 211_000_123,
            epfd: 1,
            ..Fields::default()
        });
        let armored = b"H>eq`dDUF>4ijkl3Chhi00<Tqds4";
        assert_eq!(armor_encode(&bits, total), (armored.to_vec(), 0));
        assert_eq!(crate::armor::decode(armored, 0).unwrap(), (bits, total));
    }

    #[test]
    fn dispatcher_routes_part_a() {
        let (bits, total) = build_part_a(999, b"TESTNAME");
        match decode_static_data_b(&bits, total).unwrap() {
            StaticDataB::PartA(a) => {
                assert_eq!(a.mmsi, 999);
                assert_eq!(a.vessel_name, text("TESTNAME"));
            }
            other => panic!("expected PartA, got {other:?}"),
        }
    }

    #[test]
    fn dispatcher_routes_part_b() {
        let (bits, total) = build_part_b(&Fields {
            mmsi: 1,
            ship_type: 70,
            vendor: b"V",
            callsign: b"C",
            extent_bits: dimension_bits(1, 1, 1, 1),
            epfd: 0,
        });
        match decode_static_data_b(&bits, total).unwrap() {
            StaticDataB::PartB(b) => assert_eq!(b.ship_type, FieldState::Value(70)),
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
    fn part_a_all_padding_name_is_not_available() {
        let (bits, total) = build_part_a(1, b"");
        let msg = decode_static_data_b_24a(&bits, total).unwrap();
        assert_eq!(msg.vessel_name, FieldState::NotAvailable);
    }

    #[test]
    fn too_short_payload_rejected() {
        // 100 bits is below the spec-canonical Part A floor of 160.
        let buf = [0u8; 13];
        match decode_static_data_b(&buf, 100) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    /// Regression: real-world Type 24 Part A sentences are 27 chars × 6
    /// bits = 162 gross bits, minus 2 fill bits = **160 bits exact**.
    /// That matches ITU-R M.1371-5 Annex 8 §3.22, Table 78 (40-bit header + 120-bit
    /// name). v0.1.0 enforced 168 as the floor for both parts and
    /// rejected every Part A frame with `PayloadTooShort`. Reported by
    /// a Python-bindings consumer with a batch of 160 sentences.
    #[test]
    fn part_a_at_spec_canonical_160_bits_decodes() {
        let payload = b"H42O55i18tMET00000000000000";
        let (bits, total_bits) = crate::armor::decode(payload, 2).unwrap();
        assert_eq!(total_bits, 160, "real-world Part A is 160 bits exactly");
        // Direct decoder works.
        let direct = decode_static_data_b_24a(&bits, total_bits)
            .expect("Part A decode at 160 bits must succeed");
        // Dispatcher routes to PartA.
        match decode_static_data_b(&bits, total_bits)
            .expect("dispatcher must accept 160-bit Part A")
        {
            StaticDataB::PartA(via_dispatcher) => {
                assert_eq!(via_dispatcher, direct, "dispatcher and direct must agree");
                assert_ne!(direct.mmsi, 0, "real payload has a non-zero MMSI");
                assert!(
                    matches!(direct.vessel_name, FieldState::Value(_)),
                    "name field decodes"
                );
            }
            other => panic!("expected PartA, got {other:?}"),
        }
    }

    /// 160-bit input declaring part-number = 1 (Part B). The dispatcher
    /// passes its own 160-bit floor, peeks part=1, dispatches to the
    /// Part B decoder, which rejects on its stricter 168-bit floor.
    #[test]
    fn part_b_below_168_bits_rejected_by_part_b_decoder() {
        let mut w = BitWriter::new();
        w.u(6, 24);
        w.u(2, 0);
        w.u(30, 1);
        w.u(2, 1); // part B
        w.u(64, 0); // 64
        w.u(56, 0); // +56 = 120 bits of zero name field
        let (bits, total) = w.finish();
        assert_eq!(total, 160);
        match decode_static_data_b(&bits, total) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort from Part B decoder, got {other:?}"),
        }
    }
}
