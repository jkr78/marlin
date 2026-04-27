//! Class B extended position report — AIS message Type 19.
//!
//! 312-bit payload (ITU-R M.1371-5 §5.3.19). Like Type 18 for the
//! position portion, with the Class A static-data tail (vessel name,
//! ship type, dimensions, EPFD) appended. Rarely seen on Class B
//! feeds — most Class B units transmit Type 18 + Type 24 Part A/B
//! instead.

use alloc::string::String;

use crate::shared_types::{dim_u6, dim_u9, trim_ais_string, Dimensions, EpfdType};
use crate::{AisError, BitReader};

/// Minimum valid payload size for Type 19 (ITU-R M.1371-5 §5.3.19).
pub const EXTENDED_POSITION_REPORT_B_BITS: usize = 312;

/// Sentinels match the Class A position report (§5.3.1).
const SOG_NOT_AVAILABLE: u64 = 1023;
const LON_NOT_AVAILABLE: i64 = 181 * 600_000;
const LAT_NOT_AVAILABLE: i64 = 91 * 600_000;
const COG_NOT_AVAILABLE: u64 = 3600;
const HEADING_NOT_AVAILABLE: u64 = 511;
const MINUTES_FRAC_PER_DEGREE: f64 = 600_000.0;

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
    /// Speed over ground in knots. `None` on sentinel `1023`.
    pub speed_over_ground: Option<f32>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes.
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees. `None` on sentinel `181°`.
    pub longitude_deg: Option<f64>,
    /// Latitude in signed decimal degrees. `None` on sentinel `91°`.
    pub latitude_deg: Option<f64>,
    /// Course over ground in degrees. `None` on sentinel `3600`.
    pub course_over_ground: Option<f32>,
    /// True heading in degrees (0..359). `None` on sentinel `511`.
    pub true_heading: Option<u16>,
    /// UTC second within the minute (0..=59); `60` = not available;
    /// `61..=63` carry positioning-system status. Kept as `u8` for
    /// full fidelity.
    pub timestamp: u8,
    /// Vessel name (up to 20 characters). `None` on all-padding.
    pub vessel_name: Option<String>,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53 raw value.
    pub ship_type: u8,
    /// Vessel dimensions.
    pub dimensions: Dimensions,
    /// Electronic position-fixing device type.
    pub epfd: EpfdType,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Data Terminal Equipment flag: `false` = ready, `true` = not ready.
    pub dte: bool,
    /// Assigned-mode flag (base station assigned this unit a schedule).
    pub assigned_flag: bool,
}

