import pytest
from marlin.envelope import (
    OneShotParser, StreamingParser, RawSentence, EnvelopeError, parse,
)


GGA = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
GGA_NO_CRLF = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47"
PSXN = b"$PSXN,20,1,1,1,1*3B\r\n"   # proprietary — talker is None


def _xor(b: bytes) -> int:
    acc = 0
    for x in b:
        acc ^= x
    return acc


def _tag_sentence(tag: bytes, body: bytes, terminator: bytes = b"\r\n") -> bytes:
    # Mirrors marlin-nmea-envelope's testing::build_with_tag.
    return (
        b"\\" + tag + b"*%02X\\" % _xor(tag)
        + b"$" + body + b"*%02X" % _xor(body)
        + terminator
    )


def test_oneshot_single_sentence():
    p = OneShotParser()
    p.feed(GGA)
    s = p.next_sentence()
    assert isinstance(s, RawSentence)
    assert s.talker == b"GP"
    assert s.sentence_type == "GGA"
    assert s.checksum_ok
    assert len(s.fields) == 14


def test_oneshot_no_terminator():
    p = OneShotParser()
    p.feed(GGA_NO_CRLF)
    s = p.next_sentence()
    assert s is not None
    assert s.sentence_type == "GGA"


def test_streaming_two_sentences_one_feed():
    p = StreamingParser()
    p.feed(GGA + GGA)
    sentences = list(p)
    assert len(sentences) == 2
    assert all(s.sentence_type == "GGA" for s in sentences)


def test_streaming_split_sentence():
    p = StreamingParser()
    p.feed(GGA[:20])
    assert p.next_sentence() is None
    p.feed(GGA[20:])
    s = p.next_sentence()
    assert s is not None
    assert s.sentence_type == "GGA"


def test_proprietary_talker_none():
    p = OneShotParser()
    p.feed(PSXN)
    s = p.next_sentence()
    assert s.talker is None
    assert s.sentence_type == "PSXN"


def test_rawsentence_frozen():
    p = OneShotParser()
    p.feed(GGA)
    s = p.next_sentence()
    with pytest.raises((AttributeError, TypeError)):
        s.sentence_type = "FAKE"


def test_strict_iteration_raises():
    # A sentence-shaped payload with a bad checksum reaches the parser
    # (garbage before a `$` is silently discarded in streaming mode, but
    # framing errors on a complete sentence surface as an error).
    p = StreamingParser()
    p.feed(b"$GPGGA,nothing*FF\r\n")
    with pytest.raises(EnvelopeError):
        list(p.iter(strict=True))


def test_lenient_iteration_skips_garbage():
    p = StreamingParser()
    p.feed(b"garbage " + GGA)
    sentences = list(p)
    assert len(sentences) == 1
    assert sentences[0].sentence_type == "GGA"


def test_next_sentence_raises_on_error():
    # Force a checksum error on a complete-looking sentence.
    bad = b"$GPGGA,nothing*FF\r\n"
    p = OneShotParser()
    p.feed(bad)
    with pytest.raises(EnvelopeError) as ei:
        p.next_sentence()
    assert hasattr(ei.value, "variant")
    assert ei.value.variant == "checksum_mismatch"


def test_parse_convenience():
    s = parse(GGA_NO_CRLF)
    assert s.sentence_type == "GGA"
    assert s.talker == b"GP"


def test_parse_raises_on_bad_input():
    with pytest.raises(EnvelopeError):
        parse(b"not a sentence")


def test_tag_block_preserved():
    # PRD §E4: a TAG block prefix `\...*hh\` is stripped from `raw` but its
    # content surfaces on the `tag_block` getter.
    bytes_in = _tag_sentence(b"c:1577836800", b"GPGGA,1,2,3")
    s = parse(bytes_in)
    assert s.tag_block == b"c:1577836800"
    assert s.sentence_type == "GGA"
    assert s.raw.startswith(b"$GPGGA")
    assert s.checksum_ok


def test_buffer_overflow_surfaces_variant():
    # Streaming buffer cap at 64 bytes; a 200-byte flood with no sentence
    # terminator forces the internal buffer to overflow, which the parser
    # reports on the next next_sentence() as BufferOverflow.
    p = StreamingParser(max_size=64)
    p.feed(b"$" + b"?" * 200)
    with pytest.raises(EnvelopeError) as ei:
        next(iter(p.iter(strict=True)))
    assert ei.value.variant == "buffer_overflow"


@pytest.mark.parametrize(
    "bytes_in,expected_variant",
    [
        # No `$`/`!` prefix: parse_sentence bails at start_delim.
        (b"garbage", "missing_start_delimiter"),
        # Body has no `*` to introduce the checksum.
        (b"$GPGGA,foo", "missing_checksum_delimiter"),
        # `*` is present but the two hex digits are non-hex.
        (b"$GPGGA,foo*ZZ", "invalid_checksum_digits"),
        # Structurally valid sentence with a deliberately-wrong checksum.
        (b"$GPGGA,nothing*FF", "checksum_mismatch"),
        # Non-proprietary address is only 1 byte long before the first comma.
        (b"$X,f*12", "talker_too_short"),
        # TAG-block-looking prefix with no closing `\`.
        (b"\\no_close_backslash", "malformed_tag_block"),
    ],
)
def test_envelope_error_variant(bytes_in: bytes, expected_variant: str):
    with pytest.raises(EnvelopeError) as ei:
        parse(bytes_in)
    assert ei.value.variant == expected_variant
