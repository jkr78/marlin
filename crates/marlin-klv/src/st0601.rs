use alloc::vec::Vec;

use marlin_field::{FieldState, Invalid};

use crate::ber::{
    ber_decode_len, ber_encode_len, push_item, read_bytes, read_u16, read_u64, read_u8,
};
use crate::checksum::bcc;
use crate::error::{EncodeError, Error};

/// MISB ST 0601 UAS Datalink Local Set 16-byte universal label (SMPTE UL key).
pub const UAS_LS_KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
];

/// MISB ST 0601 UAS Datalink Local Set — the typed core. Every scaled tag is a
/// [`FieldState<f64>`] in engineering units; an omitted tag is `NotAvailable`,
/// the ST 0601 sentinel on a signed tag is `SenderError` with the raw
/// code, and a known tag with the wrong wire length is `Invalid(Unparsable)`
/// with its bytes kept in [`St0601::unknown`]. Unrecognized 1-byte tags
/// round-trip via [`St0601::unknown`], so broadening the typed set is additive.
///
/// Build one with [`St0601::new`] and assign the fields directly:
/// `set.sensor_latitude_degrees = FieldState::Value(60.1768)`.
#[derive(Clone, Debug, PartialEq)]
pub struct St0601 {
    /// Tag 2: precision timestamp, microseconds since the UNIX epoch (UTC). Mandatory —
    /// always encoded, always present in a successfully decoded set.
    pub timestamp_us: u64,
    /// Tag 65: UAS LS document version number.
    pub version: FieldState<u8>,
    /// Tag 5: platform heading in degrees (0..=360; u16 on the wire).
    pub platform_heading_degrees: FieldState<f64>,
    /// Tag 6: platform pitch in degrees (−20..=20; i16 on the wire, `i16::MIN` is the
    /// sentinel).
    pub platform_pitch_degrees: FieldState<f64>,
    /// Tag 7: platform roll in degrees (−50..=50; i16 on the wire, `i16::MIN` is the
    /// sentinel).
    pub platform_roll_degrees: FieldState<f64>,
    /// Tag 8: platform true airspeed in m/s (0..=255; u8 on the wire, identity).
    pub platform_true_airspeed_mps: FieldState<f64>,
    /// Tag 13: sensor latitude in degrees WGS84 (−90..=90; i32 on the wire,
    /// `i32::MIN` is the sentinel).
    pub sensor_latitude_degrees: FieldState<f64>,
    /// Tag 14: sensor longitude in degrees WGS84 (−180..=180; i32 on the wire,
    /// `i32::MIN` is the sentinel).
    pub sensor_longitude_degrees: FieldState<f64>,
    /// Tag 15: sensor true altitude in meters MSL (−900..=19000; u16 on the wire).
    pub sensor_true_altitude_meters: FieldState<f64>,
    /// Tag 16: sensor horizontal field of view in degrees (0..=180; u16 on the wire).
    pub sensor_horizontal_fov_degrees: FieldState<f64>,
    /// Tag 17: sensor vertical field of view in degrees (0..=180; u16 on the wire).
    pub sensor_vertical_fov_degrees: FieldState<f64>,
    /// Tag 18: sensor relative azimuth in degrees (0..=360; u32 on the wire).
    pub sensor_relative_azimuth_degrees: FieldState<f64>,
    /// Tag 19: sensor relative elevation in degrees (−180..=180, negative = below
    /// horizon; i32 on the wire, `i32::MIN` is the sentinel).
    pub sensor_relative_elevation_degrees: FieldState<f64>,
    /// Tag 20: sensor relative roll in degrees (0..=360, clockwise from behind the
    /// camera; u32 on the wire).
    pub sensor_relative_roll_degrees: FieldState<f64>,
    /// Tag 21: slant range in meters (0..=5 000 000; u32 on the wire).
    pub slant_range_meters: FieldState<f64>,
    /// Tag 22: target width in meters (0..=10 000; u16 on the wire).
    pub target_width_meters: FieldState<f64>,
    /// Tag 23: frame center latitude in degrees WGS84 (−90..=90; i32 on the wire,
    /// `i32::MIN` is the sentinel).
    pub frame_center_latitude_degrees: FieldState<f64>,
    /// Tag 24: frame center longitude in degrees WGS84 (−180..=180; i32 on the wire,
    /// `i32::MIN` is the sentinel).
    pub frame_center_longitude_degrees: FieldState<f64>,
    /// Tag 25: frame center elevation in meters MSL (−900..=19000; u16 on the wire).
    pub frame_center_elevation_meters: FieldState<f64>,
    /// Tag 40: target location latitude in degrees WGS84 (−90..=90; i32 on the wire,
    /// `i32::MIN` is the sentinel).
    pub target_location_latitude_degrees: FieldState<f64>,
    /// Tag 41: target location longitude in degrees WGS84 (−180..=180; i32 on the
    /// wire, `i32::MIN` is the sentinel).
    pub target_location_longitude_degrees: FieldState<f64>,
    /// Tag 42: target location elevation in meters MSL (−900..=19000; u16 on the wire).
    pub target_location_elevation_meters: FieldState<f64>,
    /// Tags this crate does not type, preserved verbatim as `(tag, value)` in wire order
    /// and re-emitted on encode after the typed tags. The bytes of a known tag decoded
    /// with the wrong wire length also land here, beside its `Invalid(Unparsable)`
    /// field (or beside the value an occurrence with the right length gave it).
    /// Unvalidated: a typed tag pushed here by hand is emitted twice and the
    /// last occurrence wins on re-decode. Tag 2 (the mandatory timestamp) is never
    /// routed here — a malformed Tag 2 fails the whole decode instead.
    pub unknown: Vec<(u8, Vec<u8>)>,
}

