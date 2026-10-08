"""Unit tests for typed NMEA 0183 message variants."""

import pytest

from marlin.envelope import EnvelopeError, parse
from marlin.field import FieldState
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
        dgps_station_id=FieldState.Invalid(None),
    )
    assert g.talker == b"GP"
    assert g.utc == FieldState.Value(UtcTime(12, 35, 19, 0))
    assert g.fix_quality == FieldState.Value(GgaFixQuality.GPS_FIX)
    assert g.latitude_deg.value == pytest.approx(48.1173)
    assert g.dgps_age_s == FieldState.NotAvailable()
    assert g.dgps_station_id == FieldState.Invalid(None)


def test_gga_field_state_kwargs_default_to_not_available() -> None:
    g = Gga(talker=b"GP")
    assert g.utc == FieldState.NotAvailable()
    assert g.fix_quality == FieldState.NotAvailable()
    assert g.dgps_station_id == FieldState.NotAvailable()


# ---------- the write-side coercion rule, on Hdt.heading_true_deg ----------


def test_field_state_argument_passes_through() -> None:
    assert Hdt(b"IN", FieldState.Value(1.5)).heading_true_deg == FieldState.Value(1.5)
    assert Hdt(b"IN", FieldState.AtLeast(1.5)).heading_true_deg == FieldState.AtLeast(1.5)
    assert Hdt(b"IN", FieldState.NotAvailable()).heading_true_deg == FieldState.NotAvailable()
    assert Hdt(b"IN", FieldState.SenderError(-1)).heading_true_deg == FieldState.SenderError(-1)
    assert Hdt(b"IN", FieldState.Invalid(9)).heading_true_deg == FieldState.Invalid(9)
    assert Hdt(b"IN", FieldState.Invalid(None)).heading_true_deg == FieldState.Invalid(None)


def test_none_argument_is_not_available() -> None:
    assert Hdt(b"IN", None).heading_true_deg == FieldState.NotAvailable()


def test_bare_argument_is_a_value() -> None:
    assert Hdt(b"IN", 180.25).heading_true_deg == FieldState.Value(180.25)
    assert Hdt(b"IN", 180).heading_true_deg == FieldState.Value(180.0)


def test_wrong_payload_type_raises_type_error_as_is() -> None:
    with pytest.raises(TypeError, match="must be real number, not str"):
        Hdt(b"IN", "north")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="must be real number, not str"):
        Hdt(b"IN", FieldState.Value("north"))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="must be real number, not str"):
        Hdt(b"IN", FieldState.AtLeast("north"))  # type: ignore[arg-type]


def test_out_of_range_int_raises_overflow_error_as_is() -> None:
    with pytest.raises(OverflowError, match="out of range integral type conversion attempted"):
        Gga(talker=b"GP", satellites_used=300)
    with pytest.raises(OverflowError, match="out of range integral type conversion attempted"):
        Gga(talker=b"GP", satellites_used=FieldState.Value(-1))


def test_enum_payload_must_be_the_field_enum() -> None:
    with pytest.raises(TypeError, match="'VtgMode' object cannot be cast as 'GgaFixQuality'"):
        Gga(talker=b"GP", fix_quality=VtgMode.AUTONOMOUS)  # type: ignore[arg-type]


def test_message_repr_prints_the_field_state_form() -> None:
    g = Gga(talker=b"GP", fix_quality=GgaFixQuality.GPS_FIX, latitude_deg=48.5)
    assert repr(g) == (
        'Gga(talker=b"GP", fix_quality=FieldState.Value(GgaFixQuality.GPS_FIX),'
        " lat=FieldState.Value(48.5), lon=FieldState.NotAvailable())"
    )


def test_gga_fix_quality_int_compat() -> None:
    assert GgaFixQuality.NO_FIX == 0
    assert GgaFixQuality.GPS_FIX == 1
    assert int(GgaFixQuality.RTK_FIXED) == 4


def test_enum_classes_have_no_catch_all_member() -> None:
    # An unnamed letter is the invalid field state on the message, so
    # the enums carry no UNKNOWN / OTHER / INVALID member.
    for cls in (GgaFixQuality, VtgMode, DataStatus, RmcNavStatus):
        for name in ("UNKNOWN", "OTHER", "INVALID"):
            assert not hasattr(cls, name), f"{cls.__name__}.{name}"


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
    g = Gga(talker=b"GP", fix_quality=GgaFixQuality.NO_FIX)
    with pytest.raises(AttributeError, match="attribute 'talker' of 'marlin.nmea.Gga' objects is not writable"):
        g.talker = b"XX"  # type: ignore[misc]
    with pytest.raises(AttributeError, match="attribute 'hdop' of 'marlin.nmea.Gga' objects is not writable"):
        g.hdop = 1.0  # type: ignore[misc, assignment]


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
    assert v.mode == FieldState.Value(VtgMode.AUTONOMOUS)
    assert v.course_magnetic_deg == FieldState.NotAvailable()
    assert v.speed_knots.value == pytest.approx(5.5)


