use alloc::vec::Vec;

use crate::ber::{be_u16, be_u64, ber_decode_len, ber_encode_len, read_bytes};
use crate::checksum::bcc;
use crate::error::Error;

/// MISB ST 0601 UAS Datalink Local Set 16-byte universal label (SMPTE UL key).
pub const UAS_LS_KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01, 0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
];

/// MISB ST 0601 UAS Datalink Local Set — the typed core. Unrecognized 1-byte tags round-trip
/// via [`St0601::unknown`], so broadening the typed set is additive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct St0601 {
    /// Tag 2: precision timestamp, microseconds since the UNIX epoch (UTC). Mandatory —
    /// always encoded, always present in a successfully decoded set.
    pub timestamp_us: u64,
    /// Tag 65: UAS LS document version number.
    pub version: Option<u8>,
    /// Tag 5: platform heading, raw wire count (u16, 0..360° linear, full unsigned range).
    pub platform_heading: Option<u16>,
    /// Tag 6: platform pitch, raw wire count (i16, ±20° linear; `0x8000` = error sentinel).
    pub platform_pitch: Option<i16>,
    /// Tag 7: platform roll angle, raw wire count (i16, ±50° linear; `0x8000` = error sentinel).
    pub platform_roll: Option<i16>,
    /// Tag 8: platform true airspeed, raw wire count (u8, identity m/s).
    pub platform_true_airspeed: Option<u8>,
    /// Tag 13: sensor latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub sensor_latitude: Option<i32>,
    /// Tag 14: sensor longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub sensor_longitude: Option<i32>,
    /// Tag 15: sensor true altitude, raw wire count (u16, linear −900..19000 m MSL).
    pub sensor_true_altitude: Option<u16>,
    /// Tag 16: sensor horizontal field of view, raw wire count (u16, 0..180° linear).
    pub sensor_horizontal_fov: Option<u16>,
    /// Tag 17: sensor vertical field of view, raw wire count (u16, 0..180° linear).
    pub sensor_vertical_fov: Option<u16>,
    /// Tag 18: sensor relative azimuth, raw wire count (u32, 0..360° linear, full unsigned range).
    pub sensor_relative_azimuth: Option<u32>,
    /// Tag 19: sensor relative elevation, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub sensor_relative_elevation: Option<i32>,
    /// Tag 20: sensor relative roll, raw wire count (u32, 0..360° linear, full unsigned range).
    pub sensor_relative_roll: Option<u32>,
    /// Tag 21: slant range, raw wire count (u32, 0..5,000,000 m linear).
    pub slant_range: Option<u32>,
    /// Tag 22: target width, raw wire count (u16, 0..10,000 m linear).
    pub target_width: Option<u16>,
    /// Tag 23: frame center latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub frame_center_latitude: Option<i32>,
    /// Tag 24: frame center longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub frame_center_longitude: Option<i32>,
    /// Tag 25: frame center elevation, raw wire count (u16, linear −900..19000 m MSL).
    pub frame_center_elevation: Option<u16>,
    /// Tag 40: target location latitude, raw wire count (i32, ±90° linear; `0x80000000` = error sentinel).
    pub target_location_latitude: Option<i32>,
    /// Tag 41: target location longitude, raw wire count (i32, ±180° linear; `0x80000000` = error sentinel).
    pub target_location_longitude: Option<i32>,
    /// Tag 42: target location elevation, raw wire count (u16, linear −900..19000 m MSL).
    pub target_location_elevation: Option<u16>,
    /// Tags this crate does not type, preserved verbatim as `(tag, value)` in wire order
    /// and re-emitted on encode. A known tag decoded with an unexpected wire length also
    /// lands here (tolerant decode). Tag 2 (mandatory timestamp) is never routed here —
    /// a malformed Tag 2 fails the whole decode instead.
    pub unknown: Vec<(u8, Vec<u8>)>,
}

