//! Class B extended position report — AIS message Type 19.
//!
//! 312-bit payload (ITU-R M.1371-5 Annex 8 §3.17, Table 71). Like Type 18
//! for the
//! position portion, with the Class A static-data tail (vessel name,
//! ship type, dimensions, EPFD) appended. Rarely seen on Class B
//! feeds — most Class B units transmit Type 18 + Type 24 Part A/B
//! instead.

use alloc::string::String;

use marlin_field::FieldState;

use crate::shared_types::{
    read_dimensions, read_epfd, timestamp, trim_ais_string, COURSE_OVER_GROUND, LATITUDE,
    LONGITUDE, SHIP_TYPE, SPEED_OVER_GROUND, TRUE_HEADING,
};
use crate::{AisError, BitReader, Dimensions, EpfdType, Timestamp};

/// Minimum valid payload size for Type 19 (ITU-R M.1371-5 Annex 8
/// §3.17, Table 71).
pub const EXTENDED_POSITION_REPORT_B_BITS: usize = 312;

/// Decoded Class B extended position report.
///
/// Combines the Type 18 position fields with the Type 5 static tail
/// (name, ship type, dimensions, EPFD). Unlike Type 18, there are no
/// carrier-sense / display / DSC capability flags — those belong only
/// to the standard Class B CS report.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
pub struct ExtendedPositionReportB {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Speed over ground in knots. Not available on `1023`;
    /// `AtLeast(102.2)` on the over-range code `1022`.
    pub speed_over_ground: FieldState<f32>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes.
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees. Not available on `181°`;
    /// invalid with the raw code on any other code beyond ±180°.
    pub longitude_deg: FieldState<f64>,
    /// Latitude in signed decimal degrees. Not available on `91°`;
    /// invalid with the raw code on any other code beyond ±90°.
    pub latitude_deg: FieldState<f64>,
    /// Course over ground in degrees. Not available on `3600`; invalid
    /// on `3601..=4095`.
    pub course_over_ground: FieldState<f32>,
    /// True heading in degrees (0..=359). Not available on `511`;
    /// invalid on `360..=510`.
    pub true_heading: FieldState<u16>,
    /// Second of the UTC minute of the position fix, or a
    /// positioning-system status (codes 61..=63). Not available on `60`.
    pub timestamp: FieldState<Timestamp>,
    /// Vessel name (up to 20 characters). Not available on all-padding.
    pub vessel_name: FieldState<String>,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53 raw code. Not
    /// available on `0`; every other code is a value.
    pub ship_type: FieldState<u8>,
    /// Vessel dimensions.
    pub dimensions: Dimensions,
    /// Electronic position-fixing device type. Not available on `0`;
    /// invalid with the raw code on the reserved codes `9..=14`.
    pub epfd: FieldState<EpfdType>,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Data Terminal Equipment flag: `false` = ready, `true` = not ready.
    /// A plain field here: the 312-bit floor covers the bit, unlike on
    /// Type 5 where it is a `FieldState<bool>`.
    pub dte: bool,
    /// Assigned-mode flag (base station assigned this unit a schedule).
    pub assigned_flag: bool,
}