impl St0601 {
    /// A set carrying only the mandatory Tag 2 timestamp: every optional field is
    /// `NotAvailable` and `unknown` is empty.
    #[must_use]
    pub fn new(timestamp_us: u64) -> Self {
        Self {
            timestamp_us,
            version: FieldState::NotAvailable,
            platform_heading_degrees: FieldState::NotAvailable,
            platform_pitch_degrees: FieldState::NotAvailable,
            platform_roll_degrees: FieldState::NotAvailable,
            platform_true_airspeed_mps: FieldState::NotAvailable,
            sensor_latitude_degrees: FieldState::NotAvailable,
            sensor_longitude_degrees: FieldState::NotAvailable,
            sensor_true_altitude_meters: FieldState::NotAvailable,
            sensor_horizontal_fov_degrees: FieldState::NotAvailable,
            sensor_vertical_fov_degrees: FieldState::NotAvailable,
            sensor_relative_azimuth_degrees: FieldState::NotAvailable,
            sensor_relative_elevation_degrees: FieldState::NotAvailable,
            sensor_relative_roll_degrees: FieldState::NotAvailable,
            slant_range_meters: FieldState::NotAvailable,
            target_width_meters: FieldState::NotAvailable,
            frame_center_latitude_degrees: FieldState::NotAvailable,
            frame_center_longitude_degrees: FieldState::NotAvailable,
            frame_center_elevation_meters: FieldState::NotAvailable,
            target_location_latitude_degrees: FieldState::NotAvailable,
            target_location_longitude_degrees: FieldState::NotAvailable,
            target_location_elevation_meters: FieldState::NotAvailable,
            unknown: Vec::new(),
        }
    }
}

