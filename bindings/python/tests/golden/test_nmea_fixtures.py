"""Golden round-trip tests for the NMEA 0183 typed decoder."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from marlin.nmea import Nmea0183Parser

FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "envelope"
EXPECTED_DIR = Path(__file__).parent / "expected" / "nmea"
EXPECTED_DIR.mkdir(parents=True, exist_ok=True)


def _is_getter(attr: Any) -> bool:
    """True for @property or PyO3's getset_descriptor/member_descriptor."""
    if isinstance(attr, property):
        return True
    # PyO3 #[pyo3(get)] produces getset_descriptor; there is no public
    # name for it, so sniff by the type's tp_name string.
    return type(attr).__name__ in {"getset_descriptor", "member_descriptor"}


def _to_json_safe(obj: Any) -> Any:
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
    # PyO3 int-backed enum pyclass — prefer the wire int.
    try:
        return {"__enum__": type(obj).__name__, "value": int(obj)}
    except (TypeError, ValueError):
        pass
    # A pyclass with @property (or PyO3 getset_descriptor) getters. Walk them.
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
def test_nmea_fixture_matches_golden(
    fixture: Path, regenerate_goldens: bool
) -> None:
    raw = fixture.read_bytes()
    parser = Nmea0183Parser.streaming()
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
