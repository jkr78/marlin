//! Aid-to-navigation report — AIS message Type 21.
//!
//! 272–360-bit payload (ITU-R M.1371-5 Annex 8 §3.19, Tables 73 and
//! 74). The crate's first variable-length decoder: a fixed 272-bit part
//! carries a 20-character name, and an optional extension of up to 14
//! more characters follows, padded with spare bits to a byte boundary.
//! The extension length is derived from the payload length (gpsd's
//! rule): `(total_bits − 272) / 6` characters, capped at 14, remainder
//! bits ignored. Payloads longer than 360 bits are tolerated (the
//! crate's `<` floor policy); shorter than 272 are rejected.

use alloc::string::String;

use crate::shared_types::{lat_deg, lon_deg, read_dimensions, trim_ais_string};
use crate::{AisError, BitReader, Dimensions, EpfdType};

/// Minimum valid payload size for Type 21: the fixed part without a
/// name extension (ITU-R M.1371-5 Annex 8 §3.19, Table 73).
pub const AID_TO_NAVIGATION_REPORT_BITS: usize = 272;

/// Characters in the fixed-part name field (120 bits).
const NAME_CHARS: usize = 20;

/// Maximum characters in the name extension (84 bits; Table 73).
const NAME_EXTENSION_MAX_CHARS: usize = 14;

/// Type of aid to navigation (ITU-R M.1371-5 Table 74). Exhaustive: all
/// 32 codes of the 5-bit field are named, so there is no `Reserved`
/// variant and the field cannot grow.
///
/// Codes 5–19 are the fixed AtoN (`is_fixed`), 20–31 the floating AtoN
/// (`is_floating`); 0–4 are neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtonType {
    /// 0 — default, type of AtoN not specified.
    NotSpecified,
    /// 1 — reference point.
    ReferencePoint,
    /// 2 — RACON (radar transponder marking a navigation hazard).
    Racon,
    /// 3 — fixed structure off-shore, such as oil platforms and wind farms.
    FixedStructureOffshore,
    /// 4 — emergency wreck marking buoy.
    EmergencyWreckMarkingBuoy,
    /// 5 — light, without sectors.
    LightWithoutSectors,
    /// 6 — light, with sectors.
    LightWithSectors,
    /// 7 — leading light front.
    LeadingLightFront,
    /// 8 — leading light rear.
    LeadingLightRear,
    /// 9 — beacon, cardinal N.
    BeaconCardinalNorth,
    /// 10 — beacon, cardinal E.
    BeaconCardinalEast,
    /// 11 — beacon, cardinal S.
    BeaconCardinalSouth,
    /// 12 — beacon, cardinal W.
    BeaconCardinalWest,
    /// 13 — beacon, port hand.
    BeaconPortHand,
    /// 14 — beacon, starboard hand.
    BeaconStarboardHand,
    /// 15 — beacon, preferred channel port hand.
    BeaconPreferredChannelPortHand,
    /// 16 — beacon, preferred channel starboard hand.
    BeaconPreferredChannelStarboardHand,
    /// 17 — beacon, isolated danger.
    BeaconIsolatedDanger,
    /// 18 — beacon, safe water.
    BeaconSafeWater,
    /// 19 — beacon, special mark.
    BeaconSpecialMark,
    /// 20 — cardinal mark N.
    CardinalMarkNorth,
    /// 21 — cardinal mark E.
    CardinalMarkEast,
    /// 22 — cardinal mark S.
    CardinalMarkSouth,
    /// 23 — cardinal mark W.
    CardinalMarkWest,
    /// 24 — port hand mark.
    PortHandMark,
    /// 25 — starboard hand mark.
    StarboardHandMark,
    /// 26 — preferred channel port hand.
    PreferredChannelPortHand,
    /// 27 — preferred channel starboard hand.
    PreferredChannelStarboardHand,
    /// 28 — isolated danger.
    IsolatedDanger,
    /// 29 — safe water.
    SafeWater,
    /// 30 — special mark.
    SpecialMark,
    /// 31 — light vessel / LANBY / rigs.
    LightVessel,
}

