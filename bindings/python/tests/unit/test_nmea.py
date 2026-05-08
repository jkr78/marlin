"""Unit tests for typed NMEA 0183 message variants."""

import pytest

from marlin.envelope import EnvelopeError, parse
from marlin.nmea import (
    DataStatus,
    DecodeError,
    DecodeOptions,
    Gga,
    GgaFixQuality,
    Gll,
    Hdt,
    Nmea0183Parser,
    Prdid,
    PrdidDialect,
    PrdidPitchRollHeading,
    PrdidRaw,
    PrdidRollPitchHeading,
    Psxn,
    PsxnLayout,
    PsxnSlot,
    Rmc,
    RmcNavStatus,
    Unknown,
    UtcDate,
    UtcTime,
    Vtg,
    VtgMode,
    decode,
    decode_gga,
    decode_gll,
    decode_hdt,
    decode_prdid,
    decode_psxn,
    decode_rmc,
    decode_vtg,
    decode_with,
)


def _with_checksum(body: bytes, terminator: bytes = b"\r\n") -> bytes:
    x = 0
    for b in body:
        x ^= b
    return b"$" + body + b"*%02X" % x + terminator


GGA_SENTENCE = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
PSXN_SENTENCE = _with_checksum(b"PSXN,23,1.5,-2.5,0.1,,,,")
PRDID_SENTENCE = _with_checksum(b"PRDID,1.2,3.4,5.6")


def test_gga_fields_shape() -> None:
    g = Gga(
        talker=b"GP",
        utc=UtcTime(12, 35, 19, 0),
        latitude_deg=48.1173,
        longitude_deg=11.516666,
        fix_quality=GgaFixQuality.GPS_FIX,
        satellites_used=8,
        hdop=0.9,
        altitude_m=545.4,
        geoid_separation_m=46.9,
        dgps_age_s=None,
        dgps_station_id=None,
    )
    assert g.talker == b"GP"
    assert g.fix_quality == GgaFixQuality.GPS_FIX
    assert g.latitude_deg == pytest.approx(48.1173)


def test_gga_fix_quality_int_compat() -> None:
    assert GgaFixQuality.GPS_FIX == 1
    assert int(GgaFixQuality.RTK_FIXED) == 4


def test_vtg_mode_int_compat() -> None:
    # PyO3 `eq_int` enums compare equal to int and round-trip via `int()`
    # but do not expose `.value` (unlike `enum.IntEnum`).
    assert int(VtgMode.AUTONOMOUS) >= 0
    assert VtgMode.AUTONOMOUS == 1


def test_unknown_variant_fields() -> None:
    u = Unknown(talker=b"IN", sentence_type="FOO")
    assert u.talker == b"IN"
    assert u.sentence_type == "FOO"


def test_gga_frozen() -> None:
    g = Gga(
        talker=b"GP",
        utc=None,
        latitude_deg=None,
        longitude_deg=None,
        fix_quality=GgaFixQuality.INVALID,
        satellites_used=None,
        hdop=None,
        altitude_m=None,
        geoid_separation_m=None,
        dgps_age_s=None,
        dgps_station_id=None,
    )
    with pytest.raises((AttributeError, TypeError)):
        g.talker = b"XX"  # type: ignore[misc]


# Additional smoke tests — mechanically obvious, anchor Psxn/Prdid scaffolds
# so they don't silently break during Task 7 refactoring.


def test_vtg_constructs_and_reads() -> None:
    v = Vtg(
        talker=b"GP",
        course_true_deg=123.4,
        course_magnetic_deg=None,
        speed_knots=5.5,
        speed_kmh=10.2,
        mode=VtgMode.AUTONOMOUS,
    )
    assert v.talker == b"GP"
    assert v.mode == VtgMode.AUTONOMOUS
    assert v.speed_knots == pytest.approx(5.5)


def test_hdt_constructs_and_reads() -> None:
    h = Hdt(talker=b"IN", heading_true_deg=180.25)
    assert h.talker == b"IN"
    assert h.heading_true_deg == pytest.approx(180.25)


def test_psxn_layout_from_str_rphx() -> None:
    layout = PsxnLayout.from_str("rphx")
    assert layout is not None


def test_psxn_layout_from_str_with_radians_flag() -> None:
    layout = PsxnLayout.from_str("rphx1")
    assert layout is not None  # `1` sets raw_radians; successful parse


def test_psxn_layout_from_str_invalid() -> None:
    with pytest.raises(ValueError):
        PsxnLayout.from_str("not-a-layout-@@@")


def test_decode_options_builder() -> None:
    opts = (
        DecodeOptions()
        .with_psxn_layout(PsxnLayout.from_str("rphx"))
        .with_prdid_dialect(PrdidDialect.PITCH_ROLL_HEADING)
    )
    assert opts is not None


def test_prdid_dialect_enum_values() -> None:
    assert int(PrdidDialect.UNKNOWN) == 0
    assert int(PrdidDialect.PITCH_ROLL_HEADING) == 1
    assert int(PrdidDialect.ROLL_PITCH_HEADING) == 2


