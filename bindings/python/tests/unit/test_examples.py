"""Smoke tests: each example script runs and produces expected output."""

import subprocess
import sys
from pathlib import Path
from typing import Optional

EXAMPLES = Path(__file__).parent.parent.parent / "examples"
FIXTURES = Path(__file__).parent.parent / "fixtures"


def _run_example(
    name: str, *args: str, stdin_bytes: Optional[bytes] = None
) -> "subprocess.CompletedProcess[bytes]":
    return subprocess.run(
        [sys.executable, str(EXAMPLES / name), *args],
        input=stdin_bytes,
        capture_output=True,
        check=True,
        timeout=10,
    )


def test_one_shot_no_crlf_runs() -> None:
    result = _run_example("one_shot_no_crlf.py")
    assert b"GGA" in result.stdout


def test_parse_log_file_runs() -> None:
    fixture = FIXTURES / "envelope" / "01_gga_basic.nmea"
    result = _run_example("parse_log_file.py", str(fixture))
    assert b"GGA" in result.stdout


def test_streaming_tcp_style_runs() -> None:
    result = _run_example("streaming_tcp_style.py")
    assert b"total sentences:" in result.stdout
    # 3 sentences in the inline payload — must reassemble cleanly across chunks.
    assert b"total sentences: 3" in result.stdout


def test_decode_aivdm_log_runs() -> None:
    result = _run_example("decode_aivdm_log.py")
    # At minimum: 2 messages decoded (Type 1 + reassembled Type 5).
    assert result.stdout.count(b"mmsi=") >= 2


def test_parse_stdin_runs() -> None:
    fixture = FIXTURES / "envelope" / "01_gga_basic.nmea"
    stdin_bytes = fixture.read_bytes()
    result = _run_example("parse_stdin.py", stdin_bytes=stdin_bytes)
    assert b"GGA" in result.stdout


def test_live_ais_dashboard_runs() -> None:
    fixture = FIXTURES / "envelope" / "03_aivdm_encapsulation.nmea"
    stdin_bytes = fixture.read_bytes()
    result = _run_example("live_ais_dashboard.py", stdin_bytes=stdin_bytes)
    assert b"marlin live AIS dashboard" in result.stdout
    assert b"frames:" in result.stdout
    # The fixture is a Type 1 position report; dashboard must decode it.
    assert b"type1" in result.stdout
