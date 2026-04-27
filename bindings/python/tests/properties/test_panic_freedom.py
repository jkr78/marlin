from hypothesis import given, strategies as st, settings
import pytest
from marlin import MarlinError
from marlin.envelope import StreamingParser
from marlin.nmea import Nmea0183Parser
from marlin.ais import AisParser


byte_streams = st.binary(min_size=0, max_size=8192)


@given(data=byte_streams)
@settings(max_examples=500, deadline=None)
def test_envelope_panic_free(data):
    p = StreamingParser()
    try:
        p.feed(data)
        for _ in p.iter(strict=True):
            pass
    except MarlinError:
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception {type(e).__name__}: {e}")


@given(data=byte_streams)
@settings(max_examples=500, deadline=None)
def test_nmea_panic_free(data):
    p = Nmea0183Parser.streaming()
    try:
        p.feed(data)
        for _ in p.iter(strict=True):
            pass
    except MarlinError:
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception {type(e).__name__}: {e}")


@given(data=byte_streams)
@settings(max_examples=500, deadline=None)
def test_ais_panic_free(data):
    p = AisParser.streaming()
    try:
        p.feed(data)
        for _ in p.iter(strict=True):
            pass
    except MarlinError:
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception {type(e).__name__}: {e}")
