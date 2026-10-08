"""Each dataclass mirror has the same fields as the binding class it copies."""

import dataclasses
import importlib
import inspect
import types

import pytest

import marlin.dataclasses

# The converter's own list: a module missing from it is not converted at all.
BINDING_MODULES = marlin.dataclasses._BINDING_MODULES

DATACLASS_MIRRORS = sorted(
    name
    for name, obj in vars(marlin.dataclasses).items()
    if isinstance(obj, type) and dataclasses.is_dataclass(obj)
)


def binding_classes() -> list[type]:
    """Every public class of the binding modules, exceptions left out.

    A nested variant class (`FieldState.Value`) counts as a class of its own:
    it is what a field holds at runtime and what the mirror is named after.
    """
    classes = []
    for module_name in BINDING_MODULES:
        module = importlib.import_module(module_name)
        for name in module.__all__:
            obj = getattr(module, name)
            if isinstance(obj, type) and not issubclass(obj, Exception):
                classes.append(obj)
                classes.extend(variant_classes(obj))
    return classes


def variant_classes(binding_class: type) -> list[type]:
    """The nested variant classes of a sum type, in attribute order."""
    return [
        obj
        for obj in vars(binding_class).values()
        if isinstance(obj, type) and issubclass(obj, binding_class)
    ]


def is_variant_class(binding_class: type) -> bool:
    """A variant class derives from its sum type; a plain binding class, from `object`."""
    return any(base is not object for base in binding_class.__bases__)


def binding_fields(binding_class: type) -> set[str]:
    """The read-only data attributes PyO3 exposes, methods left out.

    For a variant class, the data is the variant's own fields plus the
    `kind` tag; the base class's other getters (`value`, `value_or_bound`)
    are accessors computed across states, not data a mirror carries.
    """
    if is_variant_class(binding_class):
        return {
            name
            for name, obj in vars(binding_class).items()
            if not name.startswith("_") and isinstance(obj, types.GetSetDescriptorType)
        } | {"kind"}
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
