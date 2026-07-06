"""Golden byte-exact encode test for marlin.klv (ST 0601)."""

from __future__ import annotations

from marlin.klv import St0601, encode


def test_golden_timestamp_version_packet() -> None:
    # Byte-exact mirror of the Rust `timestamp_and_version_encode_byte_exact`
    # golden: UAS LS key, outer BER length 0x11, Tag 2 timestamp, Tag 65
    # version, Tag 1 checksum 0x71AC. The Rust golden is authoritative.
    s = St0601(timestamp_us=0x0001_0203_0405_0607, version=0x0B)
    expected = bytes(
        [
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x0B, 0x01, 0x01,
            0x0E, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
            0x11,
            0x02, 0x08, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x41, 0x01, 0x0B,
            0x01, 0x02, 0x71, 0xAC,
        ]
    )
    assert encode(s) == expected