impl AtonType {
    /// Decode the 5-bit wire code. Only the low five bits of `v` are
    /// read, so every input maps to a named variant.
    pub(crate) fn from_u5(v: u8) -> Self {
        match v & 0x1F {
            1 => Self::ReferencePoint,
            2 => Self::Racon,
            3 => Self::FixedStructureOffshore,
            4 => Self::EmergencyWreckMarkingBuoy,
            5 => Self::LightWithoutSectors,
            6 => Self::LightWithSectors,
            7 => Self::LeadingLightFront,
            8 => Self::LeadingLightRear,
            9 => Self::BeaconCardinalNorth,
            10 => Self::BeaconCardinalEast,
            11 => Self::BeaconCardinalSouth,
            12 => Self::BeaconCardinalWest,
            13 => Self::BeaconPortHand,
            14 => Self::BeaconStarboardHand,
            15 => Self::BeaconPreferredChannelPortHand,
            16 => Self::BeaconPreferredChannelStarboardHand,
            17 => Self::BeaconIsolatedDanger,
            18 => Self::BeaconSafeWater,
            19 => Self::BeaconSpecialMark,
            20 => Self::CardinalMarkNorth,
            21 => Self::CardinalMarkEast,
            22 => Self::CardinalMarkSouth,
            23 => Self::CardinalMarkWest,
            24 => Self::PortHandMark,
            25 => Self::StarboardHandMark,
            26 => Self::PreferredChannelPortHand,
            27 => Self::PreferredChannelStarboardHand,
            28 => Self::IsolatedDanger,
            29 => Self::SafeWater,
            30 => Self::SpecialMark,
            31 => Self::LightVessel,
            // Code 0; after the 5-bit mask nothing else reaches here.
            _ => Self::NotSpecified,
        }
    }

    /// The 5-bit wire code this variant was decoded from (Table 74).
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::NotSpecified => 0,
            Self::ReferencePoint => 1,
            Self::Racon => 2,
            Self::FixedStructureOffshore => 3,
            Self::EmergencyWreckMarkingBuoy => 4,
            Self::LightWithoutSectors => 5,
            Self::LightWithSectors => 6,
            Self::LeadingLightFront => 7,
            Self::LeadingLightRear => 8,
            Self::BeaconCardinalNorth => 9,
            Self::BeaconCardinalEast => 10,
            Self::BeaconCardinalSouth => 11,
            Self::BeaconCardinalWest => 12,
            Self::BeaconPortHand => 13,
            Self::BeaconStarboardHand => 14,
            Self::BeaconPreferredChannelPortHand => 15,
            Self::BeaconPreferredChannelStarboardHand => 16,
            Self::BeaconIsolatedDanger => 17,
            Self::BeaconSafeWater => 18,
            Self::BeaconSpecialMark => 19,
            Self::CardinalMarkNorth => 20,
            Self::CardinalMarkEast => 21,
            Self::CardinalMarkSouth => 22,
            Self::CardinalMarkWest => 23,
            Self::PortHandMark => 24,
            Self::StarboardHandMark => 25,
            Self::PreferredChannelPortHand => 26,
            Self::PreferredChannelStarboardHand => 27,
            Self::IsolatedDanger => 28,
            Self::SafeWater => 29,
            Self::SpecialMark => 30,
            Self::LightVessel => 31,
        }
    }

    /// Codes 5–19: Table 74's "fixed AtoN" group (lights and beacons).
    #[must_use]
    pub const fn is_fixed(self) -> bool {
        matches!(self.code(), 5..=19)
    }

    /// Codes 20–31: Table 74's "floating AtoN" group (marks and light
    /// vessels). [`AidToNavigationReport::off_position`] is meaningful
    /// only when this is `true`.
    #[must_use]
    pub const fn is_floating(self) -> bool {
        matches!(self.code(), 20..=31)
    }
}

