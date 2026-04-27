"""Async iterator helpers for marlin parsers.

Wraps the synchronous parsers in `async for` loops driven by an
`asyncio.StreamReader`. The sync `feed()` calls cost microseconds per
chunk, so no thread offload is needed — see GUIDE.md §2.
"""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING, AsyncIterator, Optional

from marlin.ais import AisMessage, AisParser
from marlin.envelope import RawSentence, StreamingParser
from marlin.nmea import Nmea0183Parser

if TYPE_CHECKING:
    from marlin.nmea import Nmea0183Message

__all__ = ["aiter_sentences", "aiter_nmea_messages", "aiter_ais_messages"]


async def aiter_sentences(
    reader: asyncio.StreamReader,
    *,
    parser: Optional[StreamingParser] = None,
    chunk_size: int = 4096,
) -> AsyncIterator[RawSentence]:
    """Yield framed envelope sentences from `reader` until EOF."""
    p = parser if parser is not None else StreamingParser()
    while chunk := await reader.read(chunk_size):
        p.feed(chunk)
        for sentence in p:
            yield sentence


async def aiter_nmea_messages(
    reader: asyncio.StreamReader,
    *,
    parser: Optional[Nmea0183Parser] = None,
    chunk_size: int = 4096,
) -> AsyncIterator["Nmea0183Message"]:
    """Yield typed NMEA messages from `reader` until EOF."""
    p = parser if parser is not None else Nmea0183Parser.streaming()
    while chunk := await reader.read(chunk_size):
        p.feed(chunk)
        for msg in p:
            yield msg


async def aiter_ais_messages(
    reader: asyncio.StreamReader,
    *,
    parser: Optional[AisParser] = None,
    chunk_size: int = 4096,
) -> AsyncIterator[AisMessage]:
    """Yield decoded AIS messages from `reader` until EOF."""
    p = parser if parser is not None else AisParser.streaming()
    while chunk := await reader.read(chunk_size):
        p.feed(chunk)
        for msg in p:
            yield msg
