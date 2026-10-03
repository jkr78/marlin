"""Tests for frozen dataclass mirrors in marlin.dataclasses."""

from __future__ import annotations

import dataclasses
import json

import pytest

from .ais_vectors import (
    AIVDM_TYPE1,
    AIVDM_TYPE1_ROT_PLUS_127,
    AIVDM_TYPE5_FRAG1,
    AIVDM_TYPE5_FRAG2,
    AIVDM_TYPE9_GPSD_T9_2,
    AIVDM_TYPE21_GPSD_T21_1_FRAG1,
    AIVDM_TYPE21_GPSD_T21_1_FRAG2,
    AIVDM_TYPE21_GPSD_T21_2_FRAG1,
    AIVDM_TYPE21_GPSD_T21_2_FRAG2,
    AIVDM_TYPE24B_AUXILIARY_CRAFT,
)

GGA = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"


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


def _frame(body: bytes) -> bytes:
    """Prefix `$` and append `*<checksum>` (XOR of body bytes)."""
    checksum = 0
    for b in body:
        checksum ^= b
    return b"$" + body + b"*" + f"{checksum:02X}".encode("ascii")


def test_hdg_round_trip() -> None:
    from marlin.dataclasses import Hdg as DCHdg
    from marlin.dataclasses import to_dataclass
    from marlin.nmea import Nmea0183Parser

    p = Nmea0183Parser.streaming()
    p.feed(_frame(b"HCHDG,98.3,0.0,E,12.6,W") + b"\r\n")
    hdg = p.next_message()

    dc = to_dataclass(hdg)
    assert isinstance(dc, DCHdg)

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)

    assert dc.talker == b"HC"
    assert d["heading_magnetic_deg"] is not None
    assert d["variation_deg"] is not None


def test_ttm_round_trip() -> None:
    from marlin.dataclasses import Ttm as DCTtm
    from marlin.dataclasses import to_dataclass
    from marlin.nmea import Nmea0183Parser

    raw = b"RATTM,12,1.23,45.6,T,7.8,90.1,R,2.5,-11.0,S,TGT1,T,R,123519.00,R"
    p = Nmea0183Parser.streaming()
    p.feed(_frame(raw) + b"\r\n")
    ttm = p.next_message()

    dc = to_dataclass(ttm)
    assert isinstance(dc, DCTtm)

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)

    assert dc.talker == b"RA"
    assert dc.target_number == 12
    assert dc.name == "TGT1"
    assert dc.reference_target is True
    assert isinstance(d["bearing_reference"], int)
    assert isinstance(d["course_reference"], int)
    assert isinstance(d["units"], int)
    assert isinstance(d["status"], int)
    assert isinstance(d["acquisition"], int)
    assert d["utc_time"] is not None


def test_tll_round_trip() -> None:
    from marlin.dataclasses import Tll as DCTll
    from marlin.dataclasses import to_dataclass
    from marlin.nmea import Nmea0183Parser

    raw = b"RATLL,7,4807.038,N,01131.000,E,TGT7,123519,T,R"
    p = Nmea0183Parser.streaming()
    p.feed(_frame(raw) + b"\r\n")
    tll = p.next_message()

    dc = to_dataclass(tll)
    assert isinstance(dc, DCTll)

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)

    assert dc.target_number == 7
    assert dc.name == "TGT7"
    assert dc.reference_target is True
    assert isinstance(d["status"], int)


def test_unknown_round_trip() -> None:
    from marlin.dataclasses import Unknown as DCUnknown
    from marlin.dataclasses import to_dataclass
    from marlin.envelope import StreamingParser
    from marlin.nmea import decode

    # GSV is still un-typed (PRD §11 deferred); RMC and GLL would route
    # to their typed variants now.
    gsv_bytes = b"$GPGSV,3,1,11,18,87,050,48,22,56,250,49,21,55,122,49,03,40,284,47*78\r\n"
    p = StreamingParser()
    p.feed(gsv_bytes)
    sentence = next(iter(p))
    msg = decode(sentence)

    dc = to_dataclass(msg)
    assert isinstance(dc, DCUnknown)
    assert dc.sentence_type == "GSV"


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
    # Classic fixture carries ROT -128: both flattened fields are None.
    assert d["body"]["rate_of_turn"] is None
    assert d["body"]["turn_direction"] is None


def test_ais_position_report_a_turn_direction_is_enum_int() -> None:
    from marlin.ais import AisParser, TurnDirection
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import PositionReportA as DCPositionReportA
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE1_ROT_PLUS_127)
    msgs = list(p)
    assert len(msgs) == 1

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCPositionReportA)
    assert dc.body.rate_of_turn is None
    # The mirror stores the TurnDirection enum value, not the wire code 127.
    assert dc.body.turn_direction == int(TurnDirection.RIGHT)


def test_ais_static_and_voyage_a_round_trip() -> None:
    from marlin.ais import AisParser, StaticAndVoyageA
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import StaticAndVoyageA as DCStaticAndVoyageA
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE5_FRAG1 + AIVDM_TYPE5_FRAG2)
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


def test_ais_sar_aircraft_position_report_round_trip() -> None:
    from marlin.ais import AisParser, AltitudeSensor, SarAircraftPositionReport
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import (
        SarAircraftPositionReport as DCSarAircraftPositionReport,
    )
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE9_GPSD_T9_2)
    msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0].body, SarAircraftPositionReport)

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCSarAircraftPositionReport)
    assert dc.type_tag == "type9"
    assert dc.body.mmsi == 111232511
    assert dc.body.altitude_m == 303
    assert dc.body.speed_over_ground == 42
    # The mirror stores the AltitudeSensor wire value as int.
    assert dc.body.altitude_sensor == int(AltitudeSensor.GNSS)
    assert dc.body.dte is True

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["body"]["radio_status"] == 0x8270


