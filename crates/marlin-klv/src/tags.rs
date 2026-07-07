//! Table-driven scaled-tag machinery. Each `scaled_tags!` entry expands to the encode
//! arm, the decode dispatch arm, and the getter/setter accessor pair for one tag.
//! Adding a future ST 0601 tag = one struct field in `st0601.rs` + one entry here
//! (+ verify its formula against the standard — do NOT pattern-match new tags).

use crate::st0601::St0601;

use alloc::vec::Vec;

/// Metadata for one ST 0601 tag this crate decodes into a typed field: its wire
/// number, the [`St0601`] field base name (e.g. `"sensor_latitude"`, the
/// accessor name minus the `_degrees` / `raw_` affixes), and its engineering
/// unit. Sourced from the codec's own tag table, so it cannot drift from what
/// [`decode`](crate::decode) actually does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagInfo {
    /// ST 0601 tag number as it appears on the wire.
    pub number: u8,
    /// [`St0601`] field base name, e.g. `"sensor_latitude"`.
    pub name: &'static str,
    /// Engineering unit (`"degrees"`, `"meters"`, `"mps"`, `"microseconds"`),
    /// or `None` for unit-less tags such as the LS version.
    pub unit: Option<&'static str>,
}

macro_rules! scaled_tags {
    ($({
        tag: $tag:literal,
        field: $field:ident,
        wire_len: $len:literal,
        reader: $reader:path,
        getter: $getter:ident,
        setter: $setter:ident,
        to_units: $to_units:expr,
        from_units: $from_units:expr,
        unit: $unit:literal,
        doc: $doc:literal
    }),+ $(,)?) => {
        /// Every tag this crate decodes into a typed field, in ascending tag order.
        /// The scaled entries are generated from the table below (drift-free); the two
        /// framing tags handled outside the macro — Tag 2 (precision timestamp) and
        /// Tag 65 (LS version) — bracket them. Adding a scaled tag numbered below 65
        /// keeps the slice sorted automatically.
        pub const TAGS: &[TagInfo] = &[
            TagInfo { number: 2, name: "timestamp", unit: Some("microseconds") },
            $(
                TagInfo { number: $tag, name: stringify!($field), unit: Some($unit) },
            )+
            TagInfo { number: 65, name: "version", unit: None },
        ];

        /// Append every present scaled tag to `items`, in table (ascending tag) order.
        pub(crate) fn encode_scaled(set: &St0601, items: &mut Vec<u8>) {
            $(
                if let Some(raw) = set.$field {
                    items.push($tag);
                    crate::ber::ber_encode_len($len, items);
                    items.extend_from_slice(&raw.to_be_bytes());
                }
            )+
        }

        /// Try to decode `tag` as a scaled tag; `true` = consumed. A known tag with the
        /// wrong wire length is NOT consumed (tolerant decode: falls back to `unknown`).
        pub(crate) fn decode_scaled(tag: u8, value: &[u8], set: &mut St0601) -> bool {
            match tag {
                $(
                    $tag => match $reader(value) {
                        Some(raw) => {
                            set.$field = Some(raw);
                            true
                        }
                        None => false,
                    },
                )+
                _ => false,
            }
        }

        impl St0601 {
            $(
                #[doc = concat!("Engineering value of ST 0601 Tag ", stringify!($tag), ": ", $doc, ".")]
                #[doc = ""]
                #[doc = "`None` when the tag is absent; signed tags (i16/i32) also return `None` when the wire value is the reserved error indicator."]
                pub fn $getter(&self) -> Option<f64> {
                    self.$field.and_then(|raw| ($to_units)(raw))
                }

                #[doc = concat!("Set ST 0601 Tag ", stringify!($tag), " from ", $doc, ", clamped to the valid range (NaN clamps to the range minimum).")]
                pub fn $setter(&mut self, value: f64) {
                    self.$field = Some(($from_units)(value));
                }
            )+
        }
    };
}

