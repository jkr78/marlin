"""Tests for async iterator helpers in marlin.aio."""

from __future__ import annotations

import asyncio
from typing import Any, List

import pytest

from .ais_vectors import AIVDM_TYPE1, AIVDM_TYPE5_FRAG1, AIVDM_TYPE5_FRAG2

GGA = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"


def test_aiter_sentences_yields_framed() -> None:
    from marlin.aio import aiter_sentences
    from marlin.envelope import RawSentence

    async def run() -> List[Any]:
        reader = asyncio.StreamReader()
        reader.feed_data(GGA)
        reader.feed_eof()
        out: List[Any] = []
        async for s in aiter_sentences(reader):
            out.append(s)
        return out

    sentences = asyncio.run(run())
    assert len(sentences) == 1
    assert isinstance(sentences[0], RawSentence)
    assert sentences[0].sentence_type == "GGA"


def test_aiter_sentences_empty_input() -> None:
    from marlin.aio import aiter_sentences

    async def run() -> List[Any]:
        reader = asyncio.StreamReader()
        reader.feed_data(b"")
        reader.feed_eof()
        out: List[Any] = []
        async for s in aiter_sentences(reader):
            out.append(s)
        return out

    sentences = asyncio.run(run())
    assert sentences == []


def test_aiter_nmea_messages_yields_gga() -> None:
    from marlin.aio import aiter_nmea_messages
    from marlin.nmea import Gga

    async def run() -> List[Any]:
        reader = asyncio.StreamReader()
        reader.feed_data(GGA)
        reader.feed_eof()
        out: List[Any] = []
        async for msg in aiter_nmea_messages(reader):
            out.append(msg)
        return out

    msgs = asyncio.run(run())
    assert len(msgs) == 1
    assert isinstance(msgs[0], Gga)
    assert msgs[0].latitude_deg.value is not None


def test_aiter_ais_messages_yields_type1() -> None:
    from marlin.aio import aiter_ais_messages
    from marlin.ais import AisMessage

    async def run() -> List[Any]:
        reader = asyncio.StreamReader()
        reader.feed_data(AIVDM_TYPE1)
        reader.feed_eof()
        out: List[Any] = []
        async for msg in aiter_ais_messages(reader):
            out.append(msg)
        return out

    msgs = asyncio.run(run())
    assert len(msgs) == 1
    assert isinstance(msgs[0], AisMessage)
    assert msgs[0].type_tag == "type1"


def test_aiter_ais_messages_multi_fragment() -> None:
    from marlin.aio import aiter_ais_messages
    from marlin.ais import AisMessage, StaticAndVoyageA

    async def run() -> List[Any]:
        # Feed both fragments together.
        reader = asyncio.StreamReader()
        reader.feed_data(AIVDM_TYPE5_FRAG1 + AIVDM_TYPE5_FRAG2)
        reader.feed_eof()
        out: List[Any] = []
        async for msg in aiter_ais_messages(reader):
            out.append(msg)
        return out

    msgs = asyncio.run(run())
    assert len(msgs) == 1
    assert isinstance(msgs[0], AisMessage)
    assert msgs[0].type_tag == "type5"
    assert isinstance(msgs[0].body, StaticAndVoyageA)


def test_aiter_sentences_accepts_custom_parser() -> None:
    """Custom parser passed via keyword argument is used instead of a new one."""
    from marlin.aio import aiter_sentences
    from marlin.envelope import StreamingParser

    custom = StreamingParser()

    async def run() -> List[Any]:
        reader = asyncio.StreamReader()
        reader.feed_data(GGA)
        reader.feed_eof()
        out: List[Any] = []
        async for s in aiter_sentences(reader, parser=custom):
            out.append(s)
        return out

    sentences = asyncio.run(run())
    assert len(sentences) == 1
