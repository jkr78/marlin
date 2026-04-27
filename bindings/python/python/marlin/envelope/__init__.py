"""Raw NMEA sentence parsing (any shape, typed or untyped)."""

from .. import _core

EnvelopeError: type[Exception] = _core.EnvelopeError
RawSentence = _core.envelope.RawSentence
OneShotParser = _core.envelope.OneShotParser
StreamingParser = _core.envelope.StreamingParser
parse = _core.envelope.parse

__all__ = [
    "EnvelopeError",
    "RawSentence",
    "OneShotParser",
    "StreamingParser",
    "parse",
]
