//! Standard SAR aircraft position report — AIS message Type 9.
//!
//! 168-bit payload (ITU-R M.1371-5 Annex 8 §3.7, Table 59). Shaped
//! like a Class A position report but for search-and-rescue aircraft:
//! altitude replaces navigational status and rate of turn, speed over
//! ground is in whole knots rather than tenths, and there is no true
//! heading. The 20-bit `radio_status` follows the Type 18 precedent
//! (selector bit plus 19-bit communication state).

use crate::shared_types::{cog_deg, lat_deg, lon_deg, sentinel, whole_u16};
use crate::{AisError, BitReader};

/// Minimum valid payload size for Type 9 (ITU-R M.1371-5 Annex 8
/// §3.7, Table 59).
pub const SAR_AIRCRAFT_POSITION_REPORT_BITS: usize = 168;

/// Source of the altitude value (Table 59, bit 134). Exhaustive: a
/// one-bit field cannot grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltitudeSensor {
    /// 0 — altitude from the GNSS receiver.
    Gnss,
    /// 1 — barometric altitude.
    Barometric,
}

/// Decoded SAR aircraft position report.
///
/// No heading, rate of turn or navigational status exists on this
/// message; the struct has no such fields. Not-available codes decode
/// to `None`, over-range codes pass through as the value they name
/// (ADR-0001; codes in [`sentinel`]).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
pub struct SarAircraftPositionReport {
    /// Maritime Mobile Service Identity.
    pub mmsi: u32,
    /// Altitude in metres. `None` on [`sentinel::ALTITUDE_NOT_AVAILABLE`]
    /// (4095); [`sentinel::ALTITUDE_OVER_RANGE`] (4094) means 4094 m or
    /// higher and is kept.
    pub altitude_m: Option<u16>,
    /// Speed over ground in **whole knots** (not 0.1 kn as on vessels).
    /// `None` on [`sentinel::AIRCRAFT_SOG_NOT_AVAILABLE`] (1023);
    /// [`sentinel::AIRCRAFT_SOG_OVER_RANGE`] (1022) means 1022 kn or
    /// higher and is kept.
    pub speed_over_ground: Option<u16>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes.
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees. `None` on sentinel `181°`.
    pub longitude_deg: Option<f64>,
    /// Latitude in signed decimal degrees. `None` on sentinel `91°`.
    pub latitude_deg: Option<f64>,
    /// Course over ground in degrees. `None` on sentinel `3600`.
    pub course_over_ground: Option<f32>,
    /// UTC second within the minute (0..=59); `60` = not available;
    /// `61..=63` carry positioning-system status (codes in
    /// [`sentinel`]). Kept as `u8` for full fidelity.
    pub timestamp: u8,
    /// Which sensor produced `altitude_m`.
    pub altitude_sensor: AltitudeSensor,
    /// Data Terminal Equipment flag: `false` = ready, `true` = not
    /// ready (the default on the wire).
    pub dte: bool,
    /// Assigned-mode flag (base station assigned this unit a schedule).
    pub assigned_flag: bool,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Radio status: the communication state selector (bit 148; `0` =
    /// SOTDMA, `1` = ITDMA) followed by the 19-bit communication state.
    /// 20 bits, like Type 18.
    pub radio_status: u32,
}

/// Decode a Type 9 SAR aircraft position report.
///
/// Lenient on longer payloads: bits past 168 are ignored.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 168`.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_sar_aircraft_position_report(
    bits: &[u8],
    total_bits: usize,
) -> Result<SarAircraftPositionReport, AisError> {
    if total_bits < SAR_AIRCRAFT_POSITION_REPORT_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;

    let altitude_m = whole_u16(r.u(12), sentinel::ALTITUDE_NOT_AVAILABLE);
    let speed_over_ground = whole_u16(r.u(10), sentinel::AIRCRAFT_SOG_NOT_AVAILABLE);
    let position_accuracy = r.b();
    let longitude_deg = lon_deg(r.i(28));
    let latitude_deg = lat_deg(r.i(27));
    let course_over_ground = cog_deg(r.u(12));
    let timestamp = (r.u(6) & 0x3F) as u8;
    let altitude_sensor = if r.b() {
        AltitudeSensor::Barometric
    } else {
        AltitudeSensor::Gnss
    };
    let _ = r.u(7); // spare
    let dte = r.b();
    let _ = r.u(3); // spare
    let assigned_flag = r.b();
    let raim = r.b();
    let radio_status = (r.u(20) & 0xF_FFFF) as u32;

    Ok(SarAircraftPositionReport {
        mmsi,
        altitude_m,
        speed_over_ground,
        position_accuracy,
        longitude_deg,
        latitude_deg,
        course_over_ground,
        timestamp,
        altitude_sensor,
        dte,
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
    use crate::testing::BitWriter;

    /// Every Table 59 field a test may want to vary. Spares are zero.
    #[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
    struct Fields {
        mmsi: u32,
        altitude: u16,
        sog: u16,
        pos_acc: bool,
        lon_raw: i64,
        lat_raw: i64,
        cog: u16,
        timestamp: u8,
        barometric: bool,
        dte: bool,
        assigned: bool,
        raim: bool,
        radio: u32,
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                mmsi: 111_000_001,
                altitude: 0,
                sog: 0,
                pos_acc: false,
                lon_raw: 0,
                lat_raw: 0,
                cog: 0,
                timestamp: 0,
                barometric: false,
                dte: false,
                assigned: false,
                raim: false,
                radio: 0,
            }
        }
    }

    /// Pack `f` into a 168-bit Type 9 payload (ITU-R M.1371-5 Table 59).
    fn build_type9(f: &Fields) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 9); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(f.mmsi));
        w.u(12, u64::from(f.altitude));
        w.u(10, u64::from(f.sog));
        w.b(f.pos_acc);
        w.i(28, f.lon_raw);
        w.i(27, f.lat_raw);
        w.u(12, u64::from(f.cog));
        w.u(6, u64::from(f.timestamp));
        w.b(f.barometric);
        w.u(7, 0); // spare
        w.b(f.dte);
        w.u(3, 0); // spare
        w.b(f.assigned);
        w.b(f.raim);
        w.u(20, u64::from(f.radio)); // selector + 19-bit state
        w.finish()
    }

    #[test]
    fn decodes_happy_path_with_flags() {
        let (bits, total) = build_type9(&Fields {
            mmsi: 111_265_591,
            altitude: 1500,
            sog: 120,
            pos_acc: true,
            lon_raw: 6_600_000,  // 11°E
            lat_raw: -2_880_000, // 4.8°S
            cog: 2705,           // 270.5°
            timestamp: 33,
            barometric: true,
            dte: true,
            assigned: true,
            raim: true,
            radio: 0xF_ABCD,
        });
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 111_265_591);
        assert_eq!(msg.altitude_m, Some(1500));
        assert_eq!(msg.speed_over_ground, Some(120));
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.unwrap() + 4.8).abs() < 1e-4);
        assert!((msg.course_over_ground.unwrap() - 270.5).abs() < 1e-4);
        assert_eq!(msg.timestamp, 33);
        assert_eq!(msg.altitude_sensor, AltitudeSensor::Barometric);
        assert!(msg.dte);
        assert!(msg.assigned_flag);
        assert!(msg.raim);
        assert_eq!(msg.radio_status, 0xF_ABCD);
    }

    #[test]
    fn not_available_sentinels_decode_to_none() {
        let (bits, total) = build_type9(&Fields {
            altitude: sentinel::ALTITUDE_NOT_AVAILABLE,
            sog: sentinel::AIRCRAFT_SOG_NOT_AVAILABLE,
            lon_raw: sentinel::LON_NOT_AVAILABLE,
            lat_raw: sentinel::LAT_NOT_AVAILABLE,
            cog: sentinel::COG_NOT_AVAILABLE,
            ..Fields::default()
        });
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.altitude_m, None);
        assert_eq!(msg.speed_over_ground, None);
        assert_eq!(msg.longitude_deg, None);
        assert_eq!(msg.latitude_deg, None);
        assert_eq!(msg.course_over_ground, None);
    }

    #[test]
    fn over_range_values_are_kept() {
        let (bits, total) = build_type9(&Fields {
            altitude: sentinel::ALTITUDE_OVER_RANGE,
            sog: sentinel::AIRCRAFT_SOG_OVER_RANGE,
            ..Fields::default()
        });
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.altitude_m, Some(4094));
        assert_eq!(msg.speed_over_ground, Some(1022));
    }

    #[test]
    fn altitude_sensor_bit_is_categorical() {
        let (bits, total) = build_type9(&Fields {
            barometric: false,
            ..Fields::default()
        });
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.altitude_sensor, AltitudeSensor::Gnss);

        let (bits, total) = build_type9(&Fields {
            barometric: true,
            ..Fields::default()
        });
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.altitude_sensor, AltitudeSensor::Barometric);
    }

    #[test]
    fn timestamp_codes_pass_raw() {
        for code in [
            sentinel::TIMESTAMP_NOT_AVAILABLE,
            sentinel::TIMESTAMP_MANUAL_INPUT,
            sentinel::TIMESTAMP_DEAD_RECKONING,
            sentinel::TIMESTAMP_INOPERATIVE,
        ] {
            let (bits, total) = build_type9(&Fields {
                timestamp: code,
                ..Fields::default()
            });
            let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
            assert_eq!(msg.timestamp, code);
        }
    }

    #[test]
    fn too_short_payload_is_rejected() {
        let (bits, _) = build_type9(&Fields::default());
        match decode_sar_aircraft_position_report(&bits, SAR_AIRCRAFT_POSITION_REPORT_BITS - 1) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    #[test]
    fn longer_payload_is_tolerated() {
        let (mut bits, _) = build_type9(&Fields {
            mmsi: 123,
            ..Fields::default()
        });
        bits.push(0xFF); // 8 trailing bits the decoder must ignore
        let msg = decode_sar_aircraft_position_report(&bits, 176).unwrap();
        assert_eq!(msg.mmsi, 123);
    }

    /// gpsd `test/sample.aivdm` T9-1 (BSD-2-Clause), values from its
    /// `.chk` file: altitude 15 m, SOG 0 kn, 11.8816°E 57.778455°N, COG 0,
    /// second 28, DTE ready, radio 0xC02A.
    #[test]
    fn gpsd_vector_t9_1() {
        let (bits, total) = crate::armor::decode(b"91b77=h3h00nHt0Q3r@@07000<0b", 0).unwrap();
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 111_265_591);
        assert_eq!(msg.altitude_m, Some(15));
        assert_eq!(msg.speed_over_ground, Some(0));
        assert!(!msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() - 11.8816).abs() < 1e-6);
        assert!((msg.latitude_deg.unwrap() - 57.778_455).abs() < 1e-6);
        assert!((msg.course_over_ground.unwrap() - 0.0).abs() < 1e-4);
        assert_eq!(msg.timestamp, 28);
        assert_eq!(msg.altitude_sensor, AltitudeSensor::Gnss);
        assert!(!msg.dte);
        assert!(!msg.assigned_flag);
        assert!(!msg.raim);
        assert_eq!(msg.radio_status, 0xC02A);
    }

    /// gpsd `test/sample.aivdm` T9-2 (BSD-2-Clause), values from its
    /// `.chk` file: altitude 303 m, SOG 42 kn, 6.27884°W 58.144°N,
    /// COG 154.5°, second 15, DTE not ready, radio 0x8270.
    #[test]
    fn gpsd_vector_t9_2() {
        let (bits, total) = crate::armor::decode(b"91b55wi;hbOS@OdQAC062Ch2089h", 0).unwrap();
        let msg = decode_sar_aircraft_position_report(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 111_232_511);
        assert_eq!(msg.altitude_m, Some(303));
        assert_eq!(msg.speed_over_ground, Some(42));
        assert!(!msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() + 6.278_843).abs() < 1e-6);
        assert!((msg.latitude_deg.unwrap() - 58.144).abs() < 1e-6);
        assert!((msg.course_over_ground.unwrap() - 154.5).abs() < 1e-4);
        assert_eq!(msg.timestamp, 15);
        assert_eq!(msg.altitude_sensor, AltitudeSensor::Gnss);
        assert!(msg.dte);
        assert!(!msg.assigned_flag);
        assert!(!msg.raim);
        assert_eq!(msg.radio_status, 0x8270);
    }
}
