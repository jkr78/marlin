"""Each dataclass mirror has the same fields as the binding class it copies."""

import dataclasses
import importlib
import inspect
import types

import pytest

import marlin.dataclasses

# The converter's own list: a module missing from it is not converted at all.
BINDING_MODULES = marlin.dataclasses._BINDING_MODULES

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

    For a variant class, the data is the variant's own fields plus the sum
    type's `kind` tag where it has one (`FieldState`); the base class's
    other getters (`value`, `value_or_bound`) are accessors computed across
    states, not data a mirror carries.
    """
    if is_variant_class(binding_class):
        own = {
            name
            for name, obj in vars(binding_class).items()
            if not name.startswith("_") and isinstance(obj, types.GetSetDescriptorType)
        }
        tag = {"kind"} if hasattr(binding_class, "kind") else set()
        return own | tag
    return {
        name
        for name in dir(binding_class)
        if not name.startswith("_")
        and isinstance(
            inspect.getattr_static(binding_class, name), types.GetSetDescriptorType
        )
    }


def mirror_classes() -> list[type]:
    """Every dataclass mirror: the module-level ones and those nested under
    a namespace class named after a sum type (`Type24BExtent.Dimensions`)."""
    mirrors = []
    for obj in vars(marlin.dataclasses).values():
        if not isinstance(obj, type):
            continue
        if dataclasses.is_dataclass(obj):
            mirrors.append(obj)
        else:
            mirrors.extend(
                nested
                for nested in vars(obj).values()
                if isinstance(nested, type) and dataclasses.is_dataclass(nested)
            )
    return mirrors


MIRROR_QUALNAMES = sorted(cls.__qualname__ for cls in mirror_classes())


def test_no_binding_class_qualified_name_repeats_across_modules() -> None:
    # The converter keys on the qualified name: `Type24BExtent.Dimensions`
    # and `Dimensions` may coexist, two classes called `Dimensions` may not.
    names = [cls.__qualname__ for cls in binding_classes()]

    assert sorted(names) == sorted(set(names))


@pytest.mark.parametrize("mirror_qualname", MIRROR_QUALNAMES)
def test_dataclass_mirror_has_the_binding_class_fields(mirror_qualname: str) -> None:
    by_qualname = {cls.__qualname__: cls for cls in binding_classes()}
    # The FieldState mirrors are flat (`Value`), their binding classes nested.
    binding_qualname = (
        f"FieldState.{mirror_qualname}"
        if mirror_qualname in {"Value", "AtLeast", "NotAvailable", "SenderError", "Invalid"}
        else mirror_qualname
    )
    mirror = {cls.__qualname__: cls for cls in mirror_classes()}[mirror_qualname]

    assert binding_qualname in by_qualname, "dataclass mirror without a binding class"
    mirror_fields = {f.name for f in dataclasses.fields(mirror)}
    assert mirror_fields == binding_fields(by_qualname[binding_qualname])
