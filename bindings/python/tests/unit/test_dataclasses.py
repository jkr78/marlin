"""Tests for frozen dataclass mirrors in marlin.dataclasses."""

from __future__ import annotations

import dataclasses
import json

import pytest

GGA = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
AIVDM_TYPE1 = b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n"

# Type 5 two-fragment message (same corpus as test_aio.py).
_TYPE5_FRAG1 = (
    b"!AIVDM,2,1,3,A,"
    b"55P5TL01VIaAL@7WKO@mBplU@<PDhh000000001S;AJ::4A80?4i@E53,"
    b"0*3D\r\n"
)
_TYPE5_FRAG2 = b"!AIVDM,2,2,3,A,1CQWBDhH888888888880,2*4D\r\n"


def _json_default(v: object) -> object:
    """Allow bytes in JSON output by converting to a hex string."""
    if isinstance(v, bytes):
        return v.hex()
    return str(v)


# ---------- envelope round-trip ----------


def test_raw_sentence_round_trip() -> None:
    from marlin.dataclasses import RawSentence as DCRawSentence
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser

    p = StreamingParser()
    p.feed(GGA)
    sentence = next(iter(p))

    dc = to_dataclass(sentence)
    assert isinstance(dc, DCRawSentence)

    d = dataclasses.asdict(dc)
    # JSON-serializable with bytes → hex adapter.
    json.dumps(d, default=_json_default)

    assert d["sentence_type"] == "GGA"
    assert d["checksum_ok"] is True
    assert isinstance(d["fields"], tuple)  # tuples are preserved by asdict


# ---------- NMEA round-trips ----------


def test_gga_round_trip() -> None:
    from marlin.dataclasses import Gga as DCGga
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import decode_gga

    p = StreamingParser()
    p.feed(GGA)
    sentence = next(iter(p))
    gga = decode_gga(sentence)

    dc = to_dataclass(gga)
    assert isinstance(dc, DCGga)

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)

    assert d["latitude_deg"] is not None
    assert d["longitude_deg"] is not None
    assert d["satellites_used"] == 8
    assert isinstance(d["fix_quality"], int)


def test_vtg_round_trip() -> None:
    from marlin.dataclasses import Vtg as DCVtg
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import decode_vtg

    vtg_bytes = b"$GPVTG,054.7,T,034.4,M,005.5,N,010.2,K,A*25\r\n"
    p = StreamingParser()
    p.feed(vtg_bytes)
    sentence = next(iter(p))
    vtg = decode_vtg(sentence)

    dc = to_dataclass(vtg)
    assert isinstance(dc, DCVtg)
    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["speed_knots"] is not None


def test_hdt_round_trip() -> None:
    from marlin.dataclasses import Hdt as DCHdt
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import decode_hdt

    hdt_bytes = b"$HEHDT,045.0,T*2E\r\n"
    p = StreamingParser()
    p.feed(hdt_bytes)
    sentence = next(iter(p))
    hdt = decode_hdt(sentence)

    dc = to_dataclass(hdt)
    assert isinstance(dc, DCHdt)
    d = dataclasses.asdict(dc)
    assert d["heading_true_deg"] is not None


def test_unknown_round_trip() -> None:
    from marlin.dataclasses import Unknown as DCUnknown
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import decode

    rmc_bytes = b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A\r\n"
    p = StreamingParser()
    p.feed(rmc_bytes)
    sentence = next(iter(p))
    msg = decode(sentence)

    dc = to_dataclass(msg)
    assert isinstance(dc, DCUnknown)
    assert dc.sentence_type == "RMC"


def test_prdid_raw_round_trip() -> None:
    from marlin.dataclasses import Prdid as DCPrdid
    from marlin.dataclasses import PrdidRaw as DCPrdidRaw
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import PrdidDialect, decode_prdid

    prdid_bytes = b"$PRDID,+01.0,-00.5,180.0*42\r\n"
    p = StreamingParser()
    p.feed(prdid_bytes)
    sentence = next(iter(p))
    # Default dialect (UNKNOWN) emits Raw.
    prdid = decode_prdid(sentence, PrdidDialect.UNKNOWN)

    dc = to_dataclass(prdid)
    assert isinstance(dc, DCPrdid)
    assert dc.variant == "raw"
    assert isinstance(dc.body, DCPrdidRaw)


# ---------- AIS round-trips ----------


def test_ais_position_report_a_round_trip() -> None:
    from marlin.ais import AisParser, PositionReportA
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import PositionReportA as DCPositionReportA
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE1)
    msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0].body, PositionReportA)

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCPositionReportA)
    assert dc.type_tag == "type1"

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["body"]["mmsi"] > 0
    assert isinstance(d["body"]["navigation_status"], int)


def test_ais_static_and_voyage_a_round_trip() -> None:
    from marlin.ais import AisParser, StaticAndVoyageA
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import StaticAndVoyageA as DCStaticAndVoyageA
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(_TYPE5_FRAG1 + _TYPE5_FRAG2)
    msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0].body, StaticAndVoyageA)

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCStaticAndVoyageA)
    assert dc.type_tag == "type5"

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["body"]["mmsi"] > 0
    # eta and dimensions are always-present nested dataclasses.
    assert "eta" in d["body"]
    assert "dimensions" in d["body"]


def test_to_dataclass_type_error_on_unknown() -> None:
    """Passing an unrecognised object raises TypeError."""
    from marlin.dataclasses import to_dataclass

    with pytest.raises(TypeError, match="unrecognised marlin message type"):
        to_dataclass(object())


def test_to_dataclass_type_error_on_plain_string() -> None:
    from marlin.dataclasses import to_dataclass

    with pytest.raises(TypeError):
        to_dataclass("not a marlin message")