/// Decoded aid-to-navigation report.
///
/// Not-available codes decode to `None`, over-range codes pass through
/// as the value they name (ADR-0001; codes in [`crate::sentinel`]). The
/// flags and the 8-bit status are exposed as the wire carries them.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
pub struct AidToNavigationReport {
    /// Maritime Mobile Service Identity. AtoN use the `99MIDxxxx` form.
    pub mmsi: u32,
    /// Type of aid to navigation (Table 74).
    pub aton_type: AtonType,
    /// The 20-character name joined with the optional extension (up to
    /// 14 more characters), then trimmed of trailing `@` and spaces.
    /// `None` when the result is all padding. An `@` inside the name is
    /// kept: only the tail is trimmed.
    pub name: Option<String>,
    /// Position accuracy flag: `true` for DGNSS-corrected fixes.
    pub position_accuracy: bool,
    /// Longitude in signed decimal degrees. `None` on sentinel `181°`.
    pub longitude_deg: Option<f64>,
    /// Latitude in signed decimal degrees. `None` on sentinel `91°`.
    pub latitude_deg: Option<f64>,
    /// Extent from the reported position. All `None` for virtual AtoN
    /// and reference points (ITU: A = B = C = D = 0). 511 m / 63 m mean
    /// "or greater" and are kept.
    pub dimensions: Dimensions,
    /// Electronic position-fixing device type.
    pub epfd: EpfdType,
    /// UTC second within the minute (0..=59); `60` = not available;
    /// `61..=63` carry positioning-system status (codes in
    /// [`crate::sentinel`]). Kept as `u8` for full fidelity.
    pub timestamp: u8,
    /// Off-position indicator. Floating AtoN only
    /// ([`AtonType::is_floating`]); ITU: valid only when
    /// `timestamp <= 59`.
    pub off_position: bool,
    /// Raw 8-bit AtoN status. ITU: "reserved for the indication of the
    /// AtoN status"; the layout is IEC 62288 Annex L, not ITU text.
    pub aton_status: u8,
    /// RAIM (Receiver Autonomous Integrity Monitoring) flag.
    pub raim: bool,
    /// Virtual AtoN flag: `true` when the aid does not physically exist.
    pub virtual_aton: bool,
    /// Assigned-mode flag (base station assigned this unit a schedule).
    pub assigned_flag: bool,
}

