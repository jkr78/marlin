//! Class A static and voyage data — AIS message Type 5.
//!
//! 424-bit payload (ITU-R M.1371-5 §5.3.5). Spans multiple AIVDM
//! fragments in practice; caller is responsible for having
//! reassembled the bit stream before calling
//! [`decode_static_and_voyage_a`].

use alloc::string::String;

use crate::shared_types::{dim_u6, dim_u9, trim_ais_string, Dimensions, EpfdType};
use crate::{AisError, BitReader};

/// Minimum valid payload size for Type 5 (ITU-R M.1371-5 §5.3.5).
pub const STATIC_VOYAGE_A_BITS: usize = 424;

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
    /// `IMO` (International Maritime Organization) number,
    /// `1..=999_999_999`. `None` on sentinel `0` (not available).
    pub imo_number: Option<u32>,
    /// Vessel call sign (up to 7 characters). `None` on all-padding.
    pub call_sign: Option<String>,
    /// Vessel name (up to 20 characters). `None` on all-padding.
    pub vessel_name: Option<String>,
    /// Ship and cargo type — ITU-R M.1371-5 Table 53. `0` means "not
    /// available"; other values carry specific cargo / vessel-class
    /// meanings. Exposed as raw `u8` because the full table has 250+
    /// values and most callers use their own lookup.
    pub ship_type: u8,
    /// Vessel dimensions.
    pub dimensions: Dimensions,
    /// Electronic position-fixing device type.
    pub epfd: EpfdType,
    /// Estimated time of arrival (UTC). Individual fields may be
    /// `None` independently (month `0`, day `0`, hour `24`, minute
    /// `60` are sentinels).
    pub eta: Eta,
    /// Maximum present static draught in metres (1/10 m resolution).
    /// `None` on sentinel `0`.
    pub draught_m: Option<f32>,
    /// Destination (up to 20 characters). `None` on all-padding.
    pub destination: Option<String>,
    /// Data Terminal Equipment (DTE) flag: `false` = ready (normal
    /// operation), `true` = not ready.
    pub dte: bool,
}

/// ETA field, decoded with per-sub-field sentinels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Eta {
    /// Month of year (1..=12). `None` on sentinel `0`.
    pub month: Option<u8>,
    /// Day of month (1..=31). `None` on sentinel `0`.
    pub day: Option<u8>,
    /// Hour of day (0..=23). `None` on sentinel `24`.
    pub hour: Option<u8>,
    /// Minute of hour (0..=59). `None` on sentinel `60`.
    pub minute: Option<u8>,
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

/// Decode a Type 5 sentence from its 424-bit payload.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 424`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
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
    let imo_raw = r.u(30);
    let imo_number = if imo_raw == 0 {
        None
    } else {
        Some((imo_raw & 0xFFFF_FFFF) as u32)
    };
    let call_sign = trim_ais_string(r.string(7));
    let vessel_name = trim_ais_string(r.string(20));
    let ship_type = (r.u(8) & 0xFF) as u8;
    let dimensions = Dimensions {
        to_bow_m: dim_u9(r.u(9)),
        to_stern_m: dim_u9(r.u(9)),
        to_port_m: dim_u6(r.u(6)),
        to_starboard_m: dim_u6(r.u(6)),
    };
    let epfd = EpfdType::from_u4((r.u(4) & 0x0F) as u8);
    let eta = Eta {
        month: sentinel_u8(r.u(4), 0),
        day: sentinel_u8(r.u(5), 0),
        hour: sentinel_u8(r.u(5), 24),
        minute: sentinel_u8(r.u(6), 60),
    };
    let draught_raw = r.u(8);
    let draught_m = if draught_raw == 0 {
        None
    } else {
        Some((draught_raw as f32) / 10.0)
    };
    let destination = trim_ais_string(r.string(20));
    let dte = r.b();
    let _ = r.u(1); // spare

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