/// Encode a `St0601` set into `out` (appended) in framing order: UAS LS key, outer BER
/// length, Tag 2 (mandatory timestamp), Tag 65, the scaled tags ascending, the
/// preserved unknown tags, and the Tag 1 checksum last. `out` may be non-empty; the
/// checksum covers only this call's bytes.
///
/// A value emits its range-checked count, a sender error the tag's own error
/// indicator; a not-available or unparsable field emits nothing (the bytes of an
/// unparsable field ride in `unknown`). Any other state, or a value outside the tag's
/// range or NaN, is an [`EncodeError`], and `out` is untouched on `Err`.
pub fn encode(set: &St0601, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    let mut items: Vec<u8> = Vec::new();
    // Tag 2 Precision Time Stamp — mandatory, prepended first.
    push_item(2, &set.timestamp_us.to_be_bytes(), &mut items);
    // Tag 65 UAS LS Version.
    match set.version {
        FieldState::Value(version) => push_item(65, &[version], &mut items),
        FieldState::NotAvailable | FieldState::Invalid(Invalid::Unparsable) => {}
        other => {
            return Err(EncodeError::Unencodable {
                tag: 65,
                kind: other.kind(),
            })
        }
    }
    crate::tags::encode_scaled(set, &mut items)?;
    // Unrecognized tags, original order.
    for (tag, value) in &set.unknown {
        push_item(*tag, value, &mut items);
    }

    // Outer length covers all items plus the 4-byte Tag 1 checksum item appended below.
    let start = out.len();
    let value_len = items.len() + 4;
    out.extend_from_slice(&UAS_LS_KEY);
    ber_encode_len(value_len, out);
    out.extend_from_slice(&items);

    // Tag 1 Checksum — tag + length now; value computed over everything written this call.
    out.push(1);
    out.push(2);
    let checksum = bcc(out.get(start..).unwrap_or(&[]));
    out.extend_from_slice(&checksum.to_be_bytes());
    Ok(())
}

/// Decode an ST 0601 local set: verify the UAS LS key, walk every TLV item into typed
/// fields (unknown tags preserved in order), then verify the embedded Tag 1 checksum.
///
/// Fails only for a structural reason: framing ([`Error::BadKey`], [`Error::Truncated`],
/// [`Error::LengthOverflow`]), the checksum ([`Error::MissingChecksum`],
/// [`Error::BadChecksum`]) or the mandatory Tag 2 timestamp absent or not 8 bytes
/// ([`Error::BadTimestamp`]). A field's value never fails the set.
pub fn decode(input: &[u8]) -> Result<St0601, Error> {
    let (mut offset, end) = frame(input)?;
    // The timestamp is settled after the checksum: a checksum failure outranks a
    // missing or malformed Tag 2. The 0 is a placeholder the loop never reads; the set
    // takes the decoded timestamp or is not returned.
    let mut timestamp: Result<u64, Error> = Err(Error::BadTimestamp);
    let mut set = St0601::new(0);
    let mut checksum: Option<(u16, usize)> = None;
    while offset < end {
        let tag = *input.get(offset).ok_or(Error::Truncated {
            offset,
            needed: 1,
            available: 0,
        })?;
        let (len, value_start) = ber_decode_len(input, offset + 1)?;
        let value = read_bytes(input, value_start, len)?;
        match tag {
            1 => checksum = Some((read_u16(value).ok_or(Error::MissingChecksum)?, value_start)),
            2 => timestamp = read_u64(value).ok_or(Error::BadTimestamp),
            65 => {
                if let Some(v) = read_u8(value) {
                    set.version = FieldState::Value(v);
                } else {
                    set.unknown.push((65, value.to_vec()));
                    if set.version == FieldState::NotAvailable {
                        set.version = FieldState::Invalid(Invalid::Unparsable);
                    }
                }
            }
            other => {
                if !crate::tags::decode_scaled(other, value, &mut set) {
                    set.unknown.push((other, value.to_vec()));
                }
            }
        }
        offset = value_start + len;
    }

    // Tag 1 is mandatory and always last; verify it over everything up to its value bytes.
    let (embedded, value_start) = checksum.ok_or(Error::MissingChecksum)?;
    let computed = bcc(input.get(..value_start).unwrap_or(&[]));
    if computed != embedded {
        return Err(Error::BadChecksum { computed, embedded });
    }
    set.timestamp_us = timestamp?;
    Ok(set)
}

/// Cheap Tag 2 peek: skip the key + outer length, scan items for Tag 2, return its
/// microsecond value. Returns `Ok(None)` when absent and [`Error::BadTimestamp`] when
/// present but not 8 bytes. Does NOT verify the checksum.
pub fn precision_timestamp(input: &[u8]) -> Result<Option<u64>, Error> {
    let (mut offset, end) = frame(input)?;
    while offset < end {
        let tag = *input.get(offset).ok_or(Error::Truncated {
            offset,
            needed: 1,
            available: 0,
        })?;
        let (len, value_start) = ber_decode_len(input, offset + 1)?;
        let value = read_bytes(input, value_start, len)?;
        if tag == 2 {
            return Ok(Some(read_u64(value).ok_or(Error::BadTimestamp)?));
        }
        offset = value_start + len;
    }
    Ok(None)
}

