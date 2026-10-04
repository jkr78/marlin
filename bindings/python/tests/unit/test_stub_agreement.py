"""Each public stub exports the same names as the runtime module beside it."""

import ast
import importlib
import typing
from pathlib import Path

import pytest

import marlin

PACKAGE_DIR = Path(marlin.__file__).parent


def stubbed_modules() -> list[str]:
    """Every package under `marlin` with an `__init__.pyi` beside it."""
    names = []
    for stub_path in PACKAGE_DIR.rglob("__init__.pyi"):
        parts = stub_path.parent.relative_to(PACKAGE_DIR.parent).parts
        names.append(".".join(parts))
    return sorted(names)


STUBBED_MODULES = stubbed_modules()


def parse_stub(module_name: str) -> ast.Module:
    module = importlib.import_module(module_name)
    assert module.__file__ is not None
    stub_path = Path(module.__file__).with_suffix(".pyi")
    return ast.parse(stub_path.read_text(encoding="utf-8"))


def stub_all(stub: ast.Module) -> list[str]:
    """Read the literal `__all__ = [...]` list out of a parsed `.pyi`."""
    for node in stub.body:
        if not isinstance(node, ast.Assign):
            continue
        targets = [t.id for t in node.targets if isinstance(t, ast.Name)]
        if targets == ["__all__"]:
            names = ast.literal_eval(node.value)
            assert isinstance(names, list)
            return names
    raise AssertionError("stub has no `__all__ = [...]`")


def stub_alias_members(stub: ast.Module) -> dict[str, set[str]]:
    """Map each `Name: TypeAlias = Union[...]` / `Literal[...]` to its members.

    A `Union` member is a class name; a `Literal` member is its value.
    """
    aliases = {}
    for node in stub.body:
        if not (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and isinstance(node.annotation, ast.Name)
            and node.annotation.id == "TypeAlias"
        ):
            continue
        assert isinstance(node.value, ast.Subscript), node.target.id
        members = node.value.slice
        elements = members.elts if isinstance(members, ast.Tuple) else [members]
        aliases[node.target.id] = {
            e.id if isinstance(e, ast.Name) else ast.literal_eval(e) for e in elements
        }
    return aliases


def test_the_glob_finds_every_public_stub() -> None:
    assert set(STUBBED_MODULES) >= {
        "marlin",
        "marlin.ais",
        "marlin.nmea",
        "marlin.envelope",
        "marlin.klv",
    }


@pytest.mark.parametrize("module_name", STUBBED_MODULES)
def test_stub_all_matches_runtime_all(module_name: str) -> None:
    module = importlib.import_module(module_name)

    assert set(stub_all(parse_stub(module_name))) == set(module.__all__)


@pytest.mark.parametrize("module_name", STUBBED_MODULES)
def test_stub_type_aliases_match_runtime_aliases(module_name: str) -> None:
    module = importlib.import_module(module_name)

    for alias, stub_members in stub_alias_members(parse_stub(module_name)).items():
        runtime_members = {
            m.__name__ if isinstance(m, type) else m
            for m in typing.get_args(getattr(module, alias))
        }
        assert runtime_members == stub_members, alias
