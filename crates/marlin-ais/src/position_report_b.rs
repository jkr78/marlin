//! Class B CS position report — AIS message Type 18.
//!
//! 168-bit payload (ITU-R M.1371-5 Annex 8 §3.16, Table 70). Similar shape to
//! Types 1/2/3 but tailored for Class B transponders (smaller
//! vessels, voluntary carriage) — omits `navigation_status` and
//! `rate_of_turn`, adds Class-B-specific capability flags. The position,
//! speed, course, heading and timestamp fields carry the same field
//! states as on Types 1/2/3.

use marlin_field::FieldState;

use crate::shared_types::{
    timestamp, COURSE_OVER_GROUND, LATITUDE, LONGITUDE, SPEED_OVER_GROUND, TRUE_HEADING,
};
use crate::{AisError, BitReader, Timestamp};

/// Minimum valid payload size for Type 18 (ITU-R M.1371-5 Annex 8
/// §3.16, Table 70).
pub const POSITION_REPORT_B_BITS: usize = 168;

/// Decoded Class B position report.
///
/// Note: `navigation_status` and `rate_of_turn` are **not** present on
/// Class B — the hardware isn't required to track those quantities.
/// The capability flags (`class_b_*`) report what the Class B
/// transponder's firmware supports.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
pub struct PositionReportB {
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
    /// Class B carrier-sense flag: `false` = Class B SOTDMA,
    /// `true` = Class B Carrier Sense.
    pub class_b_cs_flag: bool,
    /// Class B unit has a visual display.
    pub class_b_display_flag: bool,
    /// Class B unit is capable of accepting DSC (Digital Selective Calling).
    pub class_b_dsc_flag: bool,
    /// Class B unit supports the full marine-band set.
    pub class_b_band_flag: bool,
    /// Class B unit accepts Message 22 channel management.
    pub class_b_message22_flag: bool,
    /// Assigned-mode flag (base station assigned this unit a schedule).
    pub assigned_flag: bool,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Radio status field — 20 bits (one wider than the Class A variant).
    pub radio_status: u32,
}