scaled_tags! {
    {
        tag: 5, field: platform_heading, wire_len: 2, reader: crate::ber::read_u16,
        getter: platform_heading_degrees, setter: set_platform_heading_degrees,
        to_units: |c: u16| Some(crate::scale::u16_to_units(c, 360.0)),
        from_units: |v: f64| crate::scale::units_to_u16(v, 360.0),
        unit: "degrees",
        doc: "platform heading in degrees (0..360)"
    },
    {
        tag: 6, field: platform_pitch, wire_len: 2, reader: crate::ber::read_i16,
        getter: platform_pitch_degrees, setter: set_platform_pitch_degrees,
        to_units: |c: i16| crate::scale::i16_to_units(c, 20.0),
        from_units: |v: f64| crate::scale::units_to_i16(v, 20.0),
        unit: "degrees",
        doc: "platform pitch in degrees (-20..20)"
    },
    {
        tag: 7, field: platform_roll, wire_len: 2, reader: crate::ber::read_i16,
        getter: platform_roll_degrees, setter: set_platform_roll_degrees,
        to_units: |c: i16| crate::scale::i16_to_units(c, 50.0),
        from_units: |v: f64| crate::scale::units_to_i16(v, 50.0),
        unit: "degrees",
        doc: "platform roll in degrees (-50..50)"
    },
    {
        tag: 8, field: platform_true_airspeed, wire_len: 1, reader: crate::ber::read_u8,
        getter: platform_true_airspeed_mps, setter: set_platform_true_airspeed_mps,
        to_units: |c: u8| Some(f64::from(c)),
        from_units: |v: f64| crate::scale::units_to_u8(v),
        unit: "mps",
        doc: "platform true airspeed in m/s (0..255)"
    },
    {
        tag: 13, field: sensor_latitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: sensor_latitude_degrees, setter: set_sensor_latitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 90.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 90.0),
        unit: "degrees",
        doc: "sensor latitude in degrees WGS84 (-90..90)"
    },
    {
        tag: 14, field: sensor_longitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: sensor_longitude_degrees, setter: set_sensor_longitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 180.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 180.0),
        unit: "degrees",
        doc: "sensor longitude in degrees WGS84 (-180..180)"
    },
    {
        tag: 15, field: sensor_true_altitude, wire_len: 2, reader: crate::ber::read_u16,
        getter: sensor_true_altitude_meters, setter: set_sensor_true_altitude_meters,
        to_units: |c: u16| Some(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        from_units: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        unit: "meters",
        doc: "sensor true altitude in meters MSL (-900..19000)"
    },
    {
        tag: 16, field: sensor_horizontal_fov, wire_len: 2, reader: crate::ber::read_u16,
        getter: sensor_horizontal_fov_degrees, setter: set_sensor_horizontal_fov_degrees,
        to_units: |c: u16| Some(crate::scale::u16_to_units(c, 180.0)),
        from_units: |v: f64| crate::scale::units_to_u16(v, 180.0),
        unit: "degrees",
        doc: "sensor horizontal field of view in degrees (0..180)"
    },
    {
        tag: 17, field: sensor_vertical_fov, wire_len: 2, reader: crate::ber::read_u16,
        getter: sensor_vertical_fov_degrees, setter: set_sensor_vertical_fov_degrees,
        to_units: |c: u16| Some(crate::scale::u16_to_units(c, 180.0)),
        from_units: |v: f64| crate::scale::units_to_u16(v, 180.0),
        unit: "degrees",
        doc: "sensor vertical field of view in degrees (0..180)"
    },
    {
        tag: 18, field: sensor_relative_azimuth, wire_len: 4, reader: crate::ber::read_u32,
        getter: sensor_relative_azimuth_degrees, setter: set_sensor_relative_azimuth_degrees,
        to_units: |c: u32| Some(crate::scale::u32_to_units(c, 360.0)),
        from_units: |v: f64| crate::scale::units_to_u32(v, 360.0),
        unit: "degrees",
        doc: "sensor relative azimuth in degrees (0..360)"
    },
    {
        tag: 19, field: sensor_relative_elevation, wire_len: 4, reader: crate::ber::read_i32,
        getter: sensor_relative_elevation_degrees, setter: set_sensor_relative_elevation_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 180.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 180.0),
        unit: "degrees",
        doc: "sensor relative elevation in degrees (-180..180, negative = below horizon)"
    },
    {
        tag: 20, field: sensor_relative_roll, wire_len: 4, reader: crate::ber::read_u32,
        getter: sensor_relative_roll_degrees, setter: set_sensor_relative_roll_degrees,
        to_units: |c: u32| Some(crate::scale::u32_to_units(c, 360.0)),
        from_units: |v: f64| crate::scale::units_to_u32(v, 360.0),
        unit: "degrees",
        doc: "sensor relative roll in degrees (0..360, clockwise from behind camera)"
    },
    {
        tag: 21, field: slant_range, wire_len: 4, reader: crate::ber::read_u32,
        getter: slant_range_meters, setter: set_slant_range_meters,
        to_units: |c: u32| Some(crate::scale::u32_to_units(c, 5_000_000.0)),
        from_units: |v: f64| crate::scale::units_to_u32(v, 5_000_000.0),
        unit: "meters",
        doc: "slant range in meters (0..5000000)"
    },
    {
        tag: 22, field: target_width, wire_len: 2, reader: crate::ber::read_u16,
        getter: target_width_meters, setter: set_target_width_meters,
        to_units: |c: u16| Some(crate::scale::u16_to_units(c, 10_000.0)),
        from_units: |v: f64| crate::scale::units_to_u16(v, 10_000.0),
        unit: "meters",
        doc: "target width in meters (0..10000)"
    },
    {
        tag: 23, field: frame_center_latitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: frame_center_latitude_degrees, setter: set_frame_center_latitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 90.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 90.0),
        unit: "degrees",
        doc: "frame center latitude in degrees WGS84 (-90..90)"
    },
    {
        tag: 24, field: frame_center_longitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: frame_center_longitude_degrees, setter: set_frame_center_longitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 180.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 180.0),
        unit: "degrees",
        doc: "frame center longitude in degrees WGS84 (-180..180)"
    },
    {
        tag: 25, field: frame_center_elevation, wire_len: 2, reader: crate::ber::read_u16,
        getter: frame_center_elevation_meters, setter: set_frame_center_elevation_meters,
        to_units: |c: u16| Some(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        from_units: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        unit: "meters",
        doc: "frame center elevation in meters MSL (-900..19000)"
    },
    {
        tag: 40, field: target_location_latitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: target_location_latitude_degrees, setter: set_target_location_latitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 90.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 90.0),
        unit: "degrees",
        doc: "target location latitude in degrees WGS84 (-90..90)"
    },
    {
        tag: 41, field: target_location_longitude, wire_len: 4, reader: crate::ber::read_i32,
        getter: target_location_longitude_degrees, setter: set_target_location_longitude_degrees,
        to_units: |c: i32| crate::scale::i32_to_units(c, 180.0),
        from_units: |v: f64| crate::scale::units_to_i32(v, 180.0),
        unit: "degrees",
        doc: "target location longitude in degrees WGS84 (-180..180)"
    },
    {
        tag: 42, field: target_location_elevation, wire_len: 2, reader: crate::ber::read_u16,
        getter: target_location_elevation_meters, setter: set_target_location_elevation_meters,
        to_units: |c: u16| Some(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        from_units: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        unit: "meters",
        doc: "target location elevation in meters MSL (-900..19000)"
    },
}

