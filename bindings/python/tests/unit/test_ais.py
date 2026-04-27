"""Unit tests for AIS data enums and value types."""

from __future__ import annotations

from unittest.mock import patch

import pytest

from marlin.ais import (
    AisMessage,
    AisParser,
    AisVersion,
    BitReader,
    Dimensions,
    EpfdType,
    Eta,
    ExtendedPositionReportB,
    ManeuverIndicator,
    NavStatus,
    Other,
    PositionReportA,
    PositionReportB,
    ReassemblyError,
    StaticAndVoyageA,
    StaticDataB24A,
    StaticDataB24B,
)


def _aivdm(
    frag_count: int,
    frag_num: int,
    seq_id: int | None,
    channel: str | None,
    payload: bytes,
    fill_bits: int,
) -> bytes:
    # Mirror of marlin_ais::testing::build_aivdm. Computes the XOR
    # envelope checksum over `AIVDM,<fields>` and wraps with !...*hh\r\n.
    parts = [b"AIVDM", str(frag_count).encode(), str(frag_num).encode()]
    parts.append(b"" if seq_id is None else str(seq_id).encode())
    parts.append(b"" if channel is None else channel.encode())
    parts.append(payload)
    parts.append(str(fill_bits).encode())
    body = b",".join(parts)
    x = 0
    for b in body:
        x ^= b
    return b"!" + body + b"*%02X\r\n" % x


# Known-good Type 1 (position report A) payload from the Rust crate's
# tests. Same payload appears at crates/marlin-ais/src/parser.rs:476.
_AIVDM_TYPE1 = _aivdm(1, 1, None, "A", b"13aGmP0P00PD;88MD5MTDww@2<0L", 0)


def test_nav_status_values() -> None:
    # Every variant is pinned to its wire value. The sparse jump from 8 to
    # 14 (and then 15) is the whole point — ITU-R M.1371 reserves 9..=13 as
    # payload bytes that upstream carries via NavStatus::Reserved(u8) and
    # that the Python enum can't represent (collapses to NOT_DEFINED at
    # the From<Rust> boundary).
    assert int(NavStatus.UNDERWAY_USING_ENGINE) == 0
    assert int(NavStatus.AT_ANCHOR) == 1
    assert int(NavStatus.NOT_UNDER_COMMAND) == 2
    assert int(NavStatus.RESTRICTED_MANEUVERABILITY) == 3
    assert int(NavStatus.CONSTRAINED_BY_DRAFT) == 4
    assert int(NavStatus.MOORED) == 5
    assert int(NavStatus.AGROUND) == 6
    assert int(NavStatus.ENGAGED_IN_FISHING) == 7
    assert int(NavStatus.UNDERWAY_SAILING) == 8
    assert int(NavStatus.AIS_SART_ACTIVE) == 14
    assert int(NavStatus.NOT_DEFINED) == 15


def test_maneuver_indicator_values() -> None:
    assert int(ManeuverIndicator.NOT_AVAILABLE) == 0
    assert int(ManeuverIndicator.NO_SPECIAL) == 1
    assert int(ManeuverIndicator.SPECIAL) == 2
    assert int(ManeuverIndicator.RESERVED) == 3


def test_epfd_type_values() -> None:
    # Same sparse-discriminant rationale as NavStatus: upstream reserves
    # 9..=14 as EpfdType::Reserved(u8) payload, so InternalGnss jumps to 15.
    assert int(EpfdType.UNDEFINED) == 0
    assert int(EpfdType.GPS) == 1
    assert int(EpfdType.GLONASS) == 2
    assert int(EpfdType.COMBINED_GPS_GLONASS) == 3
    assert int(EpfdType.LORAN_C) == 4
    assert int(EpfdType.CHAYKA) == 5
    assert int(EpfdType.INTEGRATED_NAVIGATION) == 6
    assert int(EpfdType.SURVEYED) == 7
    assert int(EpfdType.GALILEO) == 8
    assert int(EpfdType.INTERNAL_GNSS) == 15


def test_ais_version_values() -> None:
    assert int(AisVersion.ITU1371V1) == 0
    assert int(AisVersion.ITU1371V3) == 1
    assert int(AisVersion.ITU1371V5) == 2
    assert int(AisVersion.FUTURE) == 3