def test_psxn_slot_enum_values() -> None:
    assert int(PsxnSlot.ROLL) == 0
    assert int(PsxnSlot.IGNORED) == 5


def test_psxn_fields() -> None:
    p = Psxn(
        id=23,
        token=b"abc",
        roll_deg=1.5,
        pitch_deg=-2.5,
        heave_m=0.1,
    )
    assert p.id == 23
    assert p.token == b"abc"
    assert p.roll_deg == pytest.approx(1.5)
    assert p.pitch_deg == pytest.approx(-2.5)
    assert p.heave_m == pytest.approx(0.1)


def test_psxn_default_all_none() -> None:
    p = Psxn()
    assert p.id is None
    assert p.token is None
    assert p.roll_deg is None


def test_prdid_raw_round_trip() -> None:
    p = Prdid.raw(fields=[b"1.2", b"3.4", b"5.6"])
    assert p.variant == "raw"
    assert isinstance(p.body, PrdidRaw)
    assert p.body.fields == (b"1.2", b"3.4", b"5.6")


def test_prdid_pitch_roll_heading_round_trip() -> None:
    p = Prdid.pitch_roll_heading(pitch_deg=1.0, roll_deg=2.0, heading_deg=180.0)
    assert p.variant == "pitch_roll_heading"
    assert isinstance(p.body, PrdidPitchRollHeading)
    assert p.body.pitch_deg == pytest.approx(1.0)
    assert p.body.heading_deg == pytest.approx(180.0)


def test_prdid_roll_pitch_heading_round_trip() -> None:
    p = Prdid.roll_pitch_heading(roll_deg=2.0, pitch_deg=1.0, heading_deg=180.0)
    assert p.variant == "roll_pitch_heading"
    assert isinstance(p.body, PrdidRollPitchHeading)


def test_utc_time_repr_and_fields() -> None:
    t = UtcTime(9, 27, 50, 123)
    assert t.hour == 9
    assert t.minute == 27
    assert t.second == 50
    assert t.millisecond == 123
    assert repr(t) == "UtcTime(09:27:50.123)"


def test_streaming_parser_yields_gga() -> None:
    p = Nmea0183Parser.streaming()
    p.feed(GGA_SENTENCE)
    messages = list(p)
    assert len(messages) == 1
    assert isinstance(messages[0], Gga)
    assert messages[0].talker == b"GP"


def test_one_shot_parser_with_options() -> None:
    opts = DecodeOptions().with_psxn_layout(PsxnLayout.from_str("rphx"))
    p = Nmea0183Parser.one_shot(opts)
    p.feed(GGA_SENTENCE)
    msg = p.next_message()
    assert isinstance(msg, Gga)


def test_next_message_raises_on_bad_checksum() -> None:
    p = Nmea0183Parser.one_shot()
    p.feed(b"$GPGGA,badchecksum*FF\r\n")
    with pytest.raises(EnvelopeError):
        p.next_message()


def test_streaming_parser_strict_iteration_raises() -> None:
    # Lenient iteration (default) swallows errors; strict mode surfaces them.
    p = Nmea0183Parser.streaming()
    p.feed(b"$GPGGA,badchecksum*FF\r\n")
    with pytest.raises(EnvelopeError):
        list(p.iter(strict=True))


def test_streaming_parser_returns_none_when_empty() -> None:
    p = Nmea0183Parser.streaming()
    assert p.next_message() is None


def test_decode_raw_sentence_gga() -> None:
    raw = parse(GGA_SENTENCE)
    msg = decode(raw)
    assert isinstance(msg, Gga)
    assert msg.talker == b"GP"


def test_decode_with_options() -> None:
    opts = DecodeOptions().with_prdid_dialect(PrdidDialect.UNKNOWN)
    raw = parse(GGA_SENTENCE)
    msg = decode_with(raw, opts)
    assert isinstance(msg, Gga)


def test_decode_gga_directly() -> None:
    raw = parse(GGA_SENTENCE)
    gga = decode_gga(raw)
    assert isinstance(gga, Gga)
    assert gga.talker == b"GP"


def test_decode_vtg_wrong_type_raises() -> None:
    # GGA sentence through decode_vtg is a decode-level mismatch.
    raw = parse(GGA_SENTENCE)
    with pytest.raises(DecodeError):
        decode_vtg(raw)


def test_decode_hdt_wrong_type_raises() -> None:
    # HDT decoder only needs >=2 fields and a parseable float in field[0].
    # A GGA sentence's field[0] ("123519") parses cleanly, so we use a
    # single-field sentence to exercise the NotEnoughFields error path.
    raw = parse(_with_checksum(b"GPHDT"))
    with pytest.raises(DecodeError):
        decode_hdt(raw)


def test_decode_psxn_with_layout() -> None:
    raw = parse(PSXN_SENTENCE)
    layout = PsxnLayout.from_str("rphx1")  # radians flag — raw values pass through
    p = decode_psxn(raw, layout)
    assert isinstance(p, Psxn)
    assert p.id == 23