/// Decode a Type 21 aid-to-navigation report.
///
/// The name extension holds `min(14, (total_bits − 272) / 6)`
/// characters; any remaining bits are the byte-alignment spare and are
/// ignored. Payloads longer than the 360-bit Table 73 maximum are
/// tolerated and read as a 14-character extension.
///
/// # Errors
///
/// [`AisError::PayloadTooShort`] if `total_bits < 272`.
#[allow(clippy::cast_possible_truncation)] // every narrowing is masked to its field width
pub fn decode_aid_to_navigation_report(
    bits: &[u8],
    total_bits: usize,
) -> Result<AidToNavigationReport, AisError> {
    if total_bits < AID_TO_NAVIGATION_REPORT_BITS {
        return Err(AisError::PayloadTooShort);
    }
    let mut r = BitReader::new(bits, total_bits);
    let _ = r.u(6); // msg_type
    let _ = r.u(2); // repeat
    let mmsi = (r.u(30) & 0xFFFF_FFFF) as u32;
    let aton_type = AtonType::from_u5((r.u(5) & 0x1F) as u8);
    let mut name = r.string(NAME_CHARS);
    let position_accuracy = r.b();
    let longitude_deg = lon_deg(r.i(28));
    let latitude_deg = lat_deg(r.i(27));
    let dimensions = read_dimensions(&mut r);
    let epfd = EpfdType::from_u4((r.u(4) & 0x0F) as u8);
    let timestamp = (r.u(6) & 0x3F) as u8;
    let off_position = r.b();
    let aton_status = (r.u(8) & 0xFF) as u8;
    let raim = r.b();
    let virtual_aton = r.b();
    let assigned_flag = r.b();
    let _ = r.b(); // spare

    // Cannot underflow: the floor check above guarantees total_bits >= 272.
    let extension_chars =
        ((total_bits - AID_TO_NAVIGATION_REPORT_BITS) / 6).min(NAME_EXTENSION_MAX_CHARS);
    name.push_str(&r.string(extension_chars));
    let name = trim_ais_string(name);

    Ok(AidToNavigationReport {
        mmsi,
        aton_type,
        name,
        position_accuracy,
        longitude_deg,
        latitude_deg,
        dimensions,
        epfd,
        timestamp,
        off_position,
        aton_status,
        raim,
        virtual_aton,
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
    use alloc::vec::Vec;

    use super::*;
    use crate::shared_types::sentinel;
    use crate::testing::{build_aivdm, write_ais_str, BitWriter};
    use crate::{AisMessageBody, Parser};

    /// Every Table 73 field a test may want to vary. The spare bit is
    /// zero. `name` is the 20-character fixed field (`@`-padded when
    /// shorter); `extension` is written as exactly its own length in
    /// characters, followed by `tail_spare_bits` zero bits.
    #[allow(clippy::struct_excessive_bools)] // flags are the wire-format reality
    struct Fields {
        mmsi: u32,
        aton_type: u8,
        name: &'static [u8],
        extension: &'static [u8],
        tail_spare_bits: usize,
        pos_acc: bool,
        lon_raw: i64,
        lat_raw: i64,
        to_bow: u16,
        to_stern: u16,
        to_port: u8,
        to_starboard: u8,
        epfd: u8,
        timestamp: u8,
        off_position: bool,
        aton_status: u8,
        raim: bool,
        virtual_aton: bool,
        assigned: bool,
    }

    impl Default for Fields {
        fn default() -> Self {
            Self {
                mmsi: 992_000_001,
                aton_type: 0,
                name: b"",
                extension: b"",
                tail_spare_bits: 0,
                pos_acc: false,
                lon_raw: 0,
                lat_raw: 0,
                to_bow: 0,
                to_stern: 0,
                to_port: 0,
                to_starboard: 0,
                epfd: 0,
                timestamp: 0,
                off_position: false,
                aton_status: 0,
                raim: false,
                virtual_aton: false,
                assigned: false,
            }
        }
    }

    /// Pack `f` into a Type 21 payload (ITU-R M.1371-5 Table 73):
    /// 272 fixed bits, then the extension characters and tail spare.
    fn build_type21(f: &Fields) -> (Vec<u8>, usize) {
        let mut w = BitWriter::new();
        w.u(6, 21); // msg_type
        w.u(2, 0); // repeat
        w.u(30, u64::from(f.mmsi));
        w.u(5, u64::from(f.aton_type));
        write_ais_str(&mut w, f.name, NAME_CHARS);
        w.b(f.pos_acc);
        w.i(28, f.lon_raw);
        w.i(27, f.lat_raw);
        w.u(9, u64::from(f.to_bow));
        w.u(9, u64::from(f.to_stern));
        w.u(6, u64::from(f.to_port));
        w.u(6, u64::from(f.to_starboard));
        w.u(4, u64::from(f.epfd));
        w.u(6, u64::from(f.timestamp));
        w.b(f.off_position);
        w.u(8, u64::from(f.aton_status));
        w.b(f.raim);
        w.b(f.virtual_aton);
        w.b(f.assigned);
        w.b(false); // spare
        write_ais_str(&mut w, f.extension, f.extension.len());
        w.u(f.tail_spare_bits, 0);
        w.finish()
    }

    /// Spare bits that pad a `k`-character extension to a byte
    /// boundary: 272 is byte-aligned, so `(8 − 6k mod 8) mod 8`.
    fn alignment_spare(k: usize) -> usize {
        (8 - (6 * k) % 8) % 8
    }

    #[test]
    fn decodes_minimal_272_bits() {
        let (bits, total) = build_type21(&Fields {
            mmsi: 992_471_234,
            aton_type: 24, // port hand mark
            name: b"RED BUOY 7",
            pos_acc: true,
            lon_raw: 6_600_000,  // 11°E
            lat_raw: -2_880_000, // 4.8°S
            to_bow: 3,
            to_stern: 4,
            to_port: 1,
            to_starboard: 2,
            epfd: 1, // GPS
            timestamp: 42,
            off_position: true,
            aton_status: 0xA5,
            raim: true,
            virtual_aton: false,
            assigned: true,
            ..Fields::default()
        });
        assert_eq!(total, AID_TO_NAVIGATION_REPORT_BITS);
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert_eq!(msg.mmsi, 992_471_234);
        assert_eq!(msg.aton_type, AtonType::PortHandMark);
        assert_eq!(msg.name.as_deref(), Some("RED BUOY 7"));
        assert!(msg.position_accuracy);
        assert!((msg.longitude_deg.unwrap() - 11.0).abs() < 1e-4);
        assert!((msg.latitude_deg.unwrap() + 4.8).abs() < 1e-4);
        assert_eq!(
            msg.dimensions,
            Dimensions {
                to_bow_m: Some(3),
                to_stern_m: Some(4),
                to_port_m: Some(1),
                to_starboard_m: Some(2),
            }
        );
        assert_eq!(msg.epfd, EpfdType::Gps);
        assert_eq!(msg.timestamp, 42);
        assert!(msg.off_position);
        assert_eq!(msg.aton_status, 0xA5);
        assert!(msg.raim);
        assert!(!msg.virtual_aton);
        assert!(msg.assigned_flag);
    }

    #[test]
    fn extension_1_2_14_chars_with_alignment_spare() {
        // k = 1, 2 and 14 (the Table 73 maximum) with their 2 / 4 / 4
        // alignment spare bits give 280, 288 and 360-bit payloads.
        let cases: [(&[u8], usize); 3] = [(b"A", 280), (b"AB", 288), (b"ABCDEFGHIJKLMN", 360)];
        for (extension, expected_total) in cases {
            let (bits, total) = build_type21(&Fields {
                name: b"TWENTY CHARACTER NAM",
                extension,
                tail_spare_bits: alignment_spare(extension.len()),
                ..Fields::default()
            });
            assert_eq!(total, expected_total);
            let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
            let mut expected = String::from("TWENTY CHARACTER NAM");
            expected.push_str(core::str::from_utf8(extension).unwrap());
            assert_eq!(msg.name.as_deref(), Some(expected.as_str()));
        }
    }

    #[test]
    fn extension_296_bits_three_vs_four_chars() {
        // Three characters plus 6 spare bits and four characters plus
        // none are both 296 bits; length alone cannot separate them,
        // so the decoder reads four and relies on `@` trimming.
        let (bits_3, total_3) = build_type21(&Fields {
            name: b"TWENTY CHARACTER NAM",
            extension: b"XYZ",
            tail_spare_bits: alignment_spare(3),
            ..Fields::default()
        });
        let (bits_4, total_4) = build_type21(&Fields {
            name: b"TWENTY CHARACTER NAM",
            extension: b"XYZ@",
            tail_spare_bits: alignment_spare(4),
            ..Fields::default()
        });
        assert_eq!(total_3, 296);
        assert_eq!(total_4, 296);
        let msg_3 = decode_aid_to_navigation_report(&bits_3, total_3).unwrap();
        let msg_4 = decode_aid_to_navigation_report(&bits_4, total_4).unwrap();
        assert_eq!(msg_3.name.as_deref(), Some("TWENTY CHARACTER NAMXYZ"));
        assert_eq!(msg_3.name, msg_4.name);
    }

    #[test]
    fn extension_capped_at_14_chars_and_overlong_payload_tolerated() {
        // 16 extension characters make a 368-bit payload, over the
        // 360-bit Table 73 maximum. The decoder accepts it and reads
        // only the first 14 extension characters.
        let (bits, total) = build_type21(&Fields {
            name: b"TWENTY CHARACTER NAM",
            extension: b"ABCDEFGHIJKLMNOP",
            ..Fields::default()
        });
        assert_eq!(total, 368);
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert_eq!(
            msg.name.as_deref(),
            Some("TWENTY CHARACTER NAMABCDEFGHIJKLMN")
        );
    }

    #[test]
    fn name_all_padding_is_none() {
        let (bits, total) = build_type21(&Fields {
            name: b"",
            extension: b"@@",
            tail_spare_bits: alignment_spare(2),
            ..Fields::default()
        });
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert_eq!(msg.name, None);
    }

    #[test]
    fn virtual_aton_zero_dimensions_read_none() {
        let (bits, total) = build_type21(&Fields {
            aton_type: 30, // special mark
            virtual_aton: true,
            to_bow: 0,
            to_stern: 0,
            to_port: 0,
            to_starboard: 0,
            ..Fields::default()
        });
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert!(msg.virtual_aton);
        assert_eq!(msg.dimensions, Dimensions::default());
    }

    #[test]
    fn over_range_dimensions_are_kept() {
        let (bits, total) = build_type21(&Fields {
            to_bow: sentinel::DIMENSION_LONG_OVER_RANGE,
            to_stern: sentinel::DIMENSION_LONG_OVER_RANGE,
            to_port: sentinel::DIMENSION_SHORT_OVER_RANGE,
            to_starboard: sentinel::DIMENSION_SHORT_OVER_RANGE,
            ..Fields::default()
        });
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert_eq!(
            msg.dimensions,
            Dimensions {
                to_bow_m: Some(511),
                to_stern_m: Some(511),
                to_port_m: Some(63),
                to_starboard_m: Some(63),
            }
        );
    }

    #[test]
    fn not_available_position_decodes_to_none() {
        let (bits, total) = build_type21(&Fields {
            lon_raw: sentinel::LON_NOT_AVAILABLE,
            lat_raw: sentinel::LAT_NOT_AVAILABLE,
            ..Fields::default()
        });
        let msg = decode_aid_to_navigation_report(&bits, total).unwrap();
        assert_eq!(msg.longitude_deg, None);
        assert_eq!(msg.latitude_deg, None);
    }

    #[test]
    fn aton_type_all_32_codes_round_trip() {
        for code in 0u8..32 {
            let t = AtonType::from_u5(code);
            assert_eq!(t.code(), code);
            assert_eq!(t.is_fixed(), (5..=19).contains(&code), "code {code}");
            assert_eq!(t.is_floating(), (20..=31).contains(&code), "code {code}");
            if code < 5 {
                assert!(!t.is_fixed() && !t.is_floating(), "code {code}");
            }
        }
        // The mask drops bits above the field width.
        assert_eq!(AtonType::from_u5(0b10_0000 | 31), AtonType::LightVessel);
        assert_eq!(AtonType::NotSpecified.code(), 0);
        assert_eq!(AtonType::LightVessel.code(), 31);
    }

    #[test]
    fn too_short_payload_is_rejected() {
        let (bits, _) = build_type21(&Fields::default());
        match decode_aid_to_navigation_report(&bits, AID_TO_NAVIGATION_REPORT_BITS - 1) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }

        // T21-neg (nexus `ais_sample.nmea:8`): a 246-bit Type 21 whose
        // latitude would decode to 105.8°. Rejecting it is correct.
        let (bits, total) =
            crate::armor::decode(b"E>kb9II9S@0h:2ab@0b?ah00000Vc<4ONAEWCn@KP", 0).unwrap();
        assert_eq!(total, 246);
        match decode_aid_to_navigation_report(&bits, total) {
            Err(AisError::PayloadTooShort) => {}
            other => panic!("expected PayloadTooShort, got {other:?}"),
        }
    }

    /// gpsd `test/sample.aivdm` T21-1 (BSD-2-Clause) through the
    /// streaming parser: two fragments, 346 bits (fill 2 where 4 would
    /// byte-align). Values from its `.chk` file: aid 20, name
    /// `CHINA ROSE MURPHY EXPRESS ALERT` (20 + 12 characters, the
    /// extension ending in `@`), 122.698592°W 47.920618°N, dims
    /// 5/5/5/5, EPFD GPS, second 50, status 165.
    #[test]
    fn gpsd_vector_t21_1_two_fragments() {
        let mut p = Parser::streaming();
        let frag1 = build_aivdm(
            2,
            1,
            Some(5),
            Some(b'B'),
            b"E1mg=5J1T4W0h97aRh6ba84<h2d;W:Te=eLvH50```q",
            0,
        );
        let frag2 = build_aivdm(2, 2, Some(5), Some(b'B'), b":D44QDlp0C1DU00", 2);
        p.feed(&frag1);
        p.feed(b"\r\n");
        p.feed(&frag2);
        p.feed(b"\r\n");

        let msg = p.next_message().unwrap().unwrap();
        let AisMessageBody::Type21(aton) = msg.body else {
            panic!("expected Type21, got {:?}", msg.body);
        };
        assert_eq!(aton.mmsi, 123_456_789);
        assert_eq!(aton.aton_type, AtonType::CardinalMarkNorth);
        assert!(aton.aton_type.is_floating());
        assert_eq!(
            aton.name.as_deref(),
            Some("CHINA ROSE MURPHY EXPRESS ALERT")
        );
        assert!(!aton.position_accuracy);
        assert!((aton.longitude_deg.unwrap() + 122.698_592).abs() < 1e-6);
        assert!((aton.latitude_deg.unwrap() - 47.920_618).abs() < 1e-6);
        assert_eq!(
            aton.dimensions,
            Dimensions {
                to_bow_m: Some(5),
                to_stern_m: Some(5),
                to_port_m: Some(5),
                to_starboard_m: Some(5),
            }
        );
        assert_eq!(aton.epfd, EpfdType::Gps);
        assert_eq!(aton.timestamp, 50);
        assert!(!aton.off_position);
        assert_eq!(aton.aton_status, 165);
        assert!(!aton.raim);
        assert!(!aton.virtual_aton);
        assert!(!aton.assigned_flag);
        assert!(p.next_message().is_none());
    }

    /// gpsd `test/sample.aivdm` T21-2 (BSD-2-Clause): 368 bits, over
    /// the 360-bit maximum, with a 16-character all-`@` extension.
    ///
    /// Trailing-trim policy: the crate trims only the tail of the
    /// joined name, so the embedded `@` in the raw 20-character field
    /// survives and the name is `IBC G BUOY@?????????`. gpsd stops at
    /// the first `@` and reports `IBC G BUOY`; that is a presentation
    /// choice this decoder deliberately does not make (design spec §3.6).
    #[test]
    fn gpsd_vector_t21_2_overlong() {
        let mut p = Parser::streaming();
        let frag1 = build_aivdm(
            2,
            1,
            Some(8),
            Some(b'B'),
            b"E03l90w4Q1h3h1:WdPOwwwwwwwwlQdn`:e55020@@@gP0000000000000000",
            0,
        );
        let frag2 = build_aivdm(2, 2, Some(8), Some(b'B'), b"00", 4);
        p.feed(&frag1);
        p.feed(b"\r\n");
        p.feed(&frag2);
        p.feed(b"\r\n");

        let msg = p.next_message().unwrap().unwrap();
        let AisMessageBody::Type21(aton) = msg.body else {
            panic!("expected Type21, got {:?}", msg.body);
        };
        assert_eq!(aton.mmsi, 4_000_003);
        assert_eq!(aton.aton_type, AtonType::SpecialMark);
        assert_eq!(aton.name.as_deref(), Some("IBC G BUOY@?????????"));
        assert!(aton.position_accuracy);
        assert!((aton.longitude_deg.unwrap() - 126.572_226_7).abs() < 1e-6);
        assert!((aton.latitude_deg.unwrap() - 37.414_466_7).abs() < 1e-6);
        assert_eq!(
            aton.dimensions,
            Dimensions {
                to_bow_m: Some(2),
                to_stern_m: Some(2),
                to_port_m: Some(2),
                to_starboard_m: Some(2),
            }
        );
        assert_eq!(aton.epfd, EpfdType::Gps);
        assert_eq!(aton.timestamp, 31);
        assert_eq!(aton.aton_status, 0);
    }
}