def test_hdt_constructs_and_reads() -> None:
    h = Hdt(talker=b"IN", heading_true_deg=180.25)
    assert h.talker == b"IN"
    assert isinstance(h.heading_true_deg, FieldState.Value)
    assert h.heading_true_deg.value == pytest.approx(180.25)


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
    assert p.id == FieldState.Value(23)
    assert p.token == FieldState.Value(b"abc")
    assert p.roll_deg.value == pytest.approx(1.5)
    assert p.pitch_deg.value == pytest.approx(-2.5)
    assert p.heave_m.value == pytest.approx(0.1)


def test_psxn_default_all_not_available() -> None:
    p = Psxn()
    assert p.id == FieldState.NotAvailable()
    assert p.token == FieldState.NotAvailable()
    assert p.roll_deg == FieldState.NotAvailable()


def test_psxn_token_rejects_str() -> None:
    with pytest.raises(TypeError, match="Can't extract `str` to `Vec`"):
        Psxn(token="abc")  # type: ignore[arg-type]


def test_prdid_raw_round_trip() -> None:
    p = Prdid.raw(fields=[b"1.2", b"3.4", b"5.6"])
    assert p.variant == "raw"
    assert isinstance(p.body, PrdidRaw)
    assert p.body.fields == (b"1.2", b"3.4", b"5.6")


def test_prdid_pitch_roll_heading_round_trip() -> None:
    p = Prdid.pitch_roll_heading(pitch_deg=1.0, roll_deg=2.0, heading_deg=180.0)
    assert p.variant == "pitch_roll_heading"
    assert isinstance(p.body, PrdidPitchRollHeading)
    assert p.body.pitch_deg.value == pytest.approx(1.0)
    assert p.body.heading_deg.value == pytest.approx(180.0)


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


def test_decode_vtg_wrong_type_decodes_with_invalid_fields() -> None:
    # A GGA sentence has enough fields for decode_vtg, so the mismatch
    # is not a decode error: the fields that are not VTG numbers are
    # invalid, and the sentence survives.
    raw = parse(GGA_SENTENCE)
    vtg = decode_vtg(raw)
    assert isinstance(vtg, Vtg)
    assert vtg.speed_knots == FieldState.Invalid(None)
    assert vtg.mode == FieldState.Invalid(None)


def test_decode_vtg_too_few_fields_raises() -> None:
    raw = parse(_with_checksum(b"GPVTG,054.7,T,034.4"))
    with pytest.raises(DecodeError, match="expected at least 8 fields, got 3"):
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
    assert p.id == FieldState.Value(23)
    assert p.token == FieldState.Value(b"1.5")


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
    assert m.status == FieldState.Value(DataStatus.ACTIVE)
    assert m.mode == FieldState.Value(VtgMode.AUTONOMOUS)
    assert m.utc == FieldState.Value(UtcTime(12, 35, 19, 0))
    assert m.latitude_deg.value == pytest.approx(48.1173, abs=1e-4)
    assert m.longitude_deg.value == pytest.approx(11.51667, abs=1e-4)
    assert m.speed_knots.value == pytest.approx(22.4, abs=1e-2)
    assert m.course_true_deg.value == pytest.approx(84.4, abs=1e-2)
    assert m.date == FieldState.Value(UtcDate(day=23, month=3, year_yy=94))
    assert m.magnetic_variation_deg.value == pytest.approx(-3.1, abs=1e-2)
    assert m.nav_status == FieldState.NotAvailable()


def test_rmc_void_status_propagates() -> None:
    sentence = _with_checksum(b"GPRMC,,V,,,,,,,,,,N")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.status == FieldState.Value(DataStatus.VOID)
    assert m.mode == FieldState.Value(VtgMode.NOT_VALID)
    assert m.utc == FieldState.NotAvailable()
    assert m.latitude_deg == FieldState.NotAvailable()


def test_rmc_one_bad_field_is_invalid_and_the_rest_decodes() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,,A"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.magnetic_variation_deg == FieldState.Invalid(None)
    assert m.speed_knots.value == pytest.approx(22.4, abs=1e-2)


