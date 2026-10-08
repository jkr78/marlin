//! Class A static and voyage data — AIS message Type 5.
//!
//! 424-bit payload per ITU-R M.1371-5 Annex 8 §3.3, Table 52, with a
//! floor of 420 bits: many transmitters in the wild ship 420 or 422
//! bits, dropping the trailing spare and DTE bits and the tail of the
//! destination. Spans multiple AIVDM fragments in practice; caller is
//! responsible for having reassembled the bit stream before calling
//! [`decode_static_and_voyage_a`].

use alloc::string::String;

use marlin_field::FieldState;

use crate::shared_types::{
    read_dimensions, read_epfd, sentinel, tenths, trim_ais_string, whole_u8, Partition, SHIP_TYPE,
};
use crate::{AisError, BitReader, Dimensions, EpfdType};

/// The Type 5 floor: the shortest payload at which the layout is still
/// identifiable. Table 52 gives 424 bits; payloads of 420 and 422 bits
/// are common in the wild and decode with the destination read to the
/// last whole character and `dte` not available.
pub const STATIC_VOYAGE_A_BITS: usize = 420;

/// First bit of the 120-bit destination field (Table 52).
const DESTINATION_BIT: usize = 302;

/// Characters in the destination field.
const DESTINATION_CHARS: usize = 20;

/// Bit position of the DTE flag (Table 52).
const DTE_BIT: usize = 422;

/// IMO number, 30 bits: 0 not available, every other code a value. ITU
/// defines the whole range, flag-state numbers included.
const IMO_NUMBER: Partition<u32> = Partition {
    not_available: sentinel::IMO_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 1..=(1 << 30) - 1,
    scale: imo,
};

/// ETA month, 4 bits: 0 not available, 1..=12 defined, 13..=15 invalid.
const ETA_MONTH: Partition<u8> = Partition {
    not_available: sentinel::ETA_MONTH_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 1..=12,
    scale: whole_u8,
};

/// ETA day, 5 bits: 0 not available, 1..=31 defined.
const ETA_DAY: Partition<u8> = Partition {
    not_available: sentinel::ETA_DAY_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 1..=31,
    scale: whole_u8,
};

/// ETA hour, 5 bits: 24 not available, 0..=23 defined, 25..=31 invalid.
const ETA_HOUR: Partition<u8> = Partition {
    not_available: sentinel::ETA_HOUR_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 0..=23,
    scale: whole_u8,
};

/// ETA minute, 6 bits: 60 not available, 0..=59 defined, 61..=63
/// invalid.
const ETA_MINUTE: Partition<u8> = Partition {
    not_available: sentinel::ETA_MINUTE_NOT_AVAILABLE as i64,
    over_range: None,
    defined: 0..=59,
    scale: whole_u8,
};

/// Draught in 0.1 m, 8 bits: 0 not available, 1..=254 defined, 255 at
/// least 25.5 m.
const DRAUGHT: Partition<f32> = Partition {
    not_available: sentinel::DRAUGHT_NOT_AVAILABLE as i64,
    over_range: Some(sentinel::DRAUGHT_OVER_RANGE as i64),
    defined: 1..=254,
    scale: tenths,
};