/// Decode a Type 18 Class B position report.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 168`.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_position_report_b(
    bits: &[u8],
    total_bits: usize,
) -> Result<PositionReportB, AisError> {
    if total_bits < POSITION_REPORT_B_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let _ = r.u(8); // reserved (spec says "reserved for regional applications")

    let speed_over_ground = SPEED_OVER_GROUND.read(r.u(10));
    let position_accuracy = r.b();
    let longitude_deg = LONGITUDE.read_signed(r.i(28));
    let latitude_deg = LATITUDE.read_signed(r.i(27));
    let course_over_ground = COURSE_OVER_GROUND.read(r.u(12));
    let true_heading = TRUE_HEADING.read(r.u(9));
    let timestamp = timestamp(r.u(6));
    let _ = r.u(2); // regional reserved
    let class_b_cs_flag = r.b();
    let class_b_display_flag = r.b();
    let class_b_dsc_flag = r.b();
    let class_b_band_flag = r.b();
    let class_b_message22_flag = r.b();
    let assigned_flag = r.b();
    let raim = r.b();
    let radio_status = (r.u(20) & 0xFFFFF) as u32;

    Ok(PositionReportB {
        mmsi,
        speed_over_ground,
        position_accuracy,
        longitude_deg,
        latitude_deg,
        course_over_ground,
        true_heading,
        timestamp,
        class_b_cs_flag,
        class_b_display_flag,
        class_b_dsc_flag,
        class_b_band_flag,
        class_b_message22_flag,
        assigned_flag,
        raim,
        radio_status,
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
    use crate::testing::{undefined, BitWriter};
    use crate::PositioningStatus;

    /// Every Table 70 field a test may want to vary. The reserved bits
    /// are zero; `flags` is cs, display, dsc, band, msg22, assigned, raim.
    struct Fields {
        mmsi: u32,
        sog: u16,
        pos_acc: bool,
        lon_raw: i32,
        lat_raw: i32,
        cog: u16,
        heading: u16,
        timestamp: u8,
        flags: [bool; 7],
        radio: u32,
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
                flags: [false; 7],
                radio: 0,
            }
        }
    }

    /// Pack `f` into a 168-bit Type 18 payload (Table 70).
    fn build_prb(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 18); // msg_type = 18
        w.u(2, 0);
        w.u(30, u64::from(f.mmsi));
        w.u(8, 0); // reserved
        w.u(10, u64::from(f.sog));
        w.b(f.pos_acc);
        w.i(28, i64::from(f.lon_raw));
        w.i(27, i64::from(f.lat_raw));
        w.u(12, u64::from(f.cog));
        w.u(9, u64::from(f.heading));
        w.u(6, u64::from(f.timestamp));
        w.u(2, 0); // regional reserved
        for flag in f.flags {
            w.b(flag);
        }
        w.u(20, u64::from(f.radio));
        w.finish()
    }

    fn decode(f: &Fields) -> PositionReportB {
        let (bits, total) = build_prb(f);
        decode_position_report_b(&bits, total).unwrap()
    }

    #[test]
    fn decodes_happy_path_with_flags() {
        let msg = decode(&Fields {
            mmsi: 367_036_850,
            sog: 150,           // SOG = 15.0 kn
            pos_acc: true,      // pos acc
            lon_raw: 6_600_000, // 11°E
            lat_raw: 2_880_000, // 4.8°N
            cog: 900,           // COG = 90.0°
            heading: 90,        // heading 90°
            timestamp: 30,
            flags: [true, true, false, true, false, false, true],
            radio: 0xA_BCDE,
        });
        assert_eq!(msg.mmsi, 367_036_850);
        assert!((msg.speed_over_ground.value().unwrap() - 15.0).abs() < 1e-4);
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.value().unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.value().unwrap() - 4.8).abs() < 1e-4);
        assert!((msg.course_over_ground.value().unwrap() - 90.0).abs() < 1e-4);
        assert_eq!(msg.true_heading, FieldState::Value(90));
        assert_eq!(msg.timestamp, FieldState::Value(Timestamp::Second(30)));
        assert!(msg.class_b_cs_flag);
        assert!(msg.class_b_display_flag);
        assert!(!msg.class_b_dsc_flag);
        assert!(msg.class_b_band_flag);
        assert!(!msg.class_b_message22_flag);
        assert!(!msg.assigned_flag);
        assert!(msg.raim);
        assert_eq!(msg.radio_status, 0xA_BCDE);
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
            ..Fields::default()
        });
        assert_eq!(msg.speed_over_ground, FieldState::NotAvailable);
        assert_eq!(msg.longitude_deg, FieldState::NotAvailable);
        assert_eq!(msg.latitude_deg, FieldState::NotAvailable);
        assert_eq!(msg.course_over_ground, FieldState::NotAvailable);
        assert_eq!(msg.true_heading, FieldState::NotAvailable);
        assert_eq!(msg.timestamp, FieldState::NotAvailable);
    }

    #[test]
    fn speed_over_ground_rows() {
        for (raw, expected) in [
            (0, FieldState::Value(0.0)),
            (1021, FieldState::Value(102.1)),
            (sentinel::SOG_OVER_RANGE, FieldState::AtLeast(102.2)),
            (sentinel::SOG_NOT_AVAILABLE, FieldState::NotAvailable),
        ] {
            let msg = decode(&Fields {
                sog: raw,
                ..Fields::default()
            });
            match (msg.speed_over_ground, expected) {
                (FieldState::Value(got), FieldState::Value(want))
                | (FieldState::AtLeast(got), FieldState::AtLeast(want)) => {
                    assert!((got - want).abs() < 1e-4, "raw {raw}");
                }
                (got, want) => assert_eq!(got, want, "raw {raw}"),
            }
        }
    }

    #[test]
    fn longitude_and_latitude_rows() {
        let east = 180 * 600_000;
        for (raw, expected) in [
            (east, FieldState::Value(180.0)),
            (-east, FieldState::Value(-180.0)),
            (
                east + 1,
                FieldState::Invalid(undefined(i64::from(east) + 1)),
            ),
            (sentinel::LON_NOT_AVAILABLE as i32, FieldState::NotAvailable),
        ] {
            let msg = decode(&Fields {
                lon_raw: raw,
                ..Fields::default()
            });
            assert_eq!(msg.longitude_deg, expected, "lon raw {raw}");
        }
        let north = 90 * 600_000;
        for (raw, expected) in [
            (north, FieldState::Value(90.0)),
            (-north, FieldState::Value(-90.0)),
            (
                -north - 1,
                FieldState::Invalid(undefined(-i64::from(north) - 1)),
            ),
            (sentinel::LAT_NOT_AVAILABLE as i32, FieldState::NotAvailable),
        ] {
            let msg = decode(&Fields {
                lat_raw: raw,
                ..Fields::default()
            });
            assert_eq!(msg.latitude_deg, expected, "lat raw {raw}");
        }
    }

    #[test]
    fn course_and_heading_rows() {
        for (raw, expected) in [
            (3599, FieldState::Value(359.9)),
            (sentinel::COG_NOT_AVAILABLE, FieldState::NotAvailable),
            (3601, FieldState::Invalid(undefined(3601))),
        ] {
            let msg = decode(&Fields {
                cog: raw,
                ..Fields::default()
            });
            match (msg.course_over_ground, expected) {
                (FieldState::Value(got), FieldState::Value(want)) => {
                    assert!((got - want).abs() < 1e-4, "cog raw {raw}");
                }
                (got, want) => assert_eq!(got, want, "cog raw {raw}"),
            }
        }
        for (raw, expected) in [
            (0, FieldState::Value(0)),
            (359, FieldState::Value(359)),
            (360, FieldState::Invalid(undefined(360))),
            (510, FieldState::Invalid(undefined(510))),
            (sentinel::HEADING_NOT_AVAILABLE, FieldState::NotAvailable),
        ] {
            let msg = decode(&Fields {
                heading: raw,
                ..Fields::default()
            });
            assert_eq!(msg.true_heading, expected, "heading raw {raw}");
        }
    }

    #[test]
    fn timestamp_rows() {
        for (raw, expected) in [
            (59, FieldState::Value(Timestamp::Second(59))),
            (sentinel::TIMESTAMP_NOT_AVAILABLE, FieldState::NotAvailable),
            (
                sentinel::TIMESTAMP_INOPERATIVE,
                FieldState::Value(Timestamp::PositioningStatus(PositioningStatus::Inoperative)),
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
    fn too_short_payload_is_rejected() {
        let buf = [0u8; 10];
        match decode_position_report_b(&buf, 80) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }
}
