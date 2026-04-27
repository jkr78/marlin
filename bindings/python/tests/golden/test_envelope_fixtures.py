"""Golden round-trip tests for the envelope parser.

Fixtures come from the Rust crate (crates/marlin-nmea-envelope/tests/
fixtures/). Set MARLIN_REGENERATE_GOLDENS=1 to rewrite expected JSONs
after an intentional API change.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from marlin.envelope import StreamingParser

FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "envelope"
EXPECTED_DIR = Path(__file__).parent / "expected" / "envelope"
EXPECTED_DIR.mkdir(parents=True, exist_ok=True)


def _to_json_safe(obj: Any) -> Any:
    """Serialize bytes/tuples/dicts/enum-ish pyclasses to JSON-safe form."""
    if isinstance(obj, bytes):
        return {"__bytes__": obj.hex()}
    if isinstance(obj, tuple):
        return [_to_json_safe(x) for x in obj]
    if isinstance(obj, list):
        return [_to_json_safe(x) for x in obj]
    if isinstance(obj, dict):
        return {k: _to_json_safe(v) for k, v in obj.items()}
    return obj


@pytest.mark.parametrize(
    "fixture",
    sorted(FIXTURES_DIR.glob("*.nmea")),
    ids=lambda p: p.name,
)
def test_envelope_fixture_matches_golden(
    fixture: Path, regenerate_goldens: bool
) -> None:
    # Feed the whole fixture into a streaming parser; collect every
    # successful sentence (garbage is skipped in lenient mode).
    raw = fixture.read_bytes()
    parser = StreamingParser()
    parser.feed(raw)
    actual = [_to_json_safe(sentence.as_dict()) for sentence in parser]

    expected_path = EXPECTED_DIR / f"{fixture.stem}.json"

    if regenerate_goldens:
        expected_path.write_text(
            json.dumps(actual, indent=2, sort_keys=True) + "\n"
        )
        return

    assert expected_path.exists(), (
        f"Missing golden file {expected_path}. "
        f"Run `MARLIN_REGENERATE_GOLDENS=1 .venv/bin/python -m pytest "
        f"bindings/python/tests/golden/` to create it."
    )
    expected = json.loads(expected_path.read_text())
    assert actual == expected, f"Golden mismatch for {fixture.name}"