/// Encode a `St0601` set into `out` (appended): UAS LS key, outer BER length, then items —
/// Tag 2 (mandatory timestamp) first, optional typed tags, preserved unknown tags, and the
/// Tag 1 checksum last. `out` may be non-empty; the checksum covers only this call's bytes.
pub fn encode(set: &St0601, out: &mut Vec<u8>) -> Result<(), Error> {
    let start = out.len();

    let mut items: Vec<u8> = Vec::new();
    // Tag 2 Precision Time Stamp — mandatory, prepended first.
    items.push(2);
    ber_encode_len(8, &mut items);
    items.extend_from_slice(&set.timestamp_us.to_be_bytes());
    // Tag 65 UAS LS Version.
    if let Some(version) = set.version {
        items.push(65);
        ber_encode_len(1, &mut items);
        items.push(version);
    }
    crate::tags::encode_scaled(set, &mut items);
    // Unrecognized tags, original order.
    for (tag, value) in &set.unknown {
        items.push(*tag);
        ber_encode_len(value.len(), &mut items);
        items.extend_from_slice(value);
    }

    // Outer length covers all items plus the 4-byte Tag 1 checksum item appended below.
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

/// Decode an ST 0601 local set: verify the UAS LS key, walk every TLV item into typed fields
/// (unknown tags preserved in order), then verify the embedded Tag 1 checksum.
pub fn decode(input: &[u8]) -> Result<St0601, Error> {
    if read_bytes(input, 0, 16)? != UAS_LS_KEY.as_slice() {
        return Err(Error::BadKey);
    }

    let (value_len, mut offset) = ber_decode_len(input, 16)?;
    let end = offset.checked_add(value_len).ok_or(Error::LengthOverflow)?;
    if end > input.len() {
        return Err(Error::Truncated {
            offset,
            needed: value_len,
            available: input.len().saturating_sub(offset),
        });
    }

    let mut set = St0601::default();
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
            1 => checksum = Some((be_u16(value)?, value_start)),
            2 => set.timestamp_us = be_u64(value)?,
            65 => match value {
                [b] => set.version = Some(*b),
                _ => set.unknown.push((65, value.to_vec())),
            },
            other => {
                if !crate::tags::decode_scaled(other, value, &mut set) {
                    set.unknown.push((other, value.to_vec()));
                }
            }
        }
        offset = value_start + len;
    }

    // Tag 1 is mandatory and always last; verify it over everything up to its value bytes.
    let (embedded, value_start) = checksum.ok_or(Error::BadChecksum {
        computed: 0,
        embedded: 0,
    })?;
    let computed = bcc(input.get(..value_start).unwrap_or(&[]));
    if computed != embedded {
        return Err(Error::BadChecksum { computed, embedded });
    }
    Ok(set)
}

/// Cheap Tag 2 peek: skip the key + outer length, scan items for Tag 2, return its
/// microsecond value. Returns `Ok(None)` when absent. Does NOT verify the checksum.
pub fn precision_timestamp(input: &[u8]) -> Result<Option<u64>, Error> {
    if read_bytes(input, 0, 16)? != UAS_LS_KEY.as_slice() {
        return Err(Error::BadKey);
    }
    let (value_len, mut offset) = ber_decode_len(input, 16)?;
    let end = offset.checked_add(value_len).ok_or(Error::LengthOverflow)?;
    if end > input.len() {
        return Err(Error::Truncated {
            offset,
            needed: value_len,
            available: input.len().saturating_sub(offset),
        });
    }
    while offset < end {
        let tag = *input.get(offset).ok_or(Error::Truncated {
            offset,
            needed: 1,
            available: 0,
        })?;
        let (len, value_start) = ber_decode_len(input, offset + 1)?;
        let value = read_bytes(input, value_start, len)?;
        if tag == 2 {
            return Ok(Some(be_u64(value)?));
        }
        offset = value_start + len;
    }
    Ok(None)
}

/// Encode a set into freshly allocated `bytes::Bytes` for callers that hand ownership
/// downstream. Available under the `bytes` feature.
#[cfg(feature = "bytes")]
pub fn encode_to_bytes(set: &St0601) -> Result<bytes::Bytes, Error> {
    let mut out = Vec::new();
    encode(set, &mut out)?;
    Ok(bytes::Bytes::from(out))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::indexing_slicing, clippy::cast_possible_truncation
)]
mod encode_tests {
    use alloc::{vec, vec::Vec};
    use super::*;

    #[test]
    fn uas_ls_key_first_and_last_bytes() {
        assert_eq!(UAS_LS_KEY[0], 0x06);
        assert_eq!(UAS_LS_KEY[15], 0x00);
    }

    #[test]
    fn timestamp_and_version_encode_byte_exact() {
        let set = St0601 {
            timestamp_us: 0x0001_0203_0405_0607,
            version: Some(0x0B),
            ..St0601::default()
        };
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
        encode(&St0601::default(), &mut out).expect("encode");
        assert_eq!(&out[..16], &UAS_LS_KEY);
    }

    #[test]
    fn checksum_item_is_last_four_bytes() {
        let set = St0601 {
            timestamp_us: 0x0001_0203_0405_0607,
            version: Some(0x0B),
            ..St0601::default()
        };
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
        encode(&St0601::default(), &mut out).expect("encode");
        assert_eq!(&out[..2], &[0xDE, 0xAD], "pre-existing bytes untouched");
        assert_eq!(&out[2..18], &UAS_LS_KEY, "key written after existing bytes");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::indexing_slicing, clippy::cast_possible_truncation
)]
mod decode_tests {
    use alloc::{vec, vec::Vec};
    use super::*;

    #[test]
    fn round_trip_preserves_all_typed_fields() {
        let set = St0601 {
            timestamp_us: 0x1122_3344_5566_7788,
            version: Some(11),
            platform_heading: Some(0xBEEF),
            platform_pitch: Some(-1000),
            ..Default::default()
        };
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        let decoded = decode(&buf).expect("decode");
        assert_eq!(decoded, set);
    }

