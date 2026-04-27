"""Tests for context manager (__enter__ / __exit__) support on all parsers."""

from __future__ import annotations

GGA = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
AIVDM = b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n"


def test_context_manager_oneshot_parser() -> None:
    from marlin.envelope import OneShotParser

    with OneShotParser() as p:
        p.feed(GGA)
        s = p.next_sentence()
    assert s is not None
    assert s.sentence_type == "GGA"


def test_context_manager_streaming_parser() -> None:
    from marlin.envelope import StreamingParser

    with StreamingParser() as p:
        p.feed(GGA)
        sentences = list(p)
    assert len(sentences) == 1
    assert sentences[0].sentence_type == "GGA"


def test_context_manager_nmea_parser() -> None:
    from marlin.nmea import Nmea0183Parser

    with Nmea0183Parser.streaming() as p:
        p.feed(GGA)
        msgs = list(p)
    assert len(msgs) == 1
    from marlin.nmea import Gga

    assert isinstance(msgs[0], Gga)


def test_context_manager_ais_parser() -> None:
    from marlin.ais import AisMessage, AisParser

    with AisParser.streaming() as p:
        p.feed(AIVDM)
        msgs = list(p)
    assert len(msgs) == 1
    assert isinstance(msgs[0], AisMessage)
    assert msgs[0].type_tag == "type1"


def test_context_manager_does_not_suppress_exceptions() -> None:
    """__exit__ must return False so exceptions propagate normally."""
    from marlin.envelope import StreamingParser

    import pytest

    with pytest.raises(RuntimeError, match="boom"):
        with StreamingParser() as p:
            p.feed(GGA)
            raise RuntimeError("boom")


def test_context_manager_oneshot_is_same_object() -> None:
    """__enter__ must return the parser itself (identity)."""
    from marlin.envelope import OneShotParser

    outer = OneShotParser()
    with outer as inner:
        assert outer is inner


def test_context_manager_streaming_is_same_object() -> None:
    from marlin.envelope import StreamingParser

    outer = StreamingParser()
    with outer as inner:
        assert outer is inner
