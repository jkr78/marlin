"""Unit tests for marlin.klv (MISB ST 0601 KLV)."""

from __future__ import annotations

import math

import pytest

from marlin.field import FieldState
from marlin.klv import (
    UAS_LS_KEY,
    KlvEncodeError,
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

TIMESTAMP_ITEM = bytes([0x02, 0x08, 0, 0, 0, 0, 0, 0, 0, 7])  # Tag 2, ts=7


def _bcc(data: bytes) -> int:
    # ST 0601 Tag 1: 16-bit running sum, even index -> high byte, odd -> low.
    total = 0
    for i, b in enumerate(data):
        total = (total + (b << (8 * ((i + 1) % 2)))) & 0xFFFF
    return total


def _packet(items: bytes, *, checksum: bool = True) -> bytes:
    # Frame raw items with the UAS LS key, a short-form outer length and a
    # correct Tag 1 checksum, so a test can place any tag with any length.
    body_len = len(items) + (4 if checksum else 0)
    head = UAS_LS_KEY + bytes([body_len]) + items
    if not checksum:
        return head
    head += bytes([0x01, 0x02])
    return head + _bcc(head).to_bytes(2, "big")


def test_round_trip_engineering_values() -> None:
    s = St0601(timestamp_us=1_700_000_000_000_000)
    s.sensor_latitude_degrees = FieldState.Value(60.1768)
    s.platform_heading_degrees = 159.97  # a bare number becomes Value
    got = decode(encode(s))
    assert isinstance(got.sensor_latitude_degrees, FieldState.Value)
    assert got.sensor_latitude_degrees.value == pytest.approx(60.1768, abs=1e-6)
    assert got.platform_heading_degrees.value == pytest.approx(159.97, abs=1e-2)
    assert got.timestamp_us == 1_700_000_000_000_000


def test_timestamp_us_is_required() -> None:
    with pytest.raises(TypeError, match="timestamp_us"):
        St0601()  # type: ignore[call-arg]


def test_version_is_a_field_state() -> None:
    assert St0601(timestamp_us=1).version == FieldState.NotAvailable()
    assert St0601(timestamp_us=1, version=11).version == FieldState.Value(11)
    s = St0601(timestamp_us=1)
    s.version = FieldState.Value(3)
    assert decode(encode(s)).version == FieldState.Value(3)
    s.version = None
    assert s.version == FieldState.NotAvailable()


def test_setter_coerces_none_and_bare_values() -> None:
    s = St0601(timestamp_us=1)
    s.platform_roll_degrees = -12.5
    assert s.platform_roll_degrees == FieldState.Value(-12.5)
    s.platform_roll_degrees = None
    assert s.platform_roll_degrees == FieldState.NotAvailable()
    s.platform_roll_degrees = FieldState.SenderError(-32768)
    assert s.platform_roll_degrees == FieldState.SenderError(-32768)


def test_setter_rejects_a_non_number() -> None:
    s = St0601(timestamp_us=1)
    with pytest.raises(TypeError, match="must be real number, not str"):
        s.platform_roll_degrees = "12.5"  # type: ignore[assignment]


def test_raw_properties_are_gone() -> None:
    s = St0601(timestamp_us=1)
    assert not hasattr(s, "raw_platform_heading")
    assert not hasattr(s, "raw_sensor_latitude")


def test_omitted_tag_is_not_available() -> None:
    got = decode(_packet(TIMESTAMP_ITEM))
    assert got.sensor_latitude_degrees == FieldState.NotAvailable()
    assert got.version == FieldState.NotAvailable()
    assert got.unknown == []


def test_signed_error_indicator_is_a_sender_error() -> None:
    # Tag 13 = i32::MIN on the wire.
    got = decode(_packet(TIMESTAMP_ITEM + bytes([13, 4, 0x80, 0, 0, 0])))
    assert got.sensor_latitude_degrees == FieldState.SenderError(-2_147_483_648)
    assert got.sensor_latitude_degrees.value is None


def test_sender_error_round_trips_through_encode() -> None:
    s = St0601(timestamp_us=1)
    s.sensor_latitude_degrees = FieldState.SenderError(-2_147_483_648)
    assert decode(encode(s)).sensor_latitude_degrees == FieldState.SenderError(
        -2_147_483_648
    )


def test_wrong_length_tag_is_invalid_and_keeps_its_bytes() -> None:
    # Tag 13 is a 4-byte tag; 3 bytes cannot be read.
    packet = _packet(TIMESTAMP_ITEM + bytes([13, 3, 0x01, 0x02, 0x03]))
    got = decode(packet)
    assert got.sensor_latitude_degrees == FieldState.Invalid(None)
    assert got.unknown == [(13, b"\x01\x02\x03")]
    assert encode(got) == packet


def test_missing_timestamp_raises_bad_timestamp() -> None:
    with pytest.raises(KlvError, match="Tag 2 precision timestamp absent") as ei:
        decode(_packet(bytes([65, 1, 9])))
    assert ei.value.variant == "bad_timestamp"


def test_missing_checksum_raises_missing_checksum() -> None:
    with pytest.raises(KlvError, match="Tag 1 checksum absent") as ei:
        decode(_packet(TIMESTAMP_ITEM, checksum=False))
    assert ei.value.variant == "missing_checksum"


def test_bad_key_raises() -> None:
    with pytest.raises(KlvError, match="local-set key is not the UAS Datalink LS UL") as ei:
        decode(b"\x00" * 20)
    assert ei.value.variant == "bad_key"


def test_corrupted_checksum_raises_bad_checksum() -> None:
    packet = bytearray(_packet(TIMESTAMP_ITEM))
    packet[-1] ^= 0xFF
    with pytest.raises(KlvError, match="checksum mismatch") as ei:
        decode(bytes(packet))
    assert ei.value.variant == "bad_checksum"


def test_klv_encode_error_is_a_klv_error() -> None:
    assert issubclass(KlvEncodeError, KlvError)


def test_over_range_bound_is_unencodable() -> None:
    s = St0601(timestamp_us=1)
    s.sensor_latitude_degrees = FieldState.AtLeast(90.0)
    with pytest.raises(KlvEncodeError, match="tag 13 cannot carry the at_least state") as ei:
        encode(s)
    assert ei.value.variant == "unencodable"
    assert ei.value.tag == 13
    assert ei.value.kind == "at_least"


def test_foreign_sender_error_code_is_unencodable() -> None:
    s = St0601(timestamp_us=1)
    s.platform_heading_degrees = FieldState.SenderError(-1)
    with pytest.raises(KlvEncodeError, match="tag 5 cannot carry the sender_error state") as ei:
        encode(s)
    assert ei.value.variant == "unencodable"
    assert ei.value.tag == 5
    assert ei.value.kind == "sender_error"


@pytest.mark.parametrize("bad", [90.0001, -90.0001, math.nan])
def test_value_outside_range_or_nan_is_out_of_range(bad: float) -> None:
    s = St0601(timestamp_us=1)
    s.sensor_latitude_degrees = bad
    with pytest.raises(KlvEncodeError, match="tag 13 value is outside") as ei:
        encode(s)
    assert ei.value.variant == "out_of_range"
    assert ei.value.tag == 13
    assert ei.value.kind is None


def test_range_ends_encode() -> None:
    s = St0601(timestamp_us=1)
    s.sensor_latitude_degrees = 90.0
    s.platform_heading_degrees = 360.0
    got = decode(encode(s))
    assert got.sensor_latitude_degrees == FieldState.Value(90.0)
    assert got.platform_heading_degrees == FieldState.Value(360.0)


def test_precision_timestamp_peek() -> None:
    s = St0601(timestamp_us=0x0102_0304_0506_0708)
    assert precision_timestamp(encode(s)) == 0x0102_0304_0506_0708


def test_known_tag_does_not_leak_into_unknown() -> None:
    # A typed tag with the right length decodes into its field, never into `unknown`.
    got = decode(_packet(TIMESTAMP_ITEM + bytes([5, 2, 0x71, 0xC2])))
    # 0x71c2 = 29122 -> 159.97436484321355 deg (klvdata KAT).
    assert got.platform_heading_degrees.value == pytest.approx(159.97436484321355, abs=1e-9)
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


def test_repr_prints_the_version_state() -> None:
    assert repr(St0601(timestamp_us=7, version=11)) == (
        "St0601(timestamp_us=7, version=FieldState.Value(11))"
    )


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
    assert tag_number("sensor_latitude_degrees") is None  # field, not base name
    assert tag_name(1) is None  # checksum tag
    assert tag_name(200) is None


def test_every_scaled_tag_has_a_property_named_from_the_registry() -> None:
    # The binding lists the twenty properties by hand; the codec's own registry
    # names the tags, so the two must agree. Tag 2 and Tag 65 are the framing fields.
    suffix = {"degrees": "_degrees", "meters": "_meters", "mps": "_mps"}
    scaled = [t for t in tags() if t.number not in (2, 65)]
    assert len(scaled) == 20
    s = St0601(timestamp_us=1)
    for t in scaled:
        assert t.unit is not None
        prop = t.name + suffix[t.unit]
        assert getattr(s, prop) == FieldState.NotAvailable(), prop


def test_tag_info_is_hashable() -> None:
    # Frozen + hashable so census code can dedupe into a set.
    assert len(set(tags())) == len(tags())