/// Decode a Type 19 Class B extended position report.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 312`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
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

    let sog_raw = r.u(10);
    let speed_over_ground = if sog_raw == SOG_NOT_AVAILABLE {
        None
    } else {
        Some((sog_raw as f32) / 10.0)
    };

    let position_accuracy = r.b();

    let lon_raw = r.i(28);
    let longitude_deg = if lon_raw == LON_NOT_AVAILABLE {
        None
    } else {
        Some((lon_raw as f64) / MINUTES_FRAC_PER_DEGREE)
    };
    let lat_raw = r.i(27);
    let latitude_deg = if lat_raw == LAT_NOT_AVAILABLE {
        None
    } else {
        Some((lat_raw as f64) / MINUTES_FRAC_PER_DEGREE)
    };

    let cog_raw = r.u(12);
    let course_over_ground = if cog_raw == COG_NOT_AVAILABLE {
        None
    } else {
        Some((cog_raw as f32) / 10.0)
    };

    let heading_raw = r.u(9);
    let true_heading = if heading_raw == HEADING_NOT_AVAILABLE {
        None
    } else {
        Some((heading_raw & 0x1FF) as u16)
    };

    let timestamp = (r.u(6) & 0x3F) as u8;
    let _ = r.u(4); // regional reserved

    let vessel_name = trim_ais_string(r.string(20));
    let ship_type = (r.u(8) & 0xFF) as u8;
    let dimensions = Dimensions {
        to_bow_m: dim_u9(r.u(9)),
        to_stern_m: dim_u9(r.u(9)),
        to_port_m: dim_u6(r.u(6)),
        to_starboard_m: dim_u6(r.u(6)),
    };
    let epfd = EpfdType::from_u4((r.u(4) & 0x0F) as u8);
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
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
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

    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn build_t19(
        mmsi: u32,
        sog: u16,
        pos_acc: bool,
        lon_raw: i32,
        lat_raw: i32,
        cog: u16,
        heading: u16,
        timestamp: u8,
        name: &[u8],
        ship_type: u8,
        bow: u16,
        stern: u16,
        port: u8,
        starboard: u8,
        epfd: u8,
        raim: bool,
        dte: bool,
        assigned: bool,
    ) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 19); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(mmsi));
        w.u(8, 0); // reserved
        w.u(10, u64::from(sog));
        w.b(pos_acc);
        w.i(28, i64::from(lon_raw));
        w.i(27, i64::from(lat_raw));
        w.u(12, u64::from(cog));
        w.u(9, u64::from(heading));
        w.u(6, u64::from(timestamp));
        w.u(4, 0); // regional reserved
        write_ais_str(&mut w, name, 20);
        w.u(8, u64::from(ship_type));
        w.u(9, u64::from(bow));
        w.u(9, u64::from(stern));
        w.u(6, u64::from(port));
        w.u(6, u64::from(starboard));
        w.u(4, u64::from(epfd));
        w.b(raim);
        w.b(dte);
        w.b(assigned);
        w.u(4, 0); // spare
        w.finish()
    }

    #[test]
    fn decodes_happy_path() {
        let (bits, total) = build_t19(
            369_493_000,
            150,       // 15.0 kn
            true,      // pos acc
            6_600_000, // 11°E
            2_880_000, // 4.8°N
            900,       // COG 90.0°
            90,
            30,
            b"VESSEL NAME",
            37,
            30,
            10,
            5,
            3,
            1, // EPFD GPS
            true,
            false,
            false,
        );
        let msg = decode_extended_position_report_b(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 369_493_000);
        assert!((msg.speed_over_ground.unwrap() - 15.0).abs() < 1e-4);
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.unwrap() - 4.8).abs() < 1e-4);
        assert!((msg.course_over_ground.unwrap() - 90.0).abs() < 1e-4);
        assert_eq!(msg.true_heading, Some(90));
        assert_eq!(msg.timestamp, 30);
        assert_eq!(msg.vessel_name.as_deref(), Some("VESSEL NAME"));
        assert_eq!(msg.ship_type, 37);
        assert_eq!(msg.dimensions.to_bow_m, Some(30));
        assert_eq!(msg.dimensions.to_stern_m, Some(10));
        assert_eq!(msg.dimensions.to_port_m, Some(5));
        assert_eq!(msg.dimensions.to_starboard_m, Some(3));
        assert_eq!(msg.epfd, EpfdType::Gps);
        assert!(msg.raim);
        assert!(!msg.dte);
        assert!(!msg.assigned_flag);
    }

    #[test]
    fn sentinels_decode_to_none() {
        let (bits, total) = build_t19(
            1,
            SOG_NOT_AVAILABLE as u16,
            false,
            LON_NOT_AVAILABLE as i32,
            LAT_NOT_AVAILABLE as i32,
            COG_NOT_AVAILABLE as u16,
            HEADING_NOT_AVAILABLE as u16,
            60,
            b"",
            0,
            0,
            0,
            0,
            0,
            0,
            false,
            false,
            false,
        );
        let msg = decode_extended_position_report_b(&bits, total).unwrap();
        assert_eq!(msg.speed_over_ground, None);
        assert_eq!(msg.longitude_deg, None);
        assert_eq!(msg.latitude_deg, None);
        assert_eq!(msg.course_over_ground, None);
        assert_eq!(msg.true_heading, None);
        assert_eq!(msg.timestamp, 60);
        assert_eq!(msg.vessel_name, None);
        assert_eq!(msg.dimensions.to_bow_m, None);
        assert_eq!(msg.dimensions.to_stern_m, None);
        assert_eq!(msg.dimensions.to_port_m, None);
        assert_eq!(msg.dimensions.to_starboard_m, None);
        assert_eq!(msg.epfd, EpfdType::Undefined);
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
            let (bits, total) = build_t19(
                1, 0, false, 0, 0, 0, 0, 0, b"X", 0, 0, 0, 0, 0, 0, raim, dte, assigned,
            );
            let msg = decode_extended_position_report_b(&bits, total).unwrap();
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