    #[test]
    fn unknown_tags_survive_round_trip_in_order() {
        let set = St0601 {
            timestamp_us: 7,
            unknown: vec![(0x70, vec![0xDE, 0xAD]), (0x71, vec![0x01])],
            ..Default::default()
        };
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
        let set = St0601 {
            timestamp_us: 42,
            version: Some(1),
            ..St0601::default()
        };
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        let last = buf.len() - 1;
        buf[last] ^= 0xFF; // flip the low checksum byte
        let err = decode(&buf).expect_err("checksum no longer matches");
        assert!(matches!(err, Error::BadChecksum { .. }), "got {err:?}");
    }

    #[test]
    fn wrong_key_is_bad_key() {
        let set = St0601 {
            timestamp_us: 1,
            ..St0601::default()
        };
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        buf[0] = 0x00; // break the UL key
        assert_eq!(decode(&buf), Err(Error::BadKey));
    }

    #[test]
    fn tag_65_wrong_length_falls_back_to_unknown() {
        // Tag 65 (UAS LS version) is a 1-byte tag; a wire length of 2 is malformed for
        // the typed field, so it must land in `unknown` (tolerant decode) rather than
        // truncating the extra byte or erroring.
        let mut packet = UAS_LS_KEY.to_vec();
        let mut items = vec![2u8, 0x08]; // Tag 2, len 8
        items.extend_from_slice(&7u64.to_be_bytes());
        items.extend_from_slice(&[0x41, 0x02, 0x0B, 0x00]); // Tag 65, len 2 (wrong)
        let value_len = items.len() + 4; // + Tag 1 checksum item
        packet.push(value_len as u8);
        packet.append(&mut items);
        packet.push(1);
        packet.push(2);
        let checksum = bcc(&packet);
        packet.extend_from_slice(&checksum.to_be_bytes());

        let decoded = decode(&packet).expect("decode");
        assert_eq!(decoded.version, None);
        assert_eq!(decoded.unknown, vec![(65, vec![0x0B, 0x00])]);

        // Re-encoding must reproduce the exact spliced bytes (only Tag 2 + Tag 65 present,
        // same order the encoder emits: Tag 2, typed, unknown, Tag 1 — so the whole packet
        // round-trips byte-for-byte, not just semantically).
        let mut re_encoded = Vec::new();
        encode(&decoded, &mut re_encoded).expect("re-encode");
        assert_eq!(
            re_encoded, packet,
            "tag 65 unknown bytes must re-encode verbatim"
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
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::indexing_slicing, clippy::cast_possible_truncation
)]
mod precision_timestamp_tests {
    use alloc::{vec, vec::Vec};
    use super::*;

    #[test]
    fn present_timestamp_is_returned_without_full_decode() {
        let set = St0601 {
            timestamp_us: 0x0102_0304_0506_0708,
            version: Some(1),
            ..St0601::default()
        };
        let mut buf = Vec::new();
        encode(&set, &mut buf).expect("encode");
        assert_eq!(precision_timestamp(&buf), Ok(Some(0x0102_0304_0506_0708)));
    }

    #[test]
    fn absent_timestamp_returns_none() {
        // A valid-keyed local set whose only item is Tag 65 (no Tag 2).
        let mut packet = UAS_LS_KEY.to_vec();
        let mut items = vec![65u8, 0x01, 0x09]; // Tag 65, len 1, version 9
        let value_len = items.len() + 4; // + Tag 1 checksum item
        packet.push(value_len as u8);
        packet.append(&mut items);
        packet.push(1);
        packet.push(2);
        let checksum = bcc(&packet);
        packet.extend_from_slice(&checksum.to_be_bytes());
        assert_eq!(precision_timestamp(&packet), Ok(None));
    }

    #[test]
    fn truncated_input_errors_without_panicking() {
        assert!(precision_timestamp(&[0x06, 0x0E]).is_err());
        assert!(precision_timestamp(&UAS_LS_KEY).is_err());
    }
}

#[cfg(all(test, feature = "bytes"))]
#[allow(
    clippy::unwrap_used, clippy::expect_used, clippy::panic,
    clippy::indexing_slicing, clippy::cast_possible_truncation
)]
mod bytes_feature_tests {
    use super::*;

    #[test]
    fn encode_to_bytes_matches_encode_into_vec() {
        let set = St0601 {
            timestamp_us: 5,
            version: Some(1),
            ..St0601::default()
        };
        let mut vec_out = Vec::new();
        encode(&set, &mut vec_out).expect("encode");
        let bytes_out = encode_to_bytes(&set).expect("encode_to_bytes");
        assert_eq!(bytes_out.as_ref(), vec_out.as_slice());
    }
}