def test_dimensions_fields() -> None:
    d = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    assert d.to_bow_m == 10
    assert d.to_stern_m == 20
    assert d.to_port_m == 3
    assert d.to_starboard_m == 4


def test_dimensions_all_none() -> None:
    d = Dimensions()  # all sentinels -> all None
    assert d.to_bow_m is None
    assert d.to_stern_m is None
    assert d.to_port_m is None
    assert d.to_starboard_m is None


def test_dimensions_frozen() -> None:
    d = Dimensions(to_bow_m=10)
    with pytest.raises((AttributeError, TypeError)):
        d.to_bow_m = 20  # type: ignore[misc]


def test_dimensions_eq_hash() -> None:
    a = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    b = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    assert a == b
    assert hash(a) == hash(b)


def test_eta_fields() -> None:
    e = Eta(month=3, day=15, hour=12, minute=30)
    assert e.month == 3
    assert e.day == 15
    assert e.hour == 12
    assert e.minute == 30


def test_eta_all_none() -> None:
    e = Eta()
    assert e.month is None
    assert e.day is None
    assert e.hour is None
    assert e.minute is None


def test_eta_eq_hash() -> None:
    a = Eta(month=3, day=15, hour=12, minute=30)
    b = Eta(month=3, day=15, hour=12, minute=30)
    assert a == b
    assert hash(a) == hash(b)


def test_position_report_a_shape() -> None:
    p = PositionReportA(
        mmsi=123456789,
        navigation_status=NavStatus.UNDERWAY_USING_ENGINE,
        latitude_deg=48.5,
        longitude_deg=11.5,
        speed_over_ground=12.4,
        true_heading=90,
    )
    assert p.mmsi == 123456789
    assert p.navigation_status == NavStatus.UNDERWAY_USING_ENGINE
    assert p.latitude_deg == pytest.approx(48.5)
    assert p.longitude_deg == pytest.approx(11.5)
    assert p.speed_over_ground == pytest.approx(12.4)
    assert p.true_heading == 90
    # Defaults fire for unset fields:
    assert p.rate_of_turn is None
    assert p.position_accuracy is False
    assert p.special_maneuver == ManeuverIndicator.NOT_AVAILABLE
    assert p.timestamp == 60
    assert p.raim is False
    assert p.radio_status == 0


def test_static_and_voyage_a_shape() -> None:
    s = StaticAndVoyageA(
        mmsi=123456789,
        ais_version=AisVersion.ITU1371V5,
        imo_number=9074729,
        call_sign="ABCD",
        vessel_name="MY VESSEL",
        ship_type=70,  # cargo
        dimensions=Dimensions(to_bow_m=100, to_stern_m=20, to_port_m=10, to_starboard_m=10),
        epfd=EpfdType.GPS,
        eta=Eta(month=6, day=15, hour=14, minute=30),
        draught_m=8.5,
        destination="HAMBURG",
        dte=False,
    )
    assert s.mmsi == 123456789
    assert s.ais_version == AisVersion.ITU1371V5
    assert s.imo_number == 9074729
    assert s.call_sign == "ABCD"
    assert s.vessel_name == "MY VESSEL"
    assert s.ship_type == 70
    assert s.dimensions.to_bow_m == 100
    assert s.epfd == EpfdType.GPS
    assert s.eta.month == 6
    assert s.draught_m == pytest.approx(8.5)
    assert s.destination == "HAMBURG"
    assert s.dte is False


def test_position_report_b_shape() -> None:
    p = PositionReportB(
        mmsi=222333444,
        latitude_deg=48.5,
        speed_over_ground=5.0,
        class_b_cs_flag=True,
        class_b_message22_flag=True,
    )
    assert p.mmsi == 222333444
    assert p.latitude_deg == pytest.approx(48.5)
    assert p.speed_over_ground == pytest.approx(5.0)
    assert p.class_b_cs_flag is True
    assert p.class_b_message22_flag is True
    # Defaults:
    assert p.class_b_display_flag is False
    assert p.class_b_dsc_flag is False
    assert p.class_b_band_flag is False
    assert p.radio_status == 0