/// Decoded Class A static and voyage-related data.
///
/// Identity fields (MMSI, IMO, call sign, name) are the vessel's
/// long-term identity. Voyage fields (ETA, destination, draught) are
/// transmitted less frequently and update over the course of a trip.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticAndVoyageA {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// AIS protocol version the transmitter declares it speaks.
    pub ais_version: AisVersion,
    /// `IMO` (International Maritime Organization) number. Not
    /// available on `0`; every other 30-bit code is a value, as ITU
    /// defines the whole range.
    pub imo_number: FieldState<u32>,
    /// Vessel call sign (up to 7 characters). Not available on
    /// all-padding.
    pub call_sign: FieldState<String>,
    /// Vessel name (up to 20 characters). Not available on all-padding.
    pub vessel_name: FieldState<String>,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53 raw code. Not
    /// available on `0`; every other code is a value, as the crate
    /// claims no knowledge of the table.
    pub ship_type: FieldState<u8>,
    /// Vessel dimensions.
    pub dimensions: Dimensions,
    /// Electronic position-fixing device type. Not available on `0`;
    /// invalid with the raw code on the reserved codes `9..=14`.
    pub epfd: FieldState<EpfdType>,
    /// Estimated time of arrival (UTC), one field state per member.
    pub eta: Eta,
    /// Maximum present static draught in metres (1/10 m resolution).
    /// Not available on `0`; `AtLeast(25.5)` on the over-range code
    /// `255`.
    pub draught_m: FieldState<f32>,
    /// Destination (up to 20 characters). Not available on all-padding.
    /// On a payload shorter than 422 bits the field holds the whole
    /// characters transmitted; the partial character is dropped.
    pub destination: FieldState<String>,
    /// Data Terminal Equipment (DTE) flag: `false` = ready (normal
    /// operation), `true` = not ready. Not available when the payload
    /// ends before bit 422, as a 420- or 422-bit payload does. Types 9
    /// and 19 keep `dte: bool`, as their floors cover the bit.
    pub dte: FieldState<bool>,
}

/// ETA field, one field state per member. Each member is its own wire
/// field with its own not-available code (month `0`, day `0`, hour
/// `24`, minute `60`); a code the specification leaves undefined
/// (month `13..=15`, hour `25..=31`, minute `61..=63`) is invalid with
/// the raw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eta {
    /// Month of year (1..=12).
    pub month: FieldState<u8>,
    /// Day of month (1..=31).
    pub day: FieldState<u8>,
    /// Hour of day (0..=23).
    pub hour: FieldState<u8>,
    /// Minute of hour (0..=59).
    pub minute: FieldState<u8>,
}

/// AIS protocol version indicator field (2 bits, ITU-R M.1371-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AisVersion {
    /// 0 — compliant with ITU-R M.1371-1 (original AIS specification).
    Itu1371v1,
    /// 1 — compliant with ITU-R M.1371-3.
    Itu1371v3,
    /// 2 — compliant with ITU-R M.1371-5.
    Itu1371v5,
    /// 3 — future edition (station tags itself as newer than any
    /// released spec at the time of transmission).
    Future,
}

impl AisVersion {
    fn from_u2(v: u8) -> Self {
        match v {
            0 => Self::Itu1371v1,
            1 => Self::Itu1371v3,
            2 => Self::Itu1371v5,
            _ => Self::Future,
        }
    }
}

/// Decode a Type 5 message from its payload.
///
/// A 420- or 422-bit payload decodes with the destination read to its
/// last whole character and `dte` not available; from 423 bits every
/// field is present. Bits past 424 are ignored.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 420`.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_static_and_voyage_a(
    bits: &[u8],
    total_bits: usize,
) -> Result<StaticAndVoyageA, AisError> {
    if total_bits < STATIC_VOYAGE_A_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat indicator
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let ais_version = AisVersion::from_u2((r.u(2) & 0x03) as u8);
    let imo_number = IMO_NUMBER.read(r.u(30));
    let call_sign = trim_ais_string(r.string(7));
    let vessel_name = trim_ais_string(r.string(20));
    let ship_type = SHIP_TYPE.read(r.u(8));
    let dimensions = read_dimensions(&mut r);
    let epfd = read_epfd(&mut r);
    let eta = Eta {
        month: ETA_MONTH.read(r.u(4)),
        day: ETA_DAY.read(r.u(5)),
        hour: ETA_HOUR.read(r.u(5)),
        minute: ETA_MINUTE.read(r.u(6)),
    };
    let draught_m = DRAUGHT.read(r.u(8));

    // Character granularity: the whole characters the payload carries,
    // at most the field's 20. The floor check above puts total_bits at
    // or past DESTINATION_BIT, so the subtraction cannot underflow.
    let destination_chars = ((total_bits - DESTINATION_BIT) / 6).min(DESTINATION_CHARS);
    let destination = trim_ais_string(r.string(destination_chars));

    // The DTE bit is read only when the payload carries it; a 420- or
    // 422-bit payload stops before it. With 20 whole characters read,
    // the reader is positioned at the DTE bit.
    let dte = if total_bits > DTE_BIT {
        FieldState::Value(r.b())
    } else {
        FieldState::NotAvailable
    };

    Ok(StaticAndVoyageA {
        mmsi,
        ais_version,
        imo_number,
        call_sign,
        vessel_name,
        ship_type,
        dimensions,
        epfd,
        eta,
        draught_m,
        destination,
        dte,
    })
}

