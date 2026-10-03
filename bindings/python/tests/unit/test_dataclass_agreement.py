"""Each dataclass mirror has the same fields as the binding class it copies."""

import dataclasses
import importlib
import inspect
import types

import pytest

import marlin.dataclasses

BINDING_MODULES = ["marlin.ais", "marlin.nmea", "marlin.envelope"]

DATACLASS_MIRRORS = sorted(
    name
    for name, obj in vars(marlin.dataclasses).items()
    if isinstance(obj, type) and dataclasses.is_dataclass(obj)
)


def binding_classes() -> list[type]:
    """Every public class of the binding modules, exceptions left out."""
    classes = []
    for module_name in BINDING_MODULES:
        module = importlib.import_module(module_name)
        for name in module.__all__:
            obj = getattr(module, name)
            if isinstance(obj, type) and not issubclass(obj, Exception):
                classes.append(obj)
    return classes


def binding_fields(binding_class: type) -> set[str]:
    """The read-only data attributes PyO3 exposes, methods left out."""
    return {
        name
        for name in dir(binding_class)
        if not name.startswith("_")
        and isinstance(
            inspect.getattr_static(binding_class, name), types.GetSetDescriptorType
        )
    }


def test_no_binding_class_name_repeats_across_modules() -> None:
    names = [cls.__name__ for cls in binding_classes()]

    assert sorted(names) == sorted(set(names))


@pytest.mark.parametrize("mirror_name", DATACLASS_MIRRORS)
def test_dataclass_mirror_has_the_binding_class_fields(mirror_name: str) -> None:
    by_name = {cls.__name__: cls for cls in binding_classes()}
    mirror = getattr(marlin.dataclasses, mirror_name)

    assert mirror_name in by_name, "dataclass mirror without a binding class"
    mirror_fields = {f.name for f in dataclasses.fields(mirror)}
    assert mirror_fields == binding_fields(by_name[mirror_name])