def test_extended_position_report_b_shape() -> None:
    p = ExtendedPositionReportB(
        mmsi=222333444,
        vessel_name="CLASS B",
        ship_type=37,
        dimensions=Dimensions(to_bow_m=15),
        epfd=EpfdType.GLONASS,
    )
    assert p.mmsi == 222333444
    assert p.vessel_name == "CLASS B"
    assert p.ship_type == 37
    assert p.dimensions.to_bow_m == 15
    assert p.epfd == EpfdType.GLONASS
    assert p.timestamp == 60
    assert p.assigned_flag is False


def test_static_data_b24a_shape() -> None:
    s = StaticDataB24A(mmsi=222333444, vessel_name="NAMED")
    assert s.mmsi == 222333444
    assert s.vessel_name == "NAMED"


def test_static_data_b24b_shape() -> None:
    s = StaticDataB24B(
        mmsi=222333444,
        ship_type=37,
        vendor_id="VND1",
        call_sign="CS1",
        dimensions=Dimensions(to_bow_m=12),
    )
    assert s.mmsi == 222333444
    assert s.ship_type == 37
    assert s.vendor_id == "VND1"
    assert s.call_sign == "CS1"
    assert s.dimensions.to_bow_m == 12


def test_other_shape() -> None:
    o = Other(msg_type=9, raw_payload=b"\x01\x02\x03", total_bits=24)
    assert o.msg_type == 9
    assert o.raw_payload == b"\x01\x02\x03"
    assert o.total_bits == 24


def test_position_report_a_all_defaults() -> None:
    # Constructor works with no args — every field has a default.
    p = PositionReportA()
    assert p.mmsi == 0
    assert p.navigation_status == NavStatus.NOT_DEFINED
    assert p.special_maneuver == ManeuverIndicator.NOT_AVAILABLE
    assert p.timestamp == 60


def test_ais_message_class_exists() -> None:
    # Plan's minimal sanity check.
    assert AisMessage.__name__ == "AisMessage"


def test_ais_message_construct_type1() -> None:
    body = PositionReportA(mmsi=123456789, latitude_deg=48.5)
    msg = AisMessage(is_own_ship=False, type_tag="type1", body=body)
    assert msg.is_own_ship is False
    assert msg.type_tag == "type1"
    # body getter returns a reference to the same body instance (Py<PyAny>
    # clone_ref bumps refcount, does not deep-copy).
    assert msg.body is body


def test_ais_message_construct_own_ship() -> None:
    body = StaticDataB24A(mmsi=222333444, vessel_name="OWN")
    msg = AisMessage(is_own_ship=True, type_tag="type24a", body=body)
    assert msg.is_own_ship is True
    assert msg.type_tag == "type24a"
    assert isinstance(msg.body, StaticDataB24A)
    assert msg.body.vessel_name == "OWN"


def test_ais_message_body_with_other_variant() -> None:
    body = Other(msg_type=9, raw_payload=b"\x01\x02", total_bits=16)
    msg = AisMessage(is_own_ship=False, type_tag="other", body=body)
    assert msg.type_tag == "other"
    assert isinstance(msg.body, Other)
    assert msg.body.msg_type == 9


def test_ais_message_repr() -> None:
    msg = AisMessage(
        is_own_ship=True,
        type_tag="type1",
        body=PositionReportA(mmsi=1),
    )
    # Pin the exact format: `{:?}` on String gives quoted, `{}` on bool gives
    # lowercase `true`/`false`. A substring check would silently accept a
    # regression that drops either the label or the quotes.
    assert repr(msg) == 'AisMessage(type_tag="type1", is_own_ship=true)'


def test_bit_reader_basic() -> None:
    # 0b10101010 0b11110000 → first bit 1, next 7 bits 0101010, then 8 remain.
    data = bytes([0b10101010, 0b11110000])
    reader = BitReader(data, total_bits=16)
    assert reader.u(1) == 1
    assert reader.u(7) == 0b0101010
    assert reader.remaining() == 8