/// A 30-bit IMO number as `u32`. Partitioned to the 30-bit range, so
/// the fallback is unreachable.
fn imo(raw: i64) -> u32 {
    u32::try_from(raw).unwrap_or(u32::MAX)
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
    use crate::testing::{text, undefined, write_ais_str, BitWriter};

    /// Every Table 52 field a test may want to vary. `destination` is
    /// written as the first `destination_chars` characters of the field
    /// (`@`-padded when shorter), followed by the DTE bit and the spare
    /// bit when `tail_bits` asks for them, so a test can build the 420-,
    /// 422- and 424-bit forms.
    struct Fields {
        mmsi: u32,
        ais_version: u8,
        imo: u32,
        call_sign: &'static [u8],
        name: &'static [u8],
        ship_type: u8,
        bow: u16,
        stern: u16,
        port: u8,
        starboard: u8,
        epfd: u8,
        eta_month: u8,
        eta_day: u8,
        eta_hour: u8,
        eta_minute: u8,
        draught: u8,
        destination: &'static [u8],
        destination_chars: usize,
        dte: bool,
        /// Bits after the destination: 0 (420-bit form with 19
        /// characters, or 422 with 20), 1 (DTE only, 423 bits) or 2
        /// (DTE and spare, the 424-bit table form).
        tail_bits: usize,
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                mmsi: 1,
                ais_version: 0,
                imo: 0,
                call_sign: b"",
                name: b"",
                ship_type: 0,
                bow: 0,
                stern: 0,
                port: 0,
                starboard: 0,
                epfd: 0,
                eta_month: 0,
                eta_day: 0,
                eta_hour: sentinel::ETA_HOUR_NOT_AVAILABLE,
                eta_minute: sentinel::ETA_MINUTE_NOT_AVAILABLE,
                draught: 0,
                destination: b"",
                destination_chars: DESTINATION_CHARS,
                dte: false,
                tail_bits: 2,
            }
        }
    }

    /// Pack `f` into a Type 5 payload (Table 52).
    fn build_type5(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 5); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(f.mmsi));
        w.u(2, u64::from(f.ais_version));
        w.u(30, u64::from(f.imo));
        write_ais_str(&mut w, f.call_sign, 7);
        write_ais_str(&mut w, f.name, 20);
        w.u(8, u64::from(f.ship_type));
        w.u(9, u64::from(f.bow));
        w.u(9, u64::from(f.stern));
        w.u(6, u64::from(f.port));
        w.u(6, u64::from(f.starboard));
        w.u(4, u64::from(f.epfd));
        w.u(4, u64::from(f.eta_month));
        w.u(5, u64::from(f.eta_day));
        w.u(5, u64::from(f.eta_hour));
        w.u(6, u64::from(f.eta_minute));
        w.u(8, u64::from(f.draught));
        write_ais_str(&mut w, f.destination, f.destination_chars);
        if f.tail_bits >= 1 {
            w.b(f.dte);
        }
        if f.tail_bits >= 2 {
            w.u(1, 0); // spare
        }
        w.finish()
    }

    fn decode(f: &Fields) -> StaticAndVoyageA {
        let (bits, total) = build_type5(f);
        decode_static_and_voyage_a(&bits, total).unwrap()
    }

    #[test]
    fn decodes_happy_path() {
        let (bits, total) = build_type5(&Fields {
            mmsi: 123_456_789,
            ais_version: 2, // AisVersion::Itu1371v5
            imo: 9_876_543,
            call_sign: b"ABC1234",
            name: b"MY VESSEL NAME",
            ship_type: 70, // cargo (Table 53)
            bow: 100,
            stern: 50,
            port: 10,
            starboard: 15,
            epfd: 1, // GPS
            eta_month: 6,
            eta_day: 15,
            eta_hour: 12,
            eta_minute: 30,
            draught: 75, // 7.5 m
            destination: b"HAMBURG",
            dte: false,
            ..Fields::default()
        });
        assert_eq!(total, 424);

        let msg = decode_static_and_voyage_a(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.ais_version, AisVersion::Itu1371v5);
        assert_eq!(msg.imo_number, FieldState::Value(9_876_543));
        assert_eq!(msg.call_sign, text("ABC1234"));
        assert_eq!(msg.vessel_name, text("MY VESSEL NAME"));
        assert_eq!(msg.ship_type, FieldState::Value(70));
        assert_eq!(
            msg.dimensions,
            Dimensions {
                to_bow_m: FieldState::Value(100),
                to_stern_m: FieldState::Value(50),
                to_port_m: FieldState::Value(10),
                to_starboard_m: FieldState::Value(15),
            }
        );
        assert_eq!(msg.epfd, FieldState::Value(EpfdType::Gps));
        assert_eq!(
            msg.eta,
            Eta {
                month: FieldState::Value(6),
                day: FieldState::Value(15),
                hour: FieldState::Value(12),
                minute: FieldState::Value(30),
            }
        );
        assert!((msg.draught_m.value().unwrap() - 7.5).abs() < 1e-4);
        assert_eq!(msg.destination, text("HAMBURG"));
        assert_eq!(msg.dte, FieldState::Value(false));
    }

    #[test]
    fn every_not_available_code_decodes_to_not_available() {
        // The defaults are every field's not-available code (ETA hour
        // 24 and minute 60 included); the MMSI is not a field state.
        let msg = decode(&Fields {
            dte: true,
            ..Fields::default()
        });
        assert_eq!(msg.ais_version, AisVersion::Itu1371v1);
        assert_eq!(msg.imo_number, FieldState::NotAvailable);
        assert_eq!(msg.call_sign, FieldState::NotAvailable);
        assert_eq!(msg.vessel_name, FieldState::NotAvailable);
        assert_eq!(msg.ship_type, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_bow_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_stern_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_port_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_starboard_m, FieldState::NotAvailable);
        assert_eq!(msg.epfd, FieldState::NotAvailable);
        assert_eq!(
            msg.eta,
            Eta {
                month: FieldState::NotAvailable,
                day: FieldState::NotAvailable,
                hour: FieldState::NotAvailable,
                minute: FieldState::NotAvailable,
            }
        );
        assert_eq!(msg.draught_m, FieldState::NotAvailable);
        assert_eq!(msg.destination, FieldState::NotAvailable);
        assert_eq!(msg.dte, FieldState::Value(true));
    }

    #[test]
    fn imo_number_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (9_876_543, FieldState::Value(9_876_543)),
            // ITU defines the whole 30-bit range, flag-state numbers
            // included: the last code is a value.
            ((1 << 30) - 1, FieldState::Value((1 << 30) - 1)),
        ] {
            let msg = decode(&Fields {
                imo: raw,
                ..Fields::default()
            });
            assert_eq!(msg.imo_number, expected, "raw {raw}");
        }
    }

    #[test]
    fn ship_type_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (255, FieldState::Value(255)),
        ] {
            let msg = decode(&Fields {
                ship_type: raw,
                ..Fields::default()
            });
            assert_eq!(msg.ship_type, expected, "raw {raw}");
        }
    }

    #[test]
    fn dimension_rows() {
        let msg = decode(&Fields {
            bow: 511,
            stern: 510,
            port: 63,
            starboard: 62,
            ..Fields::default()
        });
        assert_eq!(msg.dimensions.to_bow_m, FieldState::AtLeast(511));
        assert_eq!(msg.dimensions.to_stern_m, FieldState::Value(510));
        assert_eq!(msg.dimensions.to_port_m, FieldState::AtLeast(63));
        assert_eq!(msg.dimensions.to_starboard_m, FieldState::Value(62));
        // ITU's mixed case: A = C = 0 with B and D set decodes per member.
        let msg = decode(&Fields {
            bow: 0,
            stern: 1,
            port: 0,
            starboard: 1,
            ..Fields::default()
        });
        assert_eq!(msg.dimensions.to_bow_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_stern_m, FieldState::Value(1));
        assert_eq!(msg.dimensions.to_port_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_starboard_m, FieldState::Value(1));
    }

    #[test]
    fn epfd_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(EpfdType::Gps)),
            (8, FieldState::Value(EpfdType::Galileo)),
            (9, FieldState::Invalid(undefined(9))),
            (14, FieldState::Invalid(undefined(14))),
            (15, FieldState::Value(EpfdType::InternalGnss)),
        ] {
            let msg = decode(&Fields {
                epfd: raw,
                ..Fields::default()
            });
            assert_eq!(msg.epfd, expected, "raw {raw}");
        }
    }

    #[test]
    fn eta_month_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (12, FieldState::Value(12)),
            (13, FieldState::Invalid(undefined(13))),
            (15, FieldState::Invalid(undefined(15))),
        ] {
            let msg = decode(&Fields {
                eta_month: raw,
                ..Fields::default()
            });
            assert_eq!(msg.eta.month, expected, "raw {raw}");
        }
    }

    #[test]
    fn eta_day_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (31, FieldState::Value(31)),
        ] {
            let msg = decode(&Fields {
                eta_day: raw,
                ..Fields::default()
            });
            assert_eq!(msg.eta.day, expected, "raw {raw}");
        }
    }

    #[test]
    fn eta_hour_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0)),
            (23, FieldState::Value(23)),
            (24, FieldState::NotAvailable),
            (25, FieldState::Invalid(undefined(25))),
            (31, FieldState::Invalid(undefined(31))),
        ] {
            let msg = decode(&Fields {
                eta_hour: raw,
                ..Fields::default()
            });
            assert_eq!(msg.eta.hour, expected, "raw {raw}");
        }
    }

    #[test]
    fn eta_minute_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0)),
            (59, FieldState::Value(59)),
            (60, FieldState::NotAvailable),
            (61, FieldState::Invalid(undefined(61))),
            (63, FieldState::Invalid(undefined(63))),
        ] {
            let msg = decode(&Fields {
                eta_minute: raw,
                ..Fields::default()
            });
            assert_eq!(msg.eta.minute, expected, "raw {raw}");
        }
    }

    #[test]
    fn draught_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(0.1)),
            (254, FieldState::Value(25.4)),
            // Table 52: "25.5 m or greater" is the over-range bound, where
            // 0.2.0 decoded a plain 25.5.
            (255, FieldState::AtLeast(25.5)),
        ] {
            let msg = decode(&Fields {
                draught: raw,
                ..Fields::default()
            });
            match (msg.draught_m, expected) {
                (FieldState::Value(got), FieldState::Value(want))
                | (FieldState::AtLeast(got), FieldState::AtLeast(want)) => {
                    assert!((got - want).abs() < 1e-4, "raw {raw}");
                }
                (got, want) => assert_eq!(got, want, "raw {raw}"),
            }
        }
    }

    #[test]
    fn strings_all_padding_are_not_available_and_trimmed_otherwise() {
        let msg = decode(&Fields {
            call_sign: b"@@@@@@@",
            name: b"NAME   ",
            destination: b"ROTTERDAM@@",
            ..Fields::default()
        });
        assert_eq!(msg.call_sign, FieldState::NotAvailable);
        assert_eq!(msg.vessel_name, text("NAME"));
        assert_eq!(msg.destination, text("ROTTERDAM"));
    }

    // -----------------------------------------------------------------
    // The 420-bit floor: destination at character granularity, DTE
    // not available until the payload carries bit 422
    // -----------------------------------------------------------------

    #[test]
    fn payload_of_420_bits_reads_19_destination_characters_and_no_dte() {
        let (bits, total) = build_type5(&Fields {
            destination: b"ABCDEFGHIJKLMNOPQRST",
            destination_chars: 19,
            tail_bits: 0,
            ..Fields::default()
        });
        // 19 whole characters and 4 bits of the 20th.
        assert_eq!(total, 416);
        let mut bits = bits;
        bits.push(0b1111_0000); // the 4 partial-character bits, set so they would show if read
        let msg = decode_static_and_voyage_a(&bits, 420).unwrap();
        assert_eq!(msg.destination, text("ABCDEFGHIJKLMNOPQRS"));
        assert_eq!(msg.dte, FieldState::NotAvailable);
    }

    #[test]
    fn payload_of_421_bits_reads_19_destination_characters_and_no_dte() {
        // Five bits of the 20th character: still 19 whole characters.
        let (bits, total) = build_type5(&Fields {
            destination: b"ABCDEFGHIJKLMNOPQRST",
            destination_chars: 19,
            tail_bits: 0,
            ..Fields::default()
        });
        assert_eq!(total, 416);
        let mut bits = bits;
        bits.push(0b1111_1000);
        let msg = decode_static_and_voyage_a(&bits, 421).unwrap();
        assert_eq!(msg.destination, text("ABCDEFGHIJKLMNOPQRS"));
        assert_eq!(msg.dte, FieldState::NotAvailable);
    }

    #[test]
    fn payload_of_422_bits_reads_the_whole_destination_and_no_dte() {
        let (bits, total) = build_type5(&Fields {
            destination: b"ABCDEFGHIJKLMNOPQRST",
            tail_bits: 0,
            ..Fields::default()
        });
        assert_eq!(total, 422);
        let msg = decode_static_and_voyage_a(&bits, total).unwrap();
        assert_eq!(msg.destination, text("ABCDEFGHIJKLMNOPQRST"));
        assert_eq!(msg.dte, FieldState::NotAvailable);
    }

    #[test]
    fn payload_of_423_bits_carries_the_dte_flag() {
        let (bits, total) = build_type5(&Fields {
            dte: true,
            tail_bits: 1,
            ..Fields::default()
        });
        assert_eq!(total, 423);
        let msg = decode_static_and_voyage_a(&bits, total).unwrap();
        assert_eq!(msg.dte, FieldState::Value(true));
    }

    #[test]
    fn payload_of_424_bits_carries_the_dte_flag() {
        for dte in [false, true] {
            let (bits, total) = build_type5(&Fields {
                destination: b"HAMBURG",
                dte,
                ..Fields::default()
            });
            assert_eq!(total, 424);
            let msg = decode_static_and_voyage_a(&bits, total).unwrap();
            assert_eq!(msg.destination, text("HAMBURG"));
            assert_eq!(msg.dte, FieldState::Value(dte));
        }
    }

    #[test]
    fn short_destination_is_not_available_at_420_bits_when_all_padding() {
        let (bits, total) = build_type5(&Fields {
            destination_chars: 19,
            tail_bits: 0,
            ..Fields::default()
        });
        let mut bits = bits;
        bits.push(0);
        assert_eq!(total + 4, 420);
        let msg = decode_static_and_voyage_a(&bits, 420).unwrap();
        assert_eq!(msg.destination, FieldState::NotAvailable);
    }

    #[test]
    fn payload_below_the_floor_is_rejected() {
        let (bits, _) = build_type5(&Fields::default());
        match decode_static_and_voyage_a(&bits, STATIC_VOYAGE_A_BITS - 1) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
        let buf = [0u8; 20];
        match decode_static_and_voyage_a(&buf, 100) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    #[test]
    fn ais_version_covers_every_value() {
        assert_eq!(AisVersion::from_u2(0), AisVersion::Itu1371v1);
        assert_eq!(AisVersion::from_u2(1), AisVersion::Itu1371v3);
        assert_eq!(AisVersion::from_u2(2), AisVersion::Itu1371v5);
        assert_eq!(AisVersion::from_u2(3), AisVersion::Future);
    }
}
