"""marlin — NMEA 0183 + AIS parser (Rust-backed).

Submodules:
    marlin.envelope — raw NMEA sentence parsing (any shape).
    marlin.nmea     — typed NMEA 0183 decoders (GGA, VTG, HDT, PSXN, PRDID).
    marlin.ais      — typed AIS decoders + multi-sentence reassembly.

Most users can start with:

    from marlin.envelope import StreamingParser
    p = StreamingParser()
    p.feed(b"...")
    for sentence in p:
        ...
"""

from . import _core

__version__: str = _core.__version__

# Base exception re-exported at the top level.
MarlinError: type[Exception] = _core.MarlinError

__all__ = ["__version__", "MarlinError"]
