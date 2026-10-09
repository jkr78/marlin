//! Table-driven scaled-tag machinery. Each `scaled_tags!` entry expands to the encode
//! arm and the decode dispatch arm for one tag, and to its [`TAGS`] row.
//! Adding a future ST 0601 tag = one struct field in `st0601.rs` + one entry here
//! (+ verify its formula against the standard — do NOT pattern-match new tags).

use alloc::vec::Vec;

use marlin_field::{FieldState, Invalid, Kind, RawCode};

use crate::ber::push_item;
use crate::error::KlvEncodeError;
use crate::st0601::St0601;

/// Metadata for one ST 0601 tag this crate decodes into a typed field: its wire
/// number, the [`St0601`] field base name (e.g. `"sensor_latitude"`, the
/// field name minus its unit suffix), and its engineering unit. Sourced from
/// the codec's own tag table, so it cannot drift from what
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
        name: $name:literal,
        wire: $wire:ty,
        reader: $reader:path,
        sentinel: $sentinel:expr,
        decode: $decode:expr,
        encode: $encode:expr,
        range: ($lo:literal, $hi:literal),
        unit: $unit:literal
    }),+ $(,)?) => {
        /// Every tag this crate decodes into a typed field, in ascending tag order.
        /// The scaled entries are generated from the table below (drift-free); the two
        /// framing tags handled outside the macro — Tag 2 (precision timestamp) and
        /// Tag 65 (LS version) — bracket them. Adding a scaled tag numbered below 65
        /// keeps the slice sorted automatically.
        pub const TAGS: &[TagInfo] = &[
            TagInfo { number: 2, name: "timestamp", unit: Some("microseconds") },
            $(
                TagInfo { number: $tag, name: $name, unit: Some($unit) },
            )+
            TagInfo { number: 65, name: "version", unit: None },
        ];

        /// The `(base name, field name)` pairs of the table, for the test that keeps
        /// the two in step.
        #[cfg(test)]
        const FIELD_NAMES: &[(&str, &str)] = &[
            $( ($name, stringify!($field)), )+
        ];

        /// Append every scaled tag the set can carry on the wire to `items`, in table
        /// (ascending tag) order: a value as its range-checked count, a sender error
        /// as the tag's own sentinel; a not-available or unparsable field emits
        /// nothing. Any other state is a [`KlvEncodeError`].
        pub(crate) fn encode_scaled(
            set: &St0601,
            items: &mut Vec<u8>,
        ) -> Result<(), KlvEncodeError> {
            $(
                let sentinel: Option<$wire> = $sentinel;
                match set.$field {
                    FieldState::Value(v) => {
                        if !crate::scale::in_range(v, $lo, $hi) {
                            return Err(KlvEncodeError::OutOfRange { tag: $tag });
                        }
                        let raw: $wire = ($encode)(v);
                        push_item($tag, &raw.to_be_bytes(), items);
                    }
                    FieldState::NotAvailable | FieldState::Invalid(Invalid::Unparsable) => {}
                    FieldState::SenderError(RawCode(code)) => match sentinel {
                        Some(own) if i64::from(own) == code => {
                            push_item($tag, &own.to_be_bytes(), items);
                        }
                        _ => {
                            return Err(KlvEncodeError::Unencodable {
                                tag: $tag,
                                kind: Kind::SenderError(RawCode(code)),
                            });
                        }
                    },
                    other @ (FieldState::AtLeast(_) | FieldState::Invalid(Invalid::Undefined(_))) => {
                        return Err(KlvEncodeError::Unencodable { tag: $tag, kind: other.kind() });
                    }
                }
            )+
            Ok(())
        }

        /// Try to decode `tag` as a scaled tag; `true` = the tag is a scaled tag. With
        /// the right wire length the field takes the tag's partition of the count;
        /// with the wrong length the bytes are kept in `unknown` and the field is
        /// `Invalid(Unparsable)` unless an earlier occurrence of the tag already gave it
        /// a state, so re-encode stays lossless and re-decodes to the same set.
        pub(crate) fn decode_scaled(tag: u8, value: &[u8], set: &mut St0601) -> bool {
            match tag {
                $(
                    $tag => {
                        match $reader(value) {
                            Some(raw) => set.$field = ($decode)(raw),
                            None => {
                                set.unknown.push(($tag, value.to_vec()));
                                if set.$field == FieldState::NotAvailable {
                                    set.$field = FieldState::Invalid(Invalid::Unparsable);
                                }
                            }
                        }
                        true
                    }
                )+
                _ => false,
            }
        }
    };
}

