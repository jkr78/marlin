"""Unit tests for marlin.klv (MISB ST 0601 KLV)."""

from __future__ import annotations

import pytest

from marlin.klv import (
    UAS_LS_KEY,
    KlvError,
    St0601,
    TagInfo,
    decode,
    encode,
    precision_timestamp,
    tag_name,
    tag_number,
    tags,
)


def test_round_trip_engineering_values() -> None:
    # Engineering setters are properties (attribute assignment), not methods.
    s = St0601(timestamp_us=1_700_000_000_000_000)
    s.sensor_latitude_degrees = 60.1768
    s.platform_heading_degrees = 159.97
    wire = encode(s)
    got = decode(wire)
    assert got.sensor_latitude_degrees == pytest.approx(60.1768, abs=1e-6)
    assert got.timestamp_us == 1_700_000_000_000_000


def test_raw_escape_hatch_round_trips() -> None:
    s = St0601(timestamp_us=1)
    s.raw_platform_heading = 0x71C2
    wire = encode(s)
    assert decode(wire).raw_platform_heading == 0x71C2
    # 0x71c2 = 29122 → 159.97436484321355° (klvdata KAT).
    assert decode(wire).platform_heading_degrees == pytest.approx(
        159.97436484321355, abs=1e-9
    )


def test_precision_timestamp_peek() -> None:
    s = St0601(timestamp_us=0x0102_0304_0506_0708)
    assert precision_timestamp(encode(s)) == 0x0102_0304_0506_0708


def test_bad_key_raises() -> None:
    with pytest.raises(KlvError):
        decode(b"\x00" * 20)


def test_signed_sentinel_reads_none() -> None:
    s = St0601(timestamp_us=1)
    s.raw_sensor_latitude = -2_147_483_648  # i32::MIN sentinel
    assert decode(encode(s)).sensor_latitude_degrees is None


def test_known_tag_does_not_leak_into_unknown() -> None:
    # A typed tag decodes into its field, never into `unknown`.
    s = St0601(timestamp_us=7)
    s.raw_platform_heading = 0x1234
    got = decode(encode(s))
    assert got.raw_platform_heading == 0x1234
    assert got.unknown == []


def test_unknown_tag_round_trips() -> None:
    # Golden packet: ts=7 plus unknown tags 0x70=[DE AD] and 0x71=[01],
    # with a correct BCC-16 (0x1D14). A tag this crate does not type surfaces
    # in `unknown` and re-encodes verbatim. (Python cannot construct unknown
    # tags — the `unknown` getter is read-only — so this drives a golden.)
    packet = bytes(
        [
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01,
            0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
            0x15,  # outer BER length
            0x02, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07,  # Tag 2, ts=7
            0x70, 0x02, 0xDE, 0xAD,  # unknown tag 0x70
            0x71, 0x01, 0x01,  # unknown tag 0x71
            0x01, 0x02, 0x1D, 0x14,  # Tag 1 checksum
        ]
    )
    got = decode(packet)
    assert got.timestamp_us == 7
    assert got.unknown == [(0x70, b"\xde\xad"), (0x71, b"\x01")]
    assert encode(got) == packet  # unknown tags preserved byte-exact


def test_uas_ls_key_frames_encoded_output() -> None:
    # The exported key is the exact 16-byte prefix of every encoded packet.
    assert isinstance(UAS_LS_KEY, bytes)
    assert len(UAS_LS_KEY) == 16
    wire = encode(St0601(timestamp_us=1))
    assert wire[:16] == UAS_LS_KEY


def test_tag_registry_covers_all_decodable_tags() -> None:
    registry = tags()
    numbers = [t.number for t in registry]
    assert len(registry) == 22  # 20 scaled + Tag 2 + Tag 65
    assert numbers == sorted(numbers), "ascending tag order"
    assert numbers[0] == 2 and numbers[-1] == 65
    assert 1 not in numbers, "Tag 1 checksum is framing, not a field"


def test_tag_info_fields_and_units() -> None:
    by_number = {t.number: t for t in tags()}
    lat = by_number[13]
    assert isinstance(lat, TagInfo)
    assert lat.name == "sensor_latitude"
    assert lat.unit == "degrees"
    assert by_number[2].name == "timestamp"
    assert by_number[2].unit == "microseconds"
    assert by_number[65].name == "version"
    assert by_number[65].unit is None


def test_tag_name_and_number_are_inverse() -> None:
    for t in tags():
        assert tag_number(t.name) == t.number
        assert tag_name(t.number) == t.name


def test_tag_lookups_return_none_for_unknown() -> None:
    assert tag_number("not_a_tag") is None
    assert tag_number("sensor_latitude_degrees") is None  # accessor, not field base
    assert tag_name(1) is None  # checksum tag
    assert tag_name(200) is None


def test_tag_info_is_hashable() -> None:
    # Frozen + hashable so census code can dedupe into a set.
    assert len(set(tags())) == len(tags())
