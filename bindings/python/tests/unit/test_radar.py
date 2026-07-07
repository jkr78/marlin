"""Unit tests for HDG / TTM / TLL (radar + heading sentences)."""

from __future__ import annotations

import marlin.nmea as nmea
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
    assert msg.heading_magnetic_deg is not None and abs(msg.heading_magnetic_deg - 98.3) < 0.01
    assert msg.variation_deg is not None and abs(msg.variation_deg - -12.6) < 0.01  # W is negative


def test_ttm_full_rattm() -> None:
    # 15-field RATTM: statute units, tracking, reference, reported acquisition.
    raw = b"RATTM,12,1.23,45.6,T,7.8,90.1,R,2.5,-11.0,S,TGT1,T,R,123519.00,R"
    msg = _decode_one(_frame(raw))
    assert isinstance(msg, nmea.Ttm)
    assert msg.talker == b"RA"
    assert msg.target_number == 12
    assert msg.units == DistanceUnits.STATUTE
    assert msg.status == TargetStatus.TRACKING
    assert msg.bearing_reference == AngleReference.TRUE
    assert msg.course_reference == AngleReference.RELATIVE
    assert msg.acquisition == AcquisitionType.REPORTED
    assert msg.reference_target is True
    assert msg.name == "TGT1"
    assert msg.tcpa is not None and abs(msg.tcpa - -11.0) < 0.001


def test_tll_position_and_status() -> None:
    msg = _decode_one(_frame(b"RATLL,7,4807.038,N,01131.000,E,TGT7,123519,T,R"))
    assert isinstance(msg, nmea.Tll)
    assert msg.target_number == 7
    assert msg.latitude_deg is not None and abs(msg.latitude_deg - 48.1173) < 0.0001
    assert msg.status == TargetStatus.TRACKING
    assert msg.reference_target is True


def test_ttm_unknown_code_collapses_to_unknown() -> None:
    msg = _decode_one(_frame(b"RATTM,1,1.0,2.0,X,3.0,4.0,T,5.0,6.0,N,n,T,"))
    assert msg.bearing_reference == AngleReference.UNKNOWN