scaled_tags! {
    {
        tag: 5, field: platform_heading_degrees, name: "platform_heading",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_to_units(c, 360.0)),
        encode: |v: f64| crate::scale::units_to_u16(v, 360.0),
        range: (0.0, 360.0), unit: "degrees"
    },
    {
        tag: 6, field: platform_pitch_degrees, name: "platform_pitch",
        wire: i16, reader: crate::ber::read_i16, sentinel: Some(i16::MIN),
        decode: |c: i16| crate::scale::i16_field(c, 20.0),
        encode: |v: f64| crate::scale::units_to_i16(v, 20.0),
        range: (-20.0, 20.0), unit: "degrees"
    },
    {
        tag: 7, field: platform_roll_degrees, name: "platform_roll",
        wire: i16, reader: crate::ber::read_i16, sentinel: Some(i16::MIN),
        decode: |c: i16| crate::scale::i16_field(c, 50.0),
        encode: |v: f64| crate::scale::units_to_i16(v, 50.0),
        range: (-50.0, 50.0), unit: "degrees"
    },
    {
        tag: 8, field: platform_true_airspeed_mps, name: "platform_true_airspeed",
        wire: u8, reader: crate::ber::read_u8, sentinel: None,
        decode: |c: u8| FieldState::Value(f64::from(c)),
        encode: |v: f64| crate::scale::units_to_u8(v),
        range: (0.0, 255.0), unit: "mps"
    },
    {
        tag: 13, field: sensor_latitude_degrees, name: "sensor_latitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 90.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 90.0),
        range: (-90.0, 90.0), unit: "degrees"
    },
    {
        tag: 14, field: sensor_longitude_degrees, name: "sensor_longitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 180.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 180.0),
        range: (-180.0, 180.0), unit: "degrees"
    },
    {
        tag: 15, field: sensor_true_altitude_meters, name: "sensor_true_altitude",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        encode: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        range: (-900.0, 19000.0), unit: "meters"
    },
    {
        tag: 16, field: sensor_horizontal_fov_degrees, name: "sensor_horizontal_fov",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_to_units(c, 180.0)),
        encode: |v: f64| crate::scale::units_to_u16(v, 180.0),
        range: (0.0, 180.0), unit: "degrees"
    },
    {
        tag: 17, field: sensor_vertical_fov_degrees, name: "sensor_vertical_fov",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_to_units(c, 180.0)),
        encode: |v: f64| crate::scale::units_to_u16(v, 180.0),
        range: (0.0, 180.0), unit: "degrees"
    },
    {
        tag: 18, field: sensor_relative_azimuth_degrees, name: "sensor_relative_azimuth",
        wire: u32, reader: crate::ber::read_u32, sentinel: None,
        decode: |c: u32| FieldState::Value(crate::scale::u32_to_units(c, 360.0)),
        encode: |v: f64| crate::scale::units_to_u32(v, 360.0),
        range: (0.0, 360.0), unit: "degrees"
    },
    {
        tag: 19, field: sensor_relative_elevation_degrees, name: "sensor_relative_elevation",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 180.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 180.0),
        range: (-180.0, 180.0), unit: "degrees"
    },
    {
        tag: 20, field: sensor_relative_roll_degrees, name: "sensor_relative_roll",
        wire: u32, reader: crate::ber::read_u32, sentinel: None,
        decode: |c: u32| FieldState::Value(crate::scale::u32_to_units(c, 360.0)),
        encode: |v: f64| crate::scale::units_to_u32(v, 360.0),
        range: (0.0, 360.0), unit: "degrees"
    },
    {
        tag: 21, field: slant_range_meters, name: "slant_range",
        wire: u32, reader: crate::ber::read_u32, sentinel: None,
        decode: |c: u32| FieldState::Value(crate::scale::u32_to_units(c, 5_000_000.0)),
        encode: |v: f64| crate::scale::units_to_u32(v, 5_000_000.0),
        range: (0.0, 5_000_000.0), unit: "meters"
    },
    {
        tag: 22, field: target_width_meters, name: "target_width",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_to_units(c, 10_000.0)),
        encode: |v: f64| crate::scale::units_to_u16(v, 10_000.0),
        range: (0.0, 10_000.0), unit: "meters"
    },
    {
        tag: 23, field: frame_center_latitude_degrees, name: "frame_center_latitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 90.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 90.0),
        range: (-90.0, 90.0), unit: "degrees"
    },
    {
        tag: 24, field: frame_center_longitude_degrees, name: "frame_center_longitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 180.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 180.0),
        range: (-180.0, 180.0), unit: "degrees"
    },
    {
        tag: 25, field: frame_center_elevation_meters, name: "frame_center_elevation",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        encode: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        range: (-900.0, 19000.0), unit: "meters"
    },
    {
        tag: 40, field: target_location_latitude_degrees, name: "target_location_latitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 90.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 90.0),
        range: (-90.0, 90.0), unit: "degrees"
    },
    {
        tag: 41, field: target_location_longitude_degrees, name: "target_location_longitude",
        wire: i32, reader: crate::ber::read_i32, sentinel: Some(i32::MIN),
        decode: |c: i32| crate::scale::i32_field(c, 180.0),
        encode: |v: f64| crate::scale::units_to_i32(v, 180.0),
        range: (-180.0, 180.0), unit: "degrees"
    },
    {
        tag: 42, field: target_location_elevation_meters, name: "target_location_elevation",
        wire: u16, reader: crate::ber::read_u16, sentinel: None,
        decode: |c: u16| FieldState::Value(crate::scale::u16_offset_to_units(c, 19900.0, -900.0)),
        encode: |v: f64| crate::scale::units_to_u16_offset(v, 19900.0, -900.0),
        range: (-900.0, 19000.0), unit: "meters"
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
mod decode_tests {
    use alloc::{vec, vec::Vec};

    use marlin_field::{FieldState, Invalid, RawCode};

    use crate::st0601::{decode, encode, St0601};
    use crate::testing::KlvBuilder;

    #[test]
    fn i16_sentinel_is_a_sender_error() {
        // Tag 7 platform roll, wire bytes 0x80 0x00 = i16::MIN = the ST 0601 sentinel.
        let packet = KlvBuilder::new().timestamp(1).tag(7, &[0x80, 0x00]).build();
        let set = decode(&packet).expect("decode");
        assert_eq!(
            set.platform_roll_degrees,
            FieldState::SenderError(RawCode(-32768))
        );
    }

    #[test]
    fn i32_sentinel_is_a_sender_error() {
        // Tag 13 sensor latitude, wire bytes 0x80 00 00 00 = i32::MIN.
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(13, &[0x80, 0x00, 0x00, 0x00])
            .build();
        let set = decode(&packet).expect("decode");
        assert_eq!(
            set.sensor_latitude_degrees,
            FieldState::SenderError(RawCode(-2_147_483_648))
        );
    }

    #[test]
    fn omitted_tags_are_not_available_in_every_width() {
        let packet = KlvBuilder::new().timestamp(1).build();
        let set = decode(&packet).expect("decode");
        assert_eq!(
            set.platform_true_airspeed_mps,
            FieldState::NotAvailable,
            "u8"
        );
        assert_eq!(
            set.platform_heading_degrees,
            FieldState::NotAvailable,
            "u16"
        );
        assert_eq!(set.platform_pitch_degrees, FieldState::NotAvailable, "i16");
        assert_eq!(
            set.sensor_relative_azimuth_degrees,
            FieldState::NotAvailable,
            "u32"
        );
        assert_eq!(set.sensor_latitude_degrees, FieldState::NotAvailable, "i32");
        assert_eq!(
            set.sensor_true_altitude_meters,
            FieldState::NotAvailable,
            "u16 offset"
        );
        assert_eq!(set.version, FieldState::NotAvailable, "Tag 65");
        assert_eq!(set, St0601::new(1));
    }

    #[test]
    fn wrong_length_known_tag_is_unparsable_and_keeps_its_bytes() {
        // Tag 13 is a 4-byte tag; 3 bytes cannot be read as an i32.
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(13, &[0x01, 0x02, 0x03])
            .build();
        let set = decode(&packet).expect("decode");
        assert_eq!(
            set.sensor_latitude_degrees,
            FieldState::Invalid(Invalid::Unparsable)
        );
        assert_eq!(set.unknown, vec![(13, vec![0x01, 0x02, 0x03])]);

        let mut re_encoded = Vec::new();
        encode(&set, &mut re_encoded).expect("re-encode");
        assert_eq!(re_encoded, packet, "the kept bytes re-encode verbatim");
    }

    #[test]
    fn wrong_length_duplicate_after_a_readable_one_keeps_the_value() {
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(13, &[0x40, 0x00, 0x00, 0x00])
            .tag(13, &[0x01, 0x02, 0x03])
            .build();
        let set = decode(&packet).expect("decode");
        assert!(matches!(set.sensor_latitude_degrees, FieldState::Value(_)));
        assert_eq!(set.unknown, vec![(13, vec![0x01, 0x02, 0x03])]);

        let mut re_encoded = Vec::new();
        encode(&set, &mut re_encoded).expect("re-encode");
        assert_eq!(re_encoded, packet, "typed then kept bytes is framing order");
        assert_eq!(decode(&re_encoded).expect("re-decode"), set);
    }

    #[test]
    fn readable_duplicate_after_a_wrong_length_one_takes_the_field() {
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(13, &[0x01, 0x02, 0x03])
            .tag(13, &[0x40, 0x00, 0x00, 0x00])
            .build();
        let set = decode(&packet).expect("decode");
        assert!(matches!(set.sensor_latitude_degrees, FieldState::Value(_)));
        assert_eq!(set.unknown, vec![(13, vec![0x01, 0x02, 0x03])]);

        // Not framing order, so not byte-exact; the re-encoded set re-decodes equal.
        let mut re_encoded = Vec::new();
        encode(&set, &mut re_encoded).expect("re-encode");
        assert_eq!(decode(&re_encoded).expect("re-decode"), set);
    }

    #[test]
    fn heading_kat_from_vector_1() {
        // 0x71c2 = 29122 → 159.97436484321355° (klvdata expected value)
        let packet = KlvBuilder::new().timestamp(1).tag(5, &[0x71, 0xC2]).build();
        let set = decode(&packet).expect("decode");
        let deg = set.platform_heading_degrees.value().expect("heading");
        assert!((deg - 159.974_364_843_213_55).abs() < 1e-9, "got {deg}");
    }

    #[test]
    fn signed_extremes_decode_to_half_span() {
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(7, &[0x7F, 0xFF]) // 32767
            .tag(13, &[0x80, 0x00, 0x00, 0x01]) // -2147483647
            .build();
        let set = decode(&packet).expect("decode");
        assert_eq!(set.platform_roll_degrees, FieldState::Value(50.0));
        assert_eq!(set.sensor_latitude_degrees, FieldState::Value(-90.0));
    }

    #[test]
    fn altitude_offset_decodes_from_range_minimum() {
        let packet = KlvBuilder::new()
            .timestamp(1)
            .tag(15, &[0x00, 0x00])
            .tag(25, &[0xFF, 0xFF])
            .build();
        let set = decode(&packet).expect("decode");
        assert_eq!(set.sensor_true_altitude_meters, FieldState::Value(-900.0));
        assert_eq!(
            set.frame_center_elevation_meters,
            FieldState::Value(19000.0)
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod encode_tests {
    use alloc::{vec, vec::Vec};

    use marlin_field::{FieldState, Invalid, Kind, RawCode};

    use crate::error::KlvEncodeError;
    use crate::st0601::{decode, encode, St0601};

    fn round_trip(set: &St0601) -> St0601 {
        let mut buf = Vec::new();
        encode(set, &mut buf).expect("encode");
        decode(&buf).expect("decode own output")
    }

    fn encode_err(set: &St0601) -> KlvEncodeError {
        let mut buf = Vec::new();
        encode(set, &mut buf).expect_err("encode must fail")
    }

    #[test]
    fn values_round_trip_within_one_lsb_in_every_width() {
        let mut set = St0601::new(1);
        set.platform_true_airspeed_mps = FieldState::Value(77.0);
        set.platform_heading_degrees = FieldState::Value(159.97);
        set.platform_roll_degrees = FieldState::Value(-12.34);
        set.sensor_relative_azimuth_degrees = FieldState::Value(359.999);
        set.sensor_latitude_degrees = FieldState::Value(60.1768);
        set.sensor_true_altitude_meters = FieldState::Value(5000.5);
        let back = round_trip(&set);
        let within = |state: FieldState<f64>, want: f64, lsb: f64| {
            let got = state.value().expect("value");
            assert!((got - want).abs() <= lsb, "{want} -> {got} (lsb {lsb})");
        };
        within(back.platform_true_airspeed_mps, 77.0, 0.0);
        within(back.platform_heading_degrees, 159.97, 360.0 / 65535.0);
        within(back.platform_roll_degrees, -12.34, 50.0 / 32767.0);
        within(
            back.sensor_relative_azimuth_degrees,
            359.999,
            360.0 / 4_294_967_295.0,
        );
        within(
            back.sensor_latitude_degrees,
            60.1768,
            90.0 / 2_147_483_647.0,
        );
        within(back.sensor_true_altitude_meters, 5000.5, 19900.0 / 65535.0);
    }

    #[test]
    fn range_ends_encode_and_decode_exactly() {
        let mut set = St0601::new(1);
        set.platform_heading_degrees = FieldState::Value(360.0);
        set.platform_roll_degrees = FieldState::Value(-50.0);
        set.sensor_latitude_degrees = FieldState::Value(90.0);
        set.sensor_true_altitude_meters = FieldState::Value(-900.0);
        set.slant_range_meters = FieldState::Value(5_000_000.0);
        assert_eq!(round_trip(&set), set);
    }

    #[test]
    fn own_sentinel_round_trips_as_a_sender_error() {
        let mut set = St0601::new(1);
        set.platform_pitch_degrees = FieldState::SenderError(RawCode(i16::MIN.into()));
        set.frame_center_longitude_degrees = FieldState::SenderError(RawCode(i32::MIN.into()));
        assert_eq!(round_trip(&set), set);
    }

    #[test]
    fn u16_value_outside_range_is_rejected() {
        for v in [-0.001, 360.001, f64::NAN] {
            let mut set = St0601::new(1);
            set.platform_heading_degrees = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 5 },
                "{v}"
            );
        }
    }

    #[test]
    fn i16_value_outside_range_is_rejected() {
        for v in [-50.001, 50.001, f64::NAN] {
            let mut set = St0601::new(1);
            set.platform_roll_degrees = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 7 },
                "{v}"
            );
        }
    }

    #[test]
    fn u8_value_outside_range_is_rejected() {
        for v in [-1.0, 255.5, f64::NAN] {
            let mut set = St0601::new(1);
            set.platform_true_airspeed_mps = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 8 },
                "{v}"
            );
        }
    }

    #[test]
    fn i32_value_outside_range_is_rejected() {
        for v in [-90.0001, 90.0001, f64::NAN] {
            let mut set = St0601::new(1);
            set.sensor_latitude_degrees = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 13 },
                "{v}"
            );
        }
    }

    #[test]
    fn u32_value_outside_range_is_rejected() {
        for v in [-0.001, 5_000_000.001, f64::NAN] {
            let mut set = St0601::new(1);
            set.slant_range_meters = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 21 },
                "{v}"
            );
        }
    }

    #[test]
    fn offset_value_outside_range_is_rejected() {
        for v in [-900.001, 19000.001, f64::NAN] {
            let mut set = St0601::new(1);
            set.target_location_elevation_meters = FieldState::Value(v);
            assert_eq!(
                encode_err(&set),
                KlvEncodeError::OutOfRange { tag: 42 },
                "{v}"
            );
        }
    }

    #[test]
    fn over_range_bound_is_unencodable() {
        let mut set = St0601::new(1);
        set.sensor_latitude_degrees = FieldState::AtLeast(90.0);
        assert_eq!(
            encode_err(&set),
            KlvEncodeError::Unencodable {
                tag: 13,
                kind: Kind::AtLeast
            }
        );
    }

    #[test]
    fn undefined_raw_code_is_unencodable() {
        let mut set = St0601::new(1);
        set.sensor_latitude_degrees = FieldState::Invalid(Invalid::Undefined(RawCode(7)));
        assert_eq!(
            encode_err(&set),
            KlvEncodeError::Unencodable {
                tag: 13,
                kind: Kind::Invalid(Invalid::Undefined(RawCode(7)))
            }
        );
    }

    #[test]
    fn foreign_sender_error_code_is_unencodable() {
        // i16::MIN is Tag 6's sentinel, not Tag 13's.
        let mut set = St0601::new(1);
        set.sensor_latitude_degrees = FieldState::SenderError(RawCode(i16::MIN.into()));
        assert_eq!(
            encode_err(&set),
            KlvEncodeError::Unencodable {
                tag: 13,
                kind: Kind::SenderError(RawCode(-32768))
            }
        );
    }

    #[test]
    fn sender_error_on_an_unsigned_tag_is_unencodable() {
        let mut set = St0601::new(1);
        set.platform_heading_degrees = FieldState::SenderError(RawCode(0x8000));
        assert_eq!(
            encode_err(&set),
            KlvEncodeError::Unencodable {
                tag: 5,
                kind: Kind::SenderError(RawCode(0x8000))
            }
        );
    }

    #[test]
    fn unparsable_field_emits_nothing_and_unknown_carries_the_bytes() {
        let mut set = St0601::new(1);
        set.platform_roll_degrees = FieldState::Invalid(Invalid::Unparsable);
        set.unknown.push((7, vec![0x01, 0x02, 0x03]));
        let back = round_trip(&set);
        assert_eq!(back, set);
    }

    #[test]
    fn unparsable_field_with_no_kept_bytes_re_decodes_as_not_available() {
        let mut set = St0601::new(1);
        set.platform_roll_degrees = FieldState::Invalid(Invalid::Unparsable);
        assert_eq!(
            round_trip(&set).platform_roll_degrees,
            FieldState::NotAvailable
        );
    }

    #[test]
    fn table_names_are_the_field_names_without_the_unit_suffix() {
        for (name, field) in super::FIELD_NAMES {
            let suffix = field
                .strip_prefix(name)
                .expect("name is a prefix of the field");
            assert!(
                ["_degrees", "_meters", "_mps"].contains(&suffix),
                "{field}: suffix {suffix:?}"
            );
        }
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
    fn sample_scaled_tag_is_named_from_the_field_base_name() {
        // Base name, not the field: `sensor_latitude`, not `sensor_latitude_degrees`.
        assert_eq!(tag_number("sensor_latitude"), Some(13));
        assert_eq!(tag_name(13), Some("sensor_latitude"));
    }

    #[test]
    fn absent_names_and_numbers_return_none() {
        assert_eq!(tag_number("not_a_tag"), None);
        assert_eq!(
            tag_number("sensor_latitude_degrees"),
            None,
            "field, not base name"
        );
        assert_eq!(tag_name(1), None, "checksum tag is not a field");
        assert_eq!(tag_name(99), None);
    }
}
