//! Class B CS position report — AIS message Type 18.
//!
//! 168-bit payload (ITU-R M.1371-5 §5.3.18). Similar shape to
//! Types 1/2/3 but tailored for Class B transponders (smaller
//! vessels, voluntary carriage) — omits `navigation_status` and
//! `rate_of_turn`, adds Class-B-specific capability flags.

use crate::{AisError, BitReader};

/// Minimum valid payload size for Type 18 (ITU-R M.1371-5 §5.3.18).
pub const POSITION_REPORT_B_BITS: usize = 168;

/// Sentinels match the Class A position report (§5.3.1).
const SOG_NOT_AVAILABLE: u64 = 1023;
const LON_NOT_AVAILABLE: i64 = 181 * 600_000;
const LAT_NOT_AVAILABLE: i64 = 91 * 600_000;
const COG_NOT_AVAILABLE: u64 = 3600;
const HEADING_NOT_AVAILABLE: u64 = 511;
const MINUTES_FRAC_PER_DEGREE: f64 = 600_000.0;

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
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
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
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use crate::testing::BitWriter;

    #[allow(clippy::too_many_arguments)]
    fn build_prb(
        mmsi: u32,
        sog: u16,
        pos_acc: bool,
        lon_raw: i32,
        lat_raw: i32,
        cog: u16,
        heading: u16,
        timestamp: u8,
        flags: [bool; 7], // cs, display, dsc, band, msg22, assigned, raim
        radio: u32,
    ) -> (alloc::vec::Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 18); // msg_type = 18
        w.u(2, 0);
        w.u(30, u64::from(mmsi));
        w.u(8, 0); // reserved
        w.u(10, u64::from(sog));
        w.b(pos_acc);
        w.i(28, i64::from(lon_raw));
        w.i(27, i64::from(lat_raw));
        w.u(12, u64::from(cog));
        w.u(9, u64::from(heading));
        w.u(6, u64::from(timestamp));
        w.u(2, 0); // regional reserved
        for f in flags {
            w.b(f);
        }
        w.u(20, u64::from(radio));
        w.finish()
    }

    #[test]
    fn decodes_happy_path_with_flags() {
        let (bits, total) = build_prb(
            367_036_850,
            150,       // SOG = 15.0 kn
            true,      // pos acc
            6_600_000, // 11°E
            2_880_000, // 4.8°N
            900,       // COG = 90.0°
            90,        // heading 90°
            30,
            [true, true, false, true, false, false, true],
            0xA_BCDE,
        );
        let msg = decode_position_report_b(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 367_036_850);
        assert!((msg.speed_over_ground.unwrap() - 15.0).abs() < 1e-4);
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.unwrap() - 4.8).abs() < 1e-4);
        assert!((msg.course_over_ground.unwrap() - 90.0).abs() < 1e-4);
        assert_eq!(msg.true_heading, Some(90));
        assert_eq!(msg.timestamp, 30);
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
    fn sentinels_decode_to_none() {
        let (bits, total) = build_prb(
            1,
            SOG_NOT_AVAILABLE as u16,
            false,
            LON_NOT_AVAILABLE as i32,
            LAT_NOT_AVAILABLE as i32,
            COG_NOT_AVAILABLE as u16,
            HEADING_NOT_AVAILABLE as u16,
            60,
            [false; 7],
            0,
        );
        let msg = decode_position_report_b(&bits, total).unwrap();
        assert_eq!(msg.speed_over_ground, None);
        assert_eq!(msg.longitude_deg, None);
        assert_eq!(msg.latitude_deg, None);
        assert_eq!(msg.course_over_ground, None);
        assert_eq!(msg.true_heading, None);
        assert_eq!(msg.timestamp, 60);
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