def test_bit_reader_signed() -> None:
    # Upper 6 bits of 0b11111100 = 0b111111 = -1 in 6-bit two's complement.
    data = bytes([0b11111100])
    reader = BitReader(data, total_bits=6)
    assert reader.i(6) == -1


def test_bit_reader_past_end() -> None:
    # Total bits 4, buffer has 8; read past the declared end saturates to 0.
    reader = BitReader(bytes([0xFF]), total_bits=4)
    assert reader.u(4) == 0b1111
    assert reader.u(4) == 0  # past-end → 0
    assert reader.remaining() == 0


def test_bit_reader_bool() -> None:
    # 0b10000000: first bit True, next bit False.
    reader = BitReader(bytes([0b10000000]), total_bits=8)
    assert reader.b() is True
    assert reader.b() is False


def test_bit_reader_string_6bit() -> None:
    # "AB" as two 6-bit AIS chars = 000001 000010, packed into bytes.
    # 000001 000010 00xxxx xxxxxxxx (padding) = 0b00000100 0b00100000
    reader = BitReader(bytes([0b00000100, 0b00100000]), total_bits=12)
    assert reader.string(2) == "AB"


def test_bit_reader_string_preserves_at_padding() -> None:
    # Upstream marlin_ais::BitReader::string does NOT trim trailing '@'
    # (which is AIS 6-bit code 0). The typed message decoders (e.g.
    # StaticAndVoyageA.vessel_name) are the layer that trims. This test
    # pins the primitive's raw-preservation behavior.
    # "A@@@" = 000001 000000 000000 000000 = 24 bits packed big-endian.
    reader = BitReader(bytes([0b00000100, 0b00000000, 0b00000000]), total_bits=24)
    assert reader.string(4) == "A@@@"


def test_bit_reader_read_across_byte_boundary() -> None:
    # Two bytes 0xAB 0xCD = 1010 1011 1100 1101. Read 4 bits, then 12
    # bits (spans the byte boundary: last 4 bits of byte 0 + all of byte 1).
    reader = BitReader(bytes([0xAB, 0xCD]), total_bits=16)
    assert reader.u(4) == 0xA       # 1010
    assert reader.u(12) == 0xBCD    # 1011 1100 1101
    assert reader.remaining() == 0


def test_ais_streaming_single_fragment() -> None:
    p = AisParser.streaming()
    p.feed(_AIVDM_TYPE1)
    messages = list(p)
    assert len(messages) == 1
    assert isinstance(messages[0], AisMessage)
    assert isinstance(messages[0].body, PositionReportA)
    assert messages[0].type_tag == "type1"


def test_ais_auto_clock_reads_time() -> None:
    # timeout_ms triggers the "auto" clock path. Patch time.monotonic_ns
    # and assert the parser calls it.
    with patch("time.monotonic_ns", return_value=0) as mock_now:
        p = AisParser.streaming(timeout_ms=60_000)  # clock="auto" default
        p.feed(_AIVDM_TYPE1)
        list(p)
        assert mock_now.called


def test_ais_manual_clock_never_reads_time() -> None:
    # CRITICAL: the "manual" clock must never touch time.monotonic_ns.
    # This is the testability contract that lets users replay historical
    # AIS data deterministically without wall-clock interference.
    with patch("time.monotonic_ns") as mock_now:
        p = AisParser.streaming(timeout_ms=60_000, clock="manual")
        p.tick(now_ms=1_000_000)
        p.feed(_AIVDM_TYPE1)
        list(p)
        mock_now.assert_not_called()


def test_ais_tick_on_auto_raises() -> None:
    p = AisParser.streaming(timeout_ms=60_000, clock="auto")
    with pytest.raises(ValueError):
        p.tick(now_ms=1000)


def test_ais_reassembly_out_of_order_raises() -> None:
    # Feed part 2 of a 2-fragment message without part 1 — the
    # reassembler emits ReassemblyError (subclass of AisError). Strict
    # iteration surfaces it; lenient would swallow.
    frag2 = _aivdm(2, 2, 1, "A", b"XXXXXXX", 0)
    p = AisParser.streaming()
    p.feed(frag2)
    with pytest.raises(ReassemblyError):
        list(p.iter(strict=True))