def test_rmc_empty_status_is_not_available_and_unnamed_byte_is_invalid() -> None:
    p = Nmea0183Parser.streaming()
    p.feed(_with_checksum(b"GPRMC,,,,,,,,,,,,"))
    p.feed(_with_checksum(b"GPRMC,,X,,,,,,,,,,"))
    empty, unnamed = list(p)
    assert isinstance(empty, Rmc) and isinstance(unnamed, Rmc)
    assert empty.status == FieldState.NotAvailable()
    assert unnamed.status == FieldState.Invalid(ord("X"))


def test_gga_undefined_fix_quality_digit_is_invalid_with_the_digit() -> None:
    raw = parse(_with_checksum(b"GPGGA,123519,4807.038,N,01131.000,E,9,08,0.9,545.4,M,46.9,M,,"))
    gga = decode_gga(raw)
    assert gga.fix_quality == FieldState.Invalid(9)


def test_gga_half_filled_latitude_pair_is_invalid() -> None:
    raw = parse(_with_checksum(b"GPGGA,123519,4807.038,,01131.000,E,1,08,0.9,545.4,M,46.9,M,,"))
    gga = decode_gga(raw)
    assert gga.latitude_deg == FieldState.Invalid(None)
    assert gga.longitude_deg.value == pytest.approx(11.51667, abs=1e-4)


def test_decoded_field_supports_match() -> None:
    raw = parse(GGA_SENTENCE)
    gga = decode_gga(raw)
    match gga.satellites_used:
        case FieldState.Value(n):
            assert n == 8
        case _:
            raise AssertionError("expected a value")


def test_rmc_eastern_variation_is_positive() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,005.0,E,A"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.magnetic_variation_deg.value == pytest.approx(5.0, abs=1e-2)


def test_rmc_with_nav_status() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A,S"
    )
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Rmc)
    assert m.nav_status == FieldState.Value(RmcNavStatus.SAFE)


def test_decode_rmc_extension_point_round_trip() -> None:
    sentence = _with_checksum(
        b"GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W,A"
    )
    raw = parse(sentence)
    m = decode_rmc(raw)
    assert isinstance(m, Rmc)
    assert m.status == FieldState.Value(DataStatus.ACTIVE)


def test_data_status_enum_values() -> None:
    assert int(DataStatus.ACTIVE) == 0
    assert int(DataStatus.VOID) == 1
    assert DataStatus.ACTIVE != DataStatus.VOID


def test_rmc_nav_status_enum_values() -> None:
    assert int(RmcNavStatus.SAFE) == 0
    assert int(RmcNavStatus.CAUTION) == 1
    assert int(RmcNavStatus.UNSAFE) == 2
    assert int(RmcNavStatus.NOT_VALID) == 3


def test_nmea_enums_are_hashable() -> None:
    # The stubs declare __hash__ on every int-backed enum; pin it at
    # runtime so the enums work as set members and dict keys.
    members = {
        DataStatus.VOID,
        GgaFixQuality.DGPS_FIX,
        PrdidDialect.ROLL_PITCH_HEADING,
        PsxnSlot.ROLL,
        RmcNavStatus.CAUTION,
        VtgMode.AUTONOMOUS,
    }
    assert len(members) == 6
    assert DataStatus.VOID in members
    assert hash(VtgMode.AUTONOMOUS) == hash(VtgMode.AUTONOMOUS)


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
    assert m.status == FieldState.Value(DataStatus.ACTIVE)
    assert m.mode == FieldState.Value(VtgMode.AUTONOMOUS)
    assert m.latitude_deg.value == pytest.approx(49.27417, abs=1e-4)
    assert m.longitude_deg.value == pytest.approx(-123.18533, abs=1e-4)
    assert m.utc == FieldState.Value(UtcTime(22, 54, 44, 0))


def test_gll_pre_2_3_form_has_no_mode() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Gll)
    assert m.mode == FieldState.NotAvailable()


def test_gll_void_status_propagates() -> None:
    sentence = _with_checksum(b"GPGLL,,,,,,V,N")
    p = Nmea0183Parser.streaming()
    p.feed(sentence)
    m = p.next_message()
    assert isinstance(m, Gll)
    assert m.status == FieldState.Value(DataStatus.VOID)
    assert m.mode == FieldState.Value(VtgMode.NOT_VALID)
    assert m.latitude_deg == FieldState.NotAvailable()
    assert m.longitude_deg == FieldState.NotAvailable()


def test_decode_gll_extension_point_round_trip() -> None:
    sentence = _with_checksum(b"GPGLL,4916.45,N,12311.12,W,225444,A,A")
    raw = parse(sentence)
    m = decode_gll(raw)
    assert isinstance(m, Gll)
    assert m.status == FieldState.Value(DataStatus.ACTIVE)


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