/// Helper: convert a raw small integer to `Option<u8>` with a given
/// sentinel value.
fn sentinel_u8(raw: u64, sentinel: u64) -> Option<u8> {
    if raw == sentinel {
        None
    } else {
        // Field widths 4/5/6 all fit in u8.
        #[allow(clippy::cast_possible_truncation)]
        Some((raw & 0xFF) as u8)
    }
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
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
mod tests {
    use super::*;
    use crate::testing::BitWriter;

    // Helper: write a 6-bit-ASCII string of exactly `n` chars,
    // padding with '@' (value 0) if the input is shorter.
    fn write_ais_str(w: &mut BitWriter, s: &[u8], chars: usize) {
        for i in 0..chars {
            let c = s.get(i).copied().unwrap_or(b'@');
            // Inverse of AIS table 47: find each byte's 6-bit value.
            // For ASCII 'A'-'Z' (65-90): value = 1..=26 = c - 64.
            // For '@' (64): value = 0.
            // For ' ' (32): value = 32.
            // For '0'-'9' (48-57): value = 48..=57 = c - 16? no...
            // The table wraps at 32: values 0..=31 are @ABC...Z[\]^_,
            // values 32..=63 are space-!"#$%&'()*+,-./0123456789:;<=>?.
            // Inverse: c = 64 + v for v < 32; c = v for v >= 32.
            let v = if c >= 64 { c - 64 } else { c };
            w.u(6, u64::from(v));
        }
    }

    #[test]
    fn decodes_happy_path() {
        let mut w = BitWriter::new();
        w.u(6, 5); // msg_type = 5
        w.u(2, 0); // repeat
        w.u(30, 123_456_789); // mmsi
        w.u(2, 2); // AisVersion::Itu1371v5
        w.u(30, 9_876_543); // imo_number
        write_ais_str(&mut w, b"ABC1234", 7); // call_sign
        write_ais_str(&mut w, b"MY VESSEL NAME", 20); // vessel_name (pads with @)
        w.u(8, 70); // ship_type = cargo (Table 53)
        w.u(9, 100); // to_bow
        w.u(9, 50); // to_stern
        w.u(6, 10); // to_port
        w.u(6, 15); // to_starboard
        w.u(4, 1); // epfd = GPS
        w.u(4, 6); // eta month
        w.u(5, 15); // eta day
        w.u(5, 12); // eta hour
        w.u(6, 30); // eta minute
        w.u(8, 75); // draught = 7.5 m
        write_ais_str(&mut w, b"HAMBURG", 20); // destination
        w.b(false); // dte = ready
        w.u(1, 0); // spare
        let (bits, total) = w.finish();
        assert_eq!(total, 424);

        let msg = decode_static_and_voyage_a(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 123_456_789);
        assert_eq!(msg.ais_version, AisVersion::Itu1371v5);
        assert_eq!(msg.imo_number, Some(9_876_543));
        assert_eq!(msg.call_sign.as_deref(), Some("ABC1234"));
        assert_eq!(msg.vessel_name.as_deref(), Some("MY VESSEL NAME"));
        assert_eq!(msg.ship_type, 70);
        assert_eq!(msg.dimensions.to_bow_m, Some(100));
        assert_eq!(msg.dimensions.to_stern_m, Some(50));
        assert_eq!(msg.dimensions.to_port_m, Some(10));
        assert_eq!(msg.dimensions.to_starboard_m, Some(15));
        assert_eq!(msg.epfd, EpfdType::Gps);
        assert_eq!(msg.eta.month, Some(6));
        assert_eq!(msg.eta.day, Some(15));
        assert_eq!(msg.eta.hour, Some(12));
        assert_eq!(msg.eta.minute, Some(30));
        assert!((msg.draught_m.unwrap() - 7.5).abs() < 1e-4);
        assert_eq!(msg.destination.as_deref(), Some("HAMBURG"));
        assert!(!msg.dte);
    }

    #[test]
    fn all_sentinels_decode_to_none() {
        let mut w = BitWriter::new();
        w.u(6, 5);
        w.u(2, 0);
        w.u(30, 1); // mmsi (not a sentinel — MMSI 0 would be malformed)
        w.u(2, 0); // AisVersion::Itu1371v1
        w.u(30, 0); // imo sentinel
        write_ais_str(&mut w, b"", 7); // all '@'
        write_ais_str(&mut w, b"", 20);
        w.u(8, 0); // ship_type
        w.u(9, 0); // dim sentinels
        w.u(9, 0);
        w.u(6, 0);
        w.u(6, 0);
        w.u(4, 0); // epfd = Undefined
        w.u(4, 0); // eta month sentinel
        w.u(5, 0); // eta day sentinel
        w.u(5, 24); // eta hour sentinel
        w.u(6, 60); // eta minute sentinel
        w.u(8, 0); // draught sentinel
        write_ais_str(&mut w, b"", 20); // destination
        w.b(true); // dte
        w.u(1, 0); // spare
        let (bits, total) = w.finish();

        let msg = decode_static_and_voyage_a(&bits, total).unwrap();
        assert_eq!(msg.imo_number, None);
        assert_eq!(msg.call_sign, None);
        assert_eq!(msg.vessel_name, None);
        assert_eq!(msg.ship_type, 0);
        assert_eq!(msg.dimensions.to_bow_m, None);
        assert_eq!(msg.dimensions.to_stern_m, None);
        assert_eq!(msg.dimensions.to_port_m, None);
        assert_eq!(msg.dimensions.to_starboard_m, None);
        assert_eq!(msg.epfd, EpfdType::Undefined);
        assert_eq!(msg.eta, Eta::default());
        assert_eq!(msg.draught_m, None);
        assert_eq!(msg.destination, None);
        assert!(msg.dte);
    }

    #[test]
    fn too_short_payload_is_rejected() {
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

    #[test]
    fn epfd_type_maps_all_known_values() {
        for (code, expected) in [
            (0u8, EpfdType::Undefined),
            (1, EpfdType::Gps),
            (2, EpfdType::Glonass),
            (3, EpfdType::CombinedGpsGlonass),
            (4, EpfdType::LoranC),
            (5, EpfdType::Chayka),
            (6, EpfdType::IntegratedNavigation),
            (7, EpfdType::Surveyed),
            (8, EpfdType::Galileo),
            (15, EpfdType::InternalGnss),
        ] {
            assert_eq!(EpfdType::from_u4(code), expected);
        }
        for code in 9u8..=14 {
            assert_eq!(EpfdType::from_u4(code), EpfdType::Reserved(code));
        }
    }

    #[test]
    fn ais_string_trims_at_padding_and_spaces() {
        assert_eq!(
            trim_ais_string(String::from("ABC@@@@")).as_deref(),
            Some("ABC")
        );
        assert_eq!(
            trim_ais_string(String::from("NAME   ")).as_deref(),
            Some("NAME")
        );
        assert_eq!(trim_ais_string(String::from("@@@@@@@")), None);
        assert_eq!(trim_ais_string(String::new()), None);
    }
}
