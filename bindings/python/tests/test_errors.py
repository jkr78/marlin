import pytest
from marlin import MarlinError
from marlin.envelope import EnvelopeError
from marlin.nmea import DecodeError
from marlin.ais import AisError, ReassemblyError


def test_hierarchy():
    assert issubclass(EnvelopeError, MarlinError)
    assert issubclass(DecodeError, MarlinError)
    assert issubclass(AisError, MarlinError)
    assert issubclass(ReassemblyError, AisError)


def test_envelope_error_variant_attribute():
    # Variant is a string attribute; set manually in this test.
    # (Real raising paths are tested in test_envelope.py.)
    err = EnvelopeError("malformed tag block")
    err.variant = "malformed_tag_block"
    assert err.variant == "malformed_tag_block"
    with pytest.raises(EnvelopeError):
        raise err