/// Every ST 0601 tag this crate decodes into a typed field, in ascending tag
/// order: the framing Tag 2 (timestamp) and Tag 65 (version) plus the scaled
/// tags. The table is the codec's own, so it stays in step with
/// [`decode`](crate::decode) across releases. Does not include Tag 1 (the
/// checksum is structural framing, never surfaced as a field) or unknown tags.
#[must_use]
pub fn tags() -> &'static [TagInfo] {
    TAGS
}

/// Wire tag number for a field base name (e.g. `"sensor_latitude"`), or `None`
/// if no typed tag carries that name.
#[must_use]
pub fn tag_number(name: &str) -> Option<u8> {
    TAGS.iter().find(|t| t.name == name).map(|t| t.number)
}

/// Field base name for a wire tag number, or `None` if the tag is not one this
/// crate types.
#[must_use]
pub fn tag_name(number: u8) -> Option<&'static str> {
    TAGS.iter().find(|t| t.number == number).map(|t| t.name)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::st0601::St0601;

    fn round_trip(set: &St0601) -> St0601 {
        let mut buf = Vec::new();
        crate::st0601::encode(set, &mut buf).expect("encode");
        crate::st0601::decode(&buf).expect("decode own output")
    }

    #[test]
    fn attitude_round_trips_at_extremes() {
        for deg in [-50.0, -12.34, 0.0, 12.34, 50.0] {
            let mut set = St0601 {
                timestamp_us: 1,
                ..Default::default()
            };
            set.set_platform_roll_degrees(deg);
            let back = round_trip(&set)
                .platform_roll_degrees()
                .expect("roll present");
            let lsb = 50.0 / 32767.0;
            assert!(
                (back - deg).abs() <= lsb,
                "roll {deg} -> {back} (lsb {lsb})"
            );
        }
    }

    #[test]
    fn roll_sentinel_yields_none_but_preserves_raw() {
        // wire bytes 0x80 0x00 = i16::MIN = ST 0601 error indicator
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.platform_roll = Some(i16::MIN);
        let back = round_trip(&set);
        assert_eq!(
            back.platform_roll,
            Some(i16::MIN),
            "raw sentinel round-trips"
        );
        assert_eq!(
            back.platform_roll_degrees(),
            None,
            "accessor hides the sentinel"
        );
    }

    #[test]
    fn setters_clamp_out_of_range() {
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.set_platform_roll_degrees(-360.0);
        assert_eq!(set.platform_roll, Some(-32767), "clamped, never -32768");
        set.set_platform_true_airspeed_mps(9999.0);
        assert_eq!(set.platform_true_airspeed, Some(255));
        set.set_platform_true_airspeed_mps(-5.0);
        assert_eq!(set.platform_true_airspeed, Some(0));
    }

    #[test]
    fn wrong_length_typed_tag_falls_back_to_unknown() {
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        // tag 7 with 3 bytes (wrong; expects 2) must NOT be consumed as typed
        set.unknown.push((7, vec![0x01, 0x02, 0x03]));
        let back = round_trip(&set);
        assert_eq!(back.platform_roll, None);
        assert_eq!(back.unknown, vec![(7, vec![0x01, 0x02, 0x03])]);
    }

    #[test]
    fn sensor_latitude_sentinel_and_extremes() {
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.sensor_latitude = Some(i32::MIN);
        let back = round_trip(&set);
        assert_eq!(back.sensor_latitude, Some(i32::MIN));
        assert_eq!(back.sensor_latitude_degrees(), None);

        for deg in [-90.0, -33.3, 0.0, 33.3, 90.0] {
            let mut set = St0601 {
                timestamp_us: 1,
                ..Default::default()
            };
            set.set_sensor_latitude_degrees(deg);
            let back = round_trip(&set)
                .sensor_latitude_degrees()
                .expect("lat present");
            let lsb = 90.0 / 2_147_483_647.0;
            assert!((back - deg).abs() <= lsb, "lat {deg} -> {back}");
        }
    }

    #[test]
    fn altitude_round_trips_across_offset_range() {
        for m in [-900.0, 0.0, 5000.5, 19000.0] {
            let mut set = St0601 {
                timestamp_us: 1,
                ..Default::default()
            };
            set.set_sensor_true_altitude_meters(m);
            let back = round_trip(&set)
                .sensor_true_altitude_meters()
                .expect("alt present");
            let lsb = 19900.0 / 65535.0;
            assert!((back - m).abs() <= lsb, "alt {m} -> {back} (lsb {lsb})");
        }

        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.set_sensor_true_altitude_meters(-99999.0);
        assert_eq!(
            set.sensor_true_altitude,
            Some(0),
            "clamped below range minimum"
        );
    }

    #[test]
    fn relative_elevation_sentinel_and_round_trip() {
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.sensor_relative_elevation = Some(i32::MIN);
        let back = round_trip(&set);
        assert_eq!(
            back.sensor_relative_elevation,
            Some(i32::MIN),
            "raw sentinel round-trips"
        );
        assert_eq!(
            back.sensor_relative_elevation_degrees(),
            None,
            "accessor hides sentinel"
        );

        for deg in [-180.0, -45.5, 0.0, 45.5, 180.0] {
            let mut set = St0601 {
                timestamp_us: 1,
                ..Default::default()
            };
            set.set_sensor_relative_elevation_degrees(deg);
            let back = round_trip(&set)
                .sensor_relative_elevation_degrees()
                .expect("el present");
            let lsb = 180.0 / 2_147_483_647.0;
            assert!((back - deg).abs() <= lsb, "el {deg} -> {back}");
        }
    }

    #[test]
    fn pointing_unsigned_tags_round_trip_and_clamp() {
        // 16/17 (u16, span 180), 18/20 (u32, span 360)
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.set_sensor_horizontal_fov_degrees(144.5);
        set.set_sensor_vertical_fov_degrees(0.0);
        set.set_sensor_relative_azimuth_degrees(359.999);
        set.set_sensor_relative_roll_degrees(180.0);
        let back = round_trip(&set);
        let lsb_u16 = 180.0 / 65535.0;
        let lsb_u32 = 360.0 / 4_294_967_295.0;
        assert!((back.sensor_horizontal_fov_degrees().expect("16") - 144.5).abs() <= lsb_u16);
        assert!((back.sensor_vertical_fov_degrees().expect("17") - 0.0).abs() <= lsb_u16);
        assert!((back.sensor_relative_azimuth_degrees().expect("18") - 359.999).abs() <= lsb_u32);
        assert!((back.sensor_relative_roll_degrees().expect("20") - 180.0).abs() <= lsb_u32);

        // clamp beyond range
        let mut wild = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        wild.set_sensor_horizontal_fov_degrees(999.0);
        wild.set_sensor_relative_azimuth_degrees(-5.0);
        assert_eq!(wild.sensor_horizontal_fov, Some(65535));
        assert_eq!(wild.sensor_relative_azimuth, Some(0));
    }

    #[test]
    fn target_location_round_trips_and_honors_sentinel() {
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.set_target_location_latitude_degrees(-45.5);
        set.set_target_location_longitude_degrees(120.25);
        set.set_target_location_elevation_meters(1234.5);
        let back = round_trip(&set);
        let lat = back.target_location_latitude_degrees().expect("tag 40");
        let lon = back.target_location_longitude_degrees().expect("tag 41");
        let elev = back.target_location_elevation_meters().expect("tag 42");
        assert!((lat - -45.5).abs() <= 90.0 / 2_147_483_647.0);
        assert!((lon - 120.25).abs() <= 180.0 / 2_147_483_647.0);
        assert!((elev - 1234.5).abs() <= 19900.0 / 65535.0);

        let mut bad = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        bad.target_location_latitude = Some(i32::MIN);
        assert_eq!(round_trip(&bad).target_location_latitude_degrees(), None);
    }

    #[test]
    fn frame_center_sentinels_yield_none() {
        // 23/24 are the only remaining i32 tags whose sentinel is otherwise untested
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.frame_center_latitude = Some(i32::MIN);
        set.frame_center_longitude = Some(i32::MIN);
        let back = round_trip(&set);
        assert_eq!(back.frame_center_latitude_degrees(), None);
        assert_eq!(back.frame_center_longitude_degrees(), None);
        assert_eq!(
            back.frame_center_latitude,
            Some(i32::MIN),
            "raw sentinel round-trips"
        );
    }

    #[test]
    fn geometry_tags_round_trip_and_clamp() {
        // 21 (u32, span 5e6 m), 22 (u16, span 1e4 m), 25 (u16 offset -900..19000 m)
        let mut set = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        set.set_slant_range_meters(2_500_000.0);
        set.set_target_width_meters(722.82);
        set.set_frame_center_elevation_meters(-900.0);
        let back = round_trip(&set);
        let lsb_range = 5_000_000.0 / 4_294_967_295.0;
        let lsb_width = 10_000.0 / 65535.0;
        let lsb_elev = 19900.0 / 65535.0;
        assert!((back.slant_range_meters().expect("21") - 2_500_000.0).abs() <= lsb_range);
        assert!((back.target_width_meters().expect("22") - 722.82).abs() <= lsb_width);
        assert!((back.frame_center_elevation_meters().expect("25") - -900.0).abs() <= lsb_elev);

        let mut wild = St0601 {
            timestamp_us: 1,
            ..Default::default()
        };
        wild.set_slant_range_meters(9e9);
        wild.set_target_width_meters(-1.0);
        wild.set_frame_center_elevation_meters(99999.0);
        assert_eq!(wild.slant_range, Some(u32::MAX));
        assert_eq!(wild.target_width, Some(0));
        assert_eq!(wild.frame_center_elevation, Some(65535));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod registry_tests {
    use super::{tag_name, tag_number, tags};

    #[test]
    fn covers_all_22_decodable_tags_in_ascending_order() {
        // 20 scaled tags + Tag 2 (timestamp) + Tag 65 (version), no Tag 1.
        let all = tags();
        assert_eq!(all.len(), 22, "20 scaled + Tag 2 + Tag 65");
        assert_eq!(all.first().map(|t| t.number), Some(2), "Tag 2 leads");
        assert_eq!(all.last().map(|t| t.number), Some(65), "Tag 65 trails");
        assert!(
            all.windows(2).all(|w| w[0].number < w[1].number),
            "strictly ascending tag numbers"
        );
        assert!(
            all.iter().all(|t| t.number != 1),
            "Tag 1 (checksum) is framing, never a field"
        );
    }

    #[test]
    fn framing_tags_carry_expected_names_and_units() {
        assert_eq!(tag_name(2), Some("timestamp"));
        assert_eq!(tag_name(65), Some("version"));
        let version = tags().iter().find(|t| t.number == 65).expect("tag 65");
        assert_eq!(version.unit, None, "version is unit-less");
        let timestamp = tags().iter().find(|t| t.number == 2).expect("tag 2");
        assert_eq!(timestamp.unit, Some("microseconds"));
    }

    #[test]
    fn scaled_tags_all_carry_a_unit() {
        for t in tags().iter().filter(|t| t.number != 65) {
            assert!(t.unit.is_some(), "tag {} has a unit", t.number);
        }
    }

    #[test]
    fn name_and_number_lookups_are_inverse() {
        for t in tags() {
            assert_eq!(tag_number(t.name), Some(t.number), "name -> number");
            assert_eq!(tag_name(t.number), Some(t.name), "number -> name");
        }
    }

    #[test]
    fn sample_scaled_tag_is_named_from_the_field() {
        // Field base name, not the accessor: `sensor_latitude`, not `sensor_latitude_degrees`.
        assert_eq!(tag_number("sensor_latitude"), Some(13));
        assert_eq!(tag_name(13), Some("sensor_latitude"));
    }

    #[test]
    fn absent_names_and_numbers_return_none() {
        assert_eq!(tag_number("not_a_tag"), None);
        assert_eq!(
            tag_number("sensor_latitude_degrees"),
            None,
            "accessor, not field"
        );
        assert_eq!(tag_name(1), None, "checksum tag is not a field");
        assert_eq!(tag_name(99), None);
    }
}