def test_decode_prdid_unknown_dialect_produces_raw() -> None:
    # With dialect=UNKNOWN the decoder always emits a Raw variant.
    raw = parse(PRDID_SENTENCE)
    p = decode_prdid(raw, PrdidDialect.UNKNOWN)
    assert p.variant == "raw"
    assert isinstance(p.body, PrdidRaw)


def test_decode_prdid_pitch_roll_heading() -> None:
    raw = parse(PRDID_SENTENCE)
    p = decode_prdid(raw, PrdidDialect.PITCH_ROLL_HEADING)
    assert p.variant == "pitch_roll_heading"


def test_decode_with_custom_psxn_layout_round_trip() -> None:
    # decode_with should route PSXN through the options' PsxnLayout.
    opts = DecodeOptions().with_psxn_layout(PsxnLayout.from_str("rphx1"))
    raw = parse(PSXN_SENTENCE)
    msg = decode_with(raw, opts)
    assert isinstance(msg, Psxn)


# ---------- RMC ----------


def test_rmc_full_with_mode_decodes() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.talker == b"GP"
    assert m.status == DataStatus.ACTIVE
    assert m.mode == VtgMode.AUTONOMOUS
    assert m.utc is not None
    assert m.utc.hour == 12 and m.utc.minute == 35 and m.utc.second == 19
    assert m.latitude_deg is not None and abs(m.latitude_deg - 48.1173) < 1e-4
    assert m.longitude_deg is not None and abs(m.longitude_deg - 11.51667) < 1e-4
    assert m.speed_knots is not None and abs(m.speed_knots - 22.4) < 1e-2
    assert m.course_true_deg is not None and abs(m.course_true_deg - 84.4) < 1e-2
    assert m.date == UtcDate(day=23, month=3, year_yy=94)
    assert m.magnetic_variation_deg is not None
    assert abs(m.magnetic_variation_deg - (-3.1)) < 1e-2
    assert m.nav_status is None


def test_rmc_void_status_propagates() -> None:
    sentence = _with_checksum(b"GPRMC,,V,,,,,,,,,,N")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.status == DataStatus.VOID
    assert m.mode == VtgMode.NOT_VALID
    assert m.utc is None and m.latitude_deg is None


def test_rmc_eastern_variation_is_positive() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,E,A"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.magnetic_variation_deg is not None
    assert abs(m.magnetic_variation_deg - 5.0) < 1e-2


def test_rmc_with_nav_status() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,S"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.nav_status == RmcNavStatus.SAFE


def test_decode_rmc_extension_point_round_trip() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A"
    )
    raw = parse(sentence)
    m = decode_rmc(raw)
    assert isinstance(m, Rmc)
    assert m.status == DataStatus.ACTIVE


def test_data_status_enum_values() -> None:
    assert int(DataStatus.ACTIVE) == 0
    assert int(DataStatus.VOID) == 1
    assert DataStatus.ACTIVE != DataStatus.VOID


def test_rmc_nav_status_enum_values() -> None:
    assert int(RmcNavStatus.SAFE) == 0
    assert int(RmcNavStatus.CAUTION) == 1
    assert int(RmcNavStatus.UNSAFE) == 2
    assert int(RmcNavStatus.NOT_VALID) == 3


def test_utc_date_construct_and_read() -> None:
    d = UtcDate(23, 3, 94)
    assert d.day == 23 and d.month == 3 and d.year_yy == 94
    assert UtcDate(23, 3, 94) == UtcDate(23, 3, 94)


# ---------- GLL ----------


def test_gll_full_with_mode_decodes() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A,A")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Gll)
    assert m.talker == b"GP"
    assert m.status == DataStatus.ACTIVE
    assert m.mode == VtgMode.AUTONOMOUS
    assert m.latitude_deg is not None and abs(m.latitude_deg - 49.27417) < 1e-4
    assert m.longitude_deg is not None and abs(m.longitude_deg - (-123.18533)) < 1e-4
    assert m.utc is not None and m.utc.hour == 22 and m.utc.minute == 54


def test_gll_pre_2_3_form_has_no_mode() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Gll)
    assert m.mode is None


def test_gll_void_status_propagates() -> None:
    sentence = _with_checksum(b"GPGLL,,,,,,V,N")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Gll)
    assert m.status == DataStatus.VOID
    assert m.mode == VtgMode.NOT_VALID
    assert m.latitude_deg is None and m.longitude_deg is None


def test_decode_gll_extension_point_round_trip() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A,A")
    raw = parse(sentence)
    m = decode_gll(raw)
    assert isinstance(m, Gll)
    assert m.status == DataStatus.ACTIVE


def test_decode_routes_rmc_to_typed_variant() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A"
    )
    raw = parse(sentence)
    assert isinstance(decode(raw), Rmc)


def test_decode_routes_gll_to_typed_variant() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A,A")
    raw = parse(sentence)
    assert isinstance(decode(raw), Gll)
