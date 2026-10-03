"""Each public stub exports the same names as the runtime module beside it."""

import ast
import importlib
from pathlib import Path

import pytest

STUBBED_MODULES = [
    "marlin",
    "marlin.ais",
    "marlin.nmea",
    "marlin.envelope",
    "marlin.klv",
]


def stub_all(stub_path: Path) -> list[str]:
    """Read the literal `__all__ = [...]` list out of a `.pyi` file."""
    tree = ast.parse(stub_path.read_text(encoding="utf-8"))
    for node in tree.body:
        if not isinstance(node, ast.Assign):
            continue
        targets = [t.id for t in node.targets if isinstance(t, ast.Name)]
        if targets == ["__all__"]:
            names = ast.literal_eval(node.value)
            assert isinstance(names, list)
            return names
    raise AssertionError(f"{stub_path} has no `__all__ = [...]`")


@pytest.mark.parametrize("module_name", STUBBED_MODULES)
def test_stub_all_matches_runtime_all(module_name: str) -> None:
    module = importlib.import_module(module_name)
    assert module.__file__ is not None
    stub_path = Path(module.__file__).with_suffix(".pyi")

    assert set(stub_all(stub_path)) == set(module.__all__)