/// Encode a set into freshly allocated `bytes::Bytes` for callers that hand ownership
/// downstream. Available under the `bytes` feature.
#[cfg(feature = "bytes")]
pub fn encode_to_bytes(set: &St0601) -> Result<bytes::Bytes, EncodeError> {
    let mut out = Vec::new();
    encode(set, &mut out)?;
    Ok(bytes::Bytes::from(out))
}

/// Verify the UAS LS key and the outer BER length; return the `(first item offset,
/// end of items)` pair.
fn frame(input: &[u8]) -> Result<(usize, usize), Error> {
    if read_bytes(input, 0, 16)? != UAS_LS_KEY.as_slice() {
        return Err(Error::BadKey);
    }
    let (value_len, offset) = ber_decode_len(input, 16)?;
    let end = offset.checked_add(value_len).ok_or(Error::LengthOverflow)?;
    if end > input.len() {
        return Err(Error::Truncated {
            offset,
            needed: value_len,
            available: input.len().saturating_sub(offset),
        });
    }
    Ok((offset, end))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod encode_tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    use marlin_field::{Kind, RawCode};

    #[test]
    fn uas_ls_key_first_and_last_bytes() {
        assert_eq!(UAS_LS_KEY[0], 0x06);
        assert_eq!(UAS_LS_KEY[15], 0x00);
    }

    #[test]
    fn timestamp_and_version_encode_byte_exact() {
        let mut set = St0601::new(0x0001_0203_0405_0607);
        set.version = FieldState::Value(0x0B);
        let mut out = Vec::new();
        encode(&set, &mut out).expect("encode");
        assert_eq!(
            out,
            vec![
                0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00,
                0x00, 0x00, // UAS LS key (16)
                0x11, // outer BER length = 17
                0x02, 0x08, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // Tag 2 timestamp
                0x41, 0x01, 0x0B, // Tag 65 version
                0x01, 0x02, 0x71, 0xAC, // Tag 1 checksum
            ]
        );
    }

    #[test]
    fn encode_starts_with_uas_ls_key() {
        let mut out = Vec::new();
        encode(&St0601::new(0), &mut out).expect("encode");
        assert_eq!(&out[..16], &UAS_LS_KEY);
    }

    #[test]
    fn checksum_item_is_last_four_bytes() {
        let mut set = St0601::new(0x0001_0203_0405_0607);
        set.version = FieldState::Value(0x0B);
        let mut out = Vec::new();
        encode(&set, &mut out).expect("encode");
        let tail = &out[out.len() - 4..];
        assert_eq!(
            tail,
            &[0x01, 0x02, 0x71, 0xAC],
            "tag 1, len 2, big-endian bcc"
        );
    }

    #[test]
    fn encode_appends_without_disturbing_existing_bytes() {
        let mut out = vec![0xDE, 0xAD];
        encode(&St0601::new(0), &mut out).expect("encode");
        assert_eq!(&out[..2], &[0xDE, 0xAD], "pre-existing bytes untouched");
        assert_eq!(&out[2..18], &UAS_LS_KEY, "key written after existing bytes");
    }

    #[test]
    fn failed_encode_leaves_out_untouched() {
        let mut set = St0601::new(1);
        set.platform_heading_degrees = FieldState::Value(12.5);
        set.sensor_latitude_degrees = FieldState::Value(90.0001);
        let mut out = vec![0xDE, 0xAD];
        let err = encode(&set, &mut out).expect_err("latitude is out of range");
        assert_eq!(err, EncodeError::OutOfRange { tag: 13 });
        assert_eq!(out, vec![0xDE, 0xAD], "nothing written before the failure");
    }

    #[test]
    fn sender_error_on_version_is_unencodable() {
        let mut set = St0601::new(1);
        set.version = FieldState::SenderError(RawCode(0));
        let mut out = Vec::new();
        assert_eq!(
            encode(&set, &mut out),
            Err(EncodeError::Unencodable {
                tag: 65,
                kind: Kind::SenderError(RawCode(0))
            })
        );
    }

    #[test]
    fn unparsable_version_emits_nothing_for_tag_65() {
        let mut set = St0601::new(1);
        set.version = FieldState::Invalid(Invalid::Unparsable);
        let mut out = Vec::new();
        encode(&set, &mut out).expect("encode");
        let mut bare = Vec::new();
        encode(&St0601::new(1), &mut bare).expect("encode");
        assert_eq!(out, bare);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod decode_tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    use crate::testing::KlvBuilder;

    #[test]
    fn round_trip_preserves_all_typed_fields() {
        let mut set = St0601::new(0x1122_3344_5566_7788);
        set.version = FieldState::Value(11);
        set.platform_heading_degrees = FieldState::Value(360.0);
        set.platform_pitch_degrees = FieldState::Value(-20.0);
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        let decoded = decode(&buf).expect("decode");
        assert_eq!(decoded, set);
    }

    #[test]
    fn unknown_tags_survive_round_trip_in_order() {
        let mut set = St0601::new(7);
        set.unknown = vec![(0x70, vec![0xDE, 0xAD]), (0x71, vec![0x01])];
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        let decoded = decode(&buf).expect("decode");
        assert_eq!(
            decoded.unknown,
            vec![(0x70, vec![0xDE, 0xAD]), (0x71, vec![0x01])],
            "unknown tags preserved in original order"
        );
        assert_eq!(decoded, set);
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let packet = KlvBuilder::new()
            .timestamp(42)
            .version(1)
            .build_bad_checksum();
        let err = decode(&packet).expect_err("checksum no longer matches");
        assert!(matches!(err, Error::BadChecksum { .. }), "got {err:?}");
    }

    #[test]
    fn absent_checksum_is_missing_checksum() {
        // A keyed set whose only item is Tag 2, with no Tag 1 at all.
        let mut packet = UAS_LS_KEY.to_vec();
        packet.push(10);
        packet.extend_from_slice(&[0x02, 0x08, 0, 0, 0, 0, 0, 0, 0, 7]);
        assert_eq!(decode(&packet), Err(Error::MissingChecksum));
    }

    #[test]
    fn one_byte_checksum_is_missing_checksum() {
        let mut packet = UAS_LS_KEY.to_vec();
        packet.push(13);
        packet.extend_from_slice(&[0x02, 0x08, 0, 0, 0, 0, 0, 0, 0, 7]);
        packet.extend_from_slice(&[0x01, 0x01, 0x00]);
        assert_eq!(decode(&packet), Err(Error::MissingChecksum));
    }

    #[test]
    fn absent_timestamp_is_bad_timestamp() {
        let packet = KlvBuilder::new().version(9).build();
        assert_eq!(decode(&packet), Err(Error::BadTimestamp));
    }

    #[test]
    fn seven_byte_timestamp_is_bad_timestamp() {
        let packet = KlvBuilder::new().tag(2, &[0; 7]).build();
        assert_eq!(decode(&packet), Err(Error::BadTimestamp));
    }

    #[test]
    fn checksum_mismatch_outranks_a_bad_timestamp() {
        let absent = KlvBuilder::new().version(9).build_bad_checksum();
        assert!(matches!(decode(&absent), Err(Error::BadChecksum { .. })));
        let short = KlvBuilder::new().tag(2, &[0; 7]).build_bad_checksum();
        assert!(matches!(decode(&short), Err(Error::BadChecksum { .. })));
    }

    #[test]
    fn wrong_key_is_bad_key() {
        let mut packet = KlvBuilder::new().timestamp(1).build();
        packet[0] = 0x00; // break the UL key
        assert_eq!(decode(&packet), Err(Error::BadKey));
    }

    #[test]
    fn tag_65_wrong_length_is_unparsable_and_keeps_its_bytes() {
        // Tag 65 (UAS LS version) is a 1-byte tag; a wire length of 2 cannot be read
        // as the typed field, so the field is unparsable and the bytes are kept.
        let packet = KlvBuilder::new()
            .timestamp(7)
            .tag(65, &[0x0B, 0x00])
            .build();

        let decoded = decode(&packet).expect("decode");
        assert_eq!(decoded.version, FieldState::Invalid(Invalid::Unparsable));
        assert_eq!(decoded.unknown, vec![(65, vec![0x0B, 0x00])]);

        // Re-encoding reproduces the packet byte for byte: Tag 2, then the kept bytes
        // in `unknown`, then Tag 1.
        let mut re_encoded = Vec::new();
        encode(&decoded, &mut re_encoded).expect("re-encode");
        assert_eq!(
            re_encoded, packet,
            "tag 65 kept bytes must re-encode verbatim"
        );
    }

    #[test]
    fn malformed_inputs_error_without_panicking() {
        let wrong_key: [u8; 16] = [0xFF; 16];
        let cases: Vec<Vec<u8>> = vec![
            vec![],                 // empty
            vec![0x06, 0x0E, 0x2B], // partial key
            UAS_LS_KEY.to_vec(),    // key only, no outer length
            wrong_key.to_vec(),     // 16 bytes, wrong UL
            {
                let mut v = UAS_LS_KEY.to_vec();
                v.push(0x05); // outer len = 5 but no item bytes follow
                v
            },
            {
                let mut v = UAS_LS_KEY.to_vec();
                v.extend_from_slice(&[0x03, 0x02, 0x08, 0x00]); // Tag 2 claims 8 bytes, 1 present
                v
            },
            {
                let mut v = UAS_LS_KEY.to_vec();
                v.push(0x80); // indefinite outer length
                v
            },
            {
                let mut v = UAS_LS_KEY.to_vec();
                v.extend_from_slice(&[0x82, 0x01]); // long-form outer len missing a byte
                v
            },
        ];
        for case in &cases {
            assert!(
                decode(case).is_err(),
                "expected Err, got Ok for {case:02X?}"
            );
        }
    }

    #[test]
    fn item_claiming_more_bytes_than_the_input_is_truncated() {
        let mut v = UAS_LS_KEY.to_vec();
        v.extend_from_slice(&[0x03, 0x02, 0x08, 0x00]); // Tag 2 claims 8 bytes, 1 present
        assert!(matches!(decode(&v), Err(Error::Truncated { .. })));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod precision_timestamp_tests {
    use super::*;

    use crate::testing::KlvBuilder;

    #[test]
    fn present_timestamp_is_returned_without_full_decode() {
        let packet = KlvBuilder::new()
            .timestamp(0x0102_0304_0506_0708)
            .version(1)
            .build_bad_checksum();
        assert_eq!(
            precision_timestamp(&packet),
            Ok(Some(0x0102_0304_0506_0708))
        );
    }

    #[test]
    fn absent_timestamp_returns_none() {
        // A valid-keyed local set whose only item is Tag 65 (no Tag 2).
        let packet = KlvBuilder::new().version(9).build();
        assert_eq!(precision_timestamp(&packet), Ok(None));
    }

    #[test]
    fn wrong_length_timestamp_is_bad_timestamp() {
        let packet = KlvBuilder::new().tag(2, &[0; 4]).build();
        assert_eq!(precision_timestamp(&packet), Err(Error::BadTimestamp));
    }

    #[test]
    fn truncated_input_errors_without_panicking() {
        assert!(precision_timestamp(&[0x06, 0x0E]).is_err());
        assert!(precision_timestamp(&UAS_LS_KEY).is_err());
    }
}

#[cfg(all(test, feature = "bytes"))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
mod bytes_feature_tests {
    use super::*;

    #[test]
    fn encode_to_bytes_matches_encode_into_vec() {
        let mut set = St0601::new(5);
        set.version = FieldState::Value(1);
        let mut vec_out = Vec::new();
        encode(&set, &mut vec_out).expect("encode");
        let bytes_out = encode_to_bytes(&set).expect("encode_to_bytes");
        assert_eq!(bytes_out.as_ref(), vec_out.as_slice());
    }

    #[test]
    fn encode_to_bytes_reports_encode_errors() {
        let mut set = St0601::new(5);
        set.sensor_latitude_degrees = FieldState::Value(f64::NAN);
        assert_eq!(
            encode_to_bytes(&set),
            Err(EncodeError::OutOfRange { tag: 13 })
        );
    }
}