/// Decode a Type 19 Class B extended position report.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 312`.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_extended_position_report_b(
    bits: &[u8],
    total_bits: usize,
) -> Result<ExtendedPositionReportB, AisError> {
    if total_bits < EXTENDED_POSITION_REPORT_B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let _ = r.u(8); // reserved

    let speed_over_ground = SPEED_OVER_GROUND.read(r.u(10));
    let position_accuracy = r.b();
    let longitude_deg = LONGITUDE.read_signed(r.i(28));
    let latitude_deg = LATITUDE.read_signed(r.i(27));
    let course_over_ground = COURSE_OVER_GROUND.read(r.u(12));
    let true_heading = TRUE_HEADING.read(r.u(9));
    let timestamp = timestamp(r.u(6));
    let _ = r.u(4); // regional reserved

    let vessel_name = trim_ais_string(r.string(20));
    let ship_type = SHIP_TYPE.read(r.u(8));
    let dimensions = read_dimensions(&mut r);
    let epfd = read_epfd(&mut r);
    let raim = r.b();
    let dte = r.b();
    let assigned_flag = r.b();
    let _ = r.u(4); // spare

    Ok(ExtendedPositionReportB {
        mmsi,
        speed_over_ground,
        position_accuracy,
        longitude_deg,
        latitude_deg,
        course_over_ground,
        true_heading,
        timestamp,
        vessel_name,
        ship_type,
        dimensions,
        epfd,
        raim,
        dte,
        assigned_flag,
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
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use crate::shared_types::sentinel;
    use crate::testing::{undefined, write_ais_str, BitWriter};
    use crate::PositioningStatus;

    /// Every Table 71 field a test may want to vary. The reserved and
    /// spare bits are zero.
    #[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
    struct Fields {
        mmsi: u32,
        sog: u16,
        pos_acc: bool,
        lon_raw: i32,
        lat_raw: i32,
        cog: u16,
        heading: u16,
        timestamp: u8,
        name: &'static [u8],
        ship_type: u8,
        bow: u16,
        stern: u16,
        port: u8,
        starboard: u8,
        epfd: u8,
        raim: bool,
        dte: bool,
        assigned: bool,
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                mmsi: 1,
                sog: 0,
                pos_acc: false,
                lon_raw: 0,
                lat_raw: 0,
                cog: 0,
                heading: 0,
                timestamp: 0,
                name: b"",
                ship_type: 0,
                bow: 0,
                stern: 0,
                port: 0,
                starboard: 0,
                epfd: 0,
                raim: false,
                dte: false,
                assigned: false,
            }
        }
    }

    /// Pack `f` into a 312-bit Type 19 payload (Table 71).
    fn build_t19(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 19); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(f.mmsi));
        w.u(8, 0); // reserved
        w.u(10, u64::from(f.sog));
        w.b(f.pos_acc);
        w.i(28, i64::from(f.lon_raw));
        w.i(27, i64::from(f.lat_raw));
        w.u(12, u64::from(f.cog));
        w.u(9, u64::from(f.heading));
        w.u(6, u64::from(f.timestamp));
        w.u(4, 0); // regional reserved
        write_ais_str(&mut w, f.name, 20);
        w.u(8, u64::from(f.ship_type));
        w.u(9, u64::from(f.bow));
        w.u(9, u64::from(f.stern));
        w.u(6, u64::from(f.port));
        w.u(6, u64::from(f.starboard));
        w.u(4, u64::from(f.epfd));
        w.b(f.raim);
        w.b(f.dte);
        w.b(f.assigned);
        w.u(4, 0); // spare
        w.finish()
    }

    fn decode(f: &Fields) -> ExtendedPositionReportB {
        let (bits, total) = build_t19(f);
        decode_extended_position_report_b(&bits, total).unwrap()
    }

    #[test]
    fn decodes_happy_path() {
        let msg = decode(&Fields {
            mmsi: 369_493_000,
            sog: 150,           // 15.0 kn
            pos_acc: true,      // pos acc
            lon_raw: 6_600_000, // 11°E
            lat_raw: 2_880_000, // 4.8°N
            cog: 900,           // COG 90.0°
            heading: 90,
            timestamp: 30,
            name: b"VESSEL NAME",
            ship_type: 37,
            bow: 30,
            stern: 10,
            port: 5,
            starboard: 3,
            epfd: 1, // EPFD GPS
            raim: true,
            dte: false,
            assigned: false,
        });
        assert_eq!(msg.mmsi, 369_493_000);
        assert!((msg.speed_over_ground.value().unwrap() - 15.0).abs() < 1e-4);
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.value().unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.value().unwrap() - 4.8).abs() < 1e-4);
        assert!((msg.course_over_ground.value().unwrap() - 90.0).abs() < 1e-4);
        assert_eq!(msg.true_heading, FieldState::Value(90));
        assert_eq!(msg.timestamp, FieldState::Value(Timestamp::Second(30)));
        assert_eq!(
            msg.vessel_name,
            FieldState::Value(String::from("VESSEL NAME"))
        );
        assert_eq!(msg.ship_type, FieldState::Value(37));
        assert_eq!(
            msg.dimensions,
            Dimensions {
                to_bow_m: FieldState::Value(30),
                to_stern_m: FieldState::Value(10),
                to_port_m: FieldState::Value(5),
                to_starboard_m: FieldState::Value(3),
            }
        );
        assert_eq!(msg.epfd, FieldState::Value(EpfdType::Gps));
        assert!(msg.raim);
        assert!(!msg.dte);
        assert!(!msg.assigned_flag);
    }

    #[test]
    fn every_not_available_code_decodes_to_not_available() {
        let msg = decode(&Fields {
            sog: sentinel::SOG_NOT_AVAILABLE,
            lon_raw: sentinel::LON_NOT_AVAILABLE as i32,
            lat_raw: sentinel::LAT_NOT_AVAILABLE as i32,
            cog: sentinel::COG_NOT_AVAILABLE,
            heading: sentinel::HEADING_NOT_AVAILABLE,
            timestamp: sentinel::TIMESTAMP_NOT_AVAILABLE,
            name: b"",
            ship_type: 0,
            epfd: 0,
            ..Fields::default()
        });
        assert_eq!(msg.speed_over_ground, FieldState::NotAvailable);
        assert_eq!(msg.longitude_deg, FieldState::NotAvailable);
        assert_eq!(msg.latitude_deg, FieldState::NotAvailable);
        assert_eq!(msg.course_over_ground, FieldState::NotAvailable);
        assert_eq!(msg.true_heading, FieldState::NotAvailable);
        assert_eq!(msg.timestamp, FieldState::NotAvailable);
        assert_eq!(msg.vessel_name, FieldState::NotAvailable);
        assert_eq!(msg.ship_type, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_bow_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_stern_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_port_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_starboard_m, FieldState::NotAvailable);
        assert_eq!(msg.epfd, FieldState::NotAvailable);
    }

    #[test]
    fn position_speed_course_heading_rows() {
        let msg = decode(&Fields {
            sog: sentinel::SOG_OVER_RANGE,
            lon_raw: 180 * 600_000 + 1,
            lat_raw: -90 * 600_000 - 1,
            cog: 3601,
            heading: 360,
            ..Fields::default()
        });
        assert_eq!(msg.speed_over_ground, FieldState::AtLeast(102.2));
        assert_eq!(
            msg.longitude_deg,
            FieldState::Invalid(undefined(180 * 600_000 + 1))
        );
        assert_eq!(
            msg.latitude_deg,
            FieldState::Invalid(undefined(-90 * 600_000 - 1))
        );
        assert_eq!(msg.course_over_ground, FieldState::Invalid(undefined(3601)));
        assert_eq!(msg.true_heading, FieldState::Invalid(undefined(360)));

        let msg = decode(&Fields {
            sog: 1021,
            lon_raw: -180 * 600_000,
            lat_raw: 90 * 600_000,
            cog: 3599,
            heading: 359,
            ..Fields::default()
        });
        assert!((msg.speed_over_ground.value().unwrap() - 102.1).abs() < 1e-4);
        assert_eq!(msg.longitude_deg, FieldState::Value(-180.0));
        assert_eq!(msg.latitude_deg, FieldState::Value(90.0));
        assert!((msg.course_over_ground.value().unwrap() - 359.9).abs() < 1e-4);
        assert_eq!(msg.true_heading, FieldState::Value(359));
    }

    #[test]
    fn timestamp_rows() {
        for (raw, expected) in [
            (59, FieldState::Value(Timestamp::Second(59))),
            (sentinel::TIMESTAMP_NOT_AVAILABLE, FieldState::NotAvailable),
            (
                sentinel::TIMESTAMP_DEAD_RECKONING,
                FieldState::Value(Timestamp::PositioningStatus(
                    PositioningStatus::DeadReckoning,
                )),
            ),
        ] {
            let msg = decode(&Fields {
                timestamp: raw,
                ..Fields::default()
            });
            assert_eq!(msg.timestamp, expected, "raw {raw}");
        }
    }

    #[test]
    fn ship_type_rows() {
        for (raw, expected) in [
            (0, FieldState::NotAvailable),
            (1, FieldState::Value(1)),
            (37, FieldState::Value(37)),
            // Table 53's reserved range is a value: the crate claims no
            // knowledge of the table.
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
            bow: sentinel::DIMENSION_LONG_OVER_RANGE,
            stern: 510,
            port: sentinel::DIMENSION_SHORT_OVER_RANGE,
            starboard: 62,
            ..Fields::default()
        });
        assert_eq!(
            msg.dimensions,
            Dimensions {
                to_bow_m: FieldState::AtLeast(511),
                to_stern_m: FieldState::Value(510),
                to_port_m: FieldState::AtLeast(63),
                to_starboard_m: FieldState::Value(62),
            }
        );
        let msg = decode(&Fields {
            bow: 1,
            stern: 0,
            port: 1,
            starboard: 0,
            ..Fields::default()
        });
        assert_eq!(msg.dimensions.to_bow_m, FieldState::Value(1));
        assert_eq!(msg.dimensions.to_stern_m, FieldState::NotAvailable);
        assert_eq!(msg.dimensions.to_port_m, FieldState::Value(1));
        assert_eq!(msg.dimensions.to_starboard_m, FieldState::NotAvailable);
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
    fn flag_ordering_raim_dte_assigned_is_spec_correct() {
        // Check each flag in isolation — a misordered trio would fail at
        // least two of these.
        for (raim, dte, assigned) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let msg = decode(&Fields {
                name: b"X",
                raim,
                dte,
                assigned,
                ..Fields::default()
            });
            assert_eq!(msg.raim, raim);
            assert_eq!(msg.dte, dte);
            assert_eq!(msg.assigned_flag, assigned);
        }
    }

    #[test]
    fn too_short_payload_is_rejected() {
        let buf = [0u8; 10];
        match decode_extended_position_report_b(&buf, 80) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }
}