def test_ais_aid_to_navigation_report_round_trip() -> None:
    from marlin.ais import AidToNavigationReport, AisParser, AtonType, EpfdType
    from marlin.dataclasses import (
        AidToNavigationReport as DCAidToNavigationReport,
    )
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE21_GPSD_T21_2_FRAG1 + AIVDM_TYPE21_GPSD_T21_2_FRAG2)
    msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0].body, AidToNavigationReport)

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCAidToNavigationReport)
    assert dc.type_tag == "type21"
    assert dc.body.mmsi == 4000003
    # The mirror stores the AtonType and EpfdType wire values as int.
    assert dc.body.aton_type == int(AtonType.SPECIAL_MARK) == 30
    assert dc.body.epfd == int(EpfdType.GPS)
    # Trailing-trim policy: the embedded @ survives (gpsd would stop there).
    assert dc.body.name == "IBC G BUOY@?????????"
    assert dc.body.dimensions.to_bow_m == 2

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["body"]["aton_status"] == 0
    assert d["body"]["dimensions"]["to_port_m"] == 2


def test_ais_static_data_b24b_mothership_round_trip() -> None:
    from marlin.ais import AisParser, EpfdType, StaticDataB24B
    from marlin.dataclasses import AisMessage as DCAisMessage
    from marlin.dataclasses import StaticDataB24B as DCStaticDataB24B
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE24B_AUXILIARY_CRAFT)
    msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0].body, StaticDataB24B)

    dc = to_dataclass(msgs[0])
    assert isinstance(dc, DCAisMessage)
    assert isinstance(dc.body, DCStaticDataB24B)
    assert dc.type_tag == "type24b"
    # The mirror keeps the flattened extent: no all-None Dimensions stand-in.
    assert dc.body.dimensions is None
    assert dc.body.mothership_mmsi == 211000123
    assert dc.body.epfd == int(EpfdType.GPS)

    d = dataclasses.asdict(dc)
    json.dumps(d, default=_json_default)
    assert d["body"]["dimensions"] is None
    assert d["body"]["mothership_mmsi"] == 211000123


def test_to_dataclass_accepts_bare_ais_body() -> None:
    # A body passed without its AisMessage wrapper converts through the
    # same dispatcher as the wrapped path.
    from marlin.ais import AisParser
    from marlin.dataclasses import (
        AidToNavigationReport as DCAidToNavigationReport,
    )
    from marlin.dataclasses import to_dataclass

    p = AisParser.streaming()
    p.feed(AIVDM_TYPE21_GPSD_T21_1_FRAG1 + AIVDM_TYPE21_GPSD_T21_1_FRAG2)
    body = list(p)[0].body
    dc = to_dataclass(body)
    assert isinstance(dc, DCAidToNavigationReport)
    assert dc.name == "CHINA ROSE MURPHY EXPRESS ALERT"


def test_to_dataclass_type_error_on_unknown() -> None:
    """Passing an unrecognised object raises TypeError."""
    from marlin.dataclasses import to_dataclass

    with pytest.raises(TypeError, match="unrecognised marlin message type"):
        to_dataclass(object())


def test_to_dataclass_type_error_on_plain_string() -> None:
    from marlin.dataclasses import to_dataclass

    with pytest.raises(TypeError):
        to_dataclass("not a marlin message")


def test_to_dataclass_type_error_on_enum() -> None:
    from marlin.ais import NavStatus
    from marlin.dataclasses import to_dataclass

    with pytest.raises(TypeError, match="unrecognised marlin message type"):
        to_dataclass(NavStatus.AT_ANCHOR)


def test_to_dataclass_type_error_on_a_dataclass_mirror() -> None:
    from marlin.dataclasses import Hdt, to_dataclass

    already_converted = Hdt(talker=b"IN", heading_true_deg=180.25)

    with pytest.raises(TypeError, match="unrecognised marlin message type"):
        to_dataclass(already_converted)


def test_to_dataclass_converts_a_value_type_on_its_own() -> None:
    import marlin.ais
    import marlin.dataclasses
    from marlin.dataclasses import to_dataclass

    dimensions = marlin.ais.Dimensions(
        to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4
    )

    assert to_dataclass(dimensions) == marlin.dataclasses.Dimensions(
        to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4
    )


def test_hand_built_message_with_no_dimensions_converts_to_all_none() -> None:
    import marlin.ais
    import marlin.dataclasses
    from marlin.dataclasses import to_dataclass

    dc = to_dataclass(marlin.ais.StaticAndVoyageA(mmsi=123456789))

    assert isinstance(dc, marlin.dataclasses.StaticAndVoyageA)
    assert dc.dimensions == marlin.dataclasses.Dimensions(
        to_bow_m=None, to_stern_m=None, to_port_m=None, to_starboard_m=None
    )
    assert dc.eta == marlin.dataclasses.Eta(
        month=None, day=None, hour=None, minute=None
    )


def test_binding_class_rejects_a_wrong_typed_nested_value() -> None:
    # `to_dataclass` needs no fallback for a wrong-typed field, because no
    # such message can be built.
    import marlin.ais

    with pytest.raises(TypeError):
        marlin.ais.StaticAndVoyageA(dimensions="not dimensions")  # type: ignore[arg-type]
