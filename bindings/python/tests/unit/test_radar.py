"""Unit tests for HDG / TTM / TLL (radar + heading sentences)."""

from __future__ import annotations

import pytest

import marlin.nmea as nmea
from marlin.field import FieldState
from marlin.nmea import (
    AcquisitionType,
    AngleReference,
    DistanceUnits,
    Nmea0183Parser,
    TargetStatus,
)


def _frame(body: bytes) -> bytes:
    """Prefix `$` and append `*<checksum>` (XOR of body bytes)."""
    checksum = 0
    for b in body:
        checksum ^= b
    return b"$" + body + b"*" + f"{checksum:02X}".encode("ascii")


def _decode_one(sentence: bytes):
    p = Nmea0183Parser.streaming()
    p.feed(sentence + b"\r\n")
    return p.next_message()


def test_hdg_signed_corrections() -> None:
    msg = _decode_one(b"$HCHDG,98.3,0.0,E,12.6,W*57")
    assert isinstance(msg, nmea.Hdg)
    assert msg.talker == b"HC"
    assert msg.heading_magnetic_deg.value == pytest.approx(98.3, abs=0.01)
    assert msg.variation_deg.value == pytest.approx(-12.6, abs=0.01)  # W is negative


def test_hdg_half_filled_pair_is_invalid_without_failing_the_sentence() -> None:
    msg = _decode_one(_frame(b"HCHDG,98.3,1.0,,7.1,E"))
    assert isinstance(msg, nmea.Hdg)
    assert msg.deviation_deg == FieldState.Invalid(None)
    assert msg.variation_deg.value == pytest.approx(7.1, abs=0.01)


def test_ttm_full_rattm() -> None:
    # 15-field RATTM: statute units, tracking, reference, reported acquisition.
    raw = b"RATTM,12,1.23,45.6,T,7.8,90.1,R,2.5,-11.0,S,TGT1,T,R,123519.00,R"
    msg = _decode_one(_frame(raw))
    assert isinstance(msg, nmea.Ttm)
    assert msg.talker == b"RA"
    assert msg.target_number == FieldState.Value(12)
    assert msg.units == FieldState.Value(DistanceUnits.STATUTE)
    assert msg.status == FieldState.Value(TargetStatus.TRACKING)
    assert msg.bearing_reference == FieldState.Value(AngleReference.TRUE)
    assert msg.course_reference == FieldState.Value(AngleReference.RELATIVE)
    assert msg.acquisition == FieldState.Value(AcquisitionType.REPORTED)
    assert msg.reference_target == FieldState.Value(True)
    assert msg.name == FieldState.Value("TGT1")
    assert msg.tcpa.value == pytest.approx(-11.0, abs=0.001)
    assert msg.utc_time == FieldState.Value(nmea.UtcTime(12, 35, 19, 0))


def test_ttm_base_13_fields_trailing_not_available_and_reference_false() -> None:
    msg = _decode_one(_frame(b"RATTM,3,5.0,180.0,T,10.0,270.0,T,1.0,5.0,N,,Q,"))
    assert isinstance(msg, nmea.Ttm)
    assert msg.name == FieldState.NotAvailable()
    assert msg.reference_target == FieldState.Value(False)
    assert msg.utc_time == FieldState.NotAvailable()
    assert msg.acquisition == FieldState.NotAvailable()


def test_tll_position_and_status() -> None:
    msg = _decode_one(_frame(b"RATLL,7,4807.038,N,01131.000,E,TGT7,123519,T,R"))
    assert isinstance(msg, nmea.Tll)
    assert msg.target_number == FieldState.Value(7)
    assert msg.latitude_deg.value == pytest.approx(48.1173, abs=0.0001)
    assert msg.status == FieldState.Value(TargetStatus.TRACKING)
    assert msg.reference_target == FieldState.Value(True)


def test_tll_position_only_trailing_fields_not_available() -> None:
    msg = _decode_one(_frame(b"RATLL,2,5000.00,N,00500.00,E"))
    assert isinstance(msg, nmea.Tll)
    assert msg.name == FieldState.NotAvailable()
    assert msg.status == FieldState.NotAvailable()
    assert msg.reference_target == FieldState.Value(False)


def test_ttm_unnamed_code_is_invalid_with_the_byte() -> None:
    msg = _decode_one(_frame(b"RATTM,1,1.0,2.0,X,3.0,4.0,T,5.0,6.0,N,n,T,Q"))
    assert isinstance(msg, nmea.Ttm)
    assert msg.bearing_reference == FieldState.Invalid(ord("X"))
    assert msg.reference_target == FieldState.Invalid(ord("Q"))


def test_radar_enums_have_no_unknown_member() -> None:
    for cls in (AcquisitionType, AngleReference, DistanceUnits, TargetStatus):
        assert not hasattr(cls, "UNKNOWN"), cls.__name__
