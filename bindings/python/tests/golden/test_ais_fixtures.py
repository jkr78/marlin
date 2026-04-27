"""Golden round-trip tests for the AIS decoder."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from marlin.ais import AisParser

FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "envelope"
EXPECTED_DIR = Path(__file__).parent / "expected" / "ais"
EXPECTED_DIR.mkdir(parents=True, exist_ok=True)


def _is_getter(attr: Any) -> bool:
    """True for @property or PyO3's getset_descriptor/member_descriptor."""
    if isinstance(attr, property):
        return True
    return type(attr).__name__ in {"getset_descriptor", "member_descriptor"}


def _to_json_safe(obj: Any) -> Any:
    # Same implementation as in test_nmea_fixtures.py. If you prefer
    # de-duplication, lift to a shared module under tests/golden/_util.py;
    # two copies at ~25 lines each is acceptable for v0.1.
    if obj is None:
        return None
    if isinstance(obj, (bool, int, float, str)):
        return obj
    if isinstance(obj, bytes):
        return {"__bytes__": obj.hex()}
    if isinstance(obj, (list, tuple)):
        return [_to_json_safe(x) for x in obj]
    if isinstance(obj, dict):
        return {k: _to_json_safe(v) for k, v in obj.items()}
    try:
        return {"__enum__": type(obj).__name__, "value": int(obj)}
    except (TypeError, ValueError):
        pass
    cls = type(obj)
    result: dict[str, Any] = {"__class__": cls.__name__}
    for name in sorted(dir(cls)):
        if name.startswith("_"):
            continue
        attr = getattr(cls, name, None)
        if _is_getter(attr):
            result[name] = _to_json_safe(getattr(obj, name))
    return result


@pytest.mark.parametrize(
    "fixture",
    sorted(FIXTURES_DIR.glob("*.nmea")),
    ids=lambda p: p.name,
)
def test_ais_fixture_matches_golden(
    fixture: Path, regenerate_goldens: bool
) -> None:
    raw = fixture.read_bytes()
    # timeout_ms=None → reassembler never reads the clock; deterministic.
    parser = AisParser.streaming(timeout_ms=None)
    parser.feed(raw)
    actual = [_to_json_safe(msg) for msg in parser]

    expected_path = EXPECTED_DIR / f"{fixture.stem}.json"

    if regenerate_goldens:
        expected_path.write_text(
            json.dumps(actual, indent=2, sort_keys=True) + "\n"
        )
        return

    assert expected_path.exists(), (
        f"Missing golden file {expected_path}. "
        f"Run MARLIN_REGENERATE_GOLDENS=1 to create it."
    )
    expected = json.loads(expected_path.read_text())
    assert actual == expected, f"Golden mismatch for {fixture.name}"
