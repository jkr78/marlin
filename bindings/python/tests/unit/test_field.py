"""Tests for marlin.field.FieldState, the decoded-field state class."""

from __future__ import annotations

from typing import get_args

import pytest

from marlin.field import FieldState, Kind

# ---------- construction and the five states ----------


def test_each_variant_is_an_instance_of_the_base_and_of_itself() -> None:
    value: FieldState[float] = FieldState.Value(10.2)
    at_least: FieldState[float] = FieldState.AtLeast(102.2)
    not_available: FieldState[float] = FieldState.NotAvailable()
    sender_error: FieldState[float] = FieldState.SenderError(-2147483648)
    invalid: FieldState[float] = FieldState.Invalid(91)

    assert isinstance(value, FieldState)
    assert isinstance(value, FieldState.Value)
    assert isinstance(at_least, FieldState.AtLeast)
    assert isinstance(not_available, FieldState.NotAvailable)
    assert isinstance(sender_error, FieldState.SenderError)
    assert isinstance(invalid, FieldState.Invalid)
    assert not isinstance(value, FieldState.AtLeast)


def test_variant_class_names_are_the_bare_variant_names() -> None:
    assert type(FieldState.Value(1)).__name__ == "Value"
    assert type(FieldState.AtLeast(1)).__name__ == "AtLeast"
    assert type(FieldState.NotAvailable()).__name__ == "NotAvailable"
    assert type(FieldState.SenderError(1)).__name__ == "SenderError"
    assert type(FieldState.Invalid(None)).__name__ == "Invalid"
    assert FieldState.Value.__module__ == "marlin.field"
    assert FieldState.Value.__qualname__ == "FieldState.Value"


def test_base_class_is_not_instantiable() -> None:
    with pytest.raises(TypeError, match="cannot create 'marlin.field.FieldState' instances"):
        FieldState()


def test_kind_is_the_snake_case_tag_shared_with_rust() -> None:
    assert FieldState.Value(10.2).kind == "value"
    assert FieldState.AtLeast(102.2).kind == "at_least"
    assert FieldState.NotAvailable().kind == "not_available"
    assert FieldState.SenderError(-1).kind == "sender_error"
    assert FieldState.Invalid(91).kind == "invalid"
    assert FieldState.Invalid(None).kind == "invalid"


def test_kind_alias_lists_exactly_the_runtime_tags() -> None:
    """The `Kind` literal is a copy of the Rust table; this pins the two."""
    runtime_tags = {
        FieldState.Value(0).kind,
        FieldState.AtLeast(0).kind,
        FieldState.NotAvailable().kind,
        FieldState.SenderError(0).kind,
        FieldState.Invalid(None).kind,
    }

    assert set(get_args(Kind)) == runtime_tags


# ---------- payload accessors ----------


def test_value_is_the_payload_only_in_the_value_state() -> None:
    assert FieldState.Value(10.2).value == 10.2
    assert FieldState.AtLeast(102.2).value is None
    assert FieldState.NotAvailable().value is None
    assert FieldState.SenderError(-1).value is None
    assert FieldState.Invalid(91).value is None


def test_value_or_bound_accepts_the_over_range_bound() -> None:
    assert FieldState.Value(10.2).value_or_bound == 10.2
    assert FieldState.AtLeast(102.2).value_or_bound == 102.2
    assert FieldState.NotAvailable().value_or_bound is None
    assert FieldState.SenderError(-1).value_or_bound is None
    assert FieldState.Invalid(91).value_or_bound is None


def test_variant_fields_are_readable_by_name() -> None:
    assert FieldState.AtLeast(102.2).bound == 102.2
    assert FieldState.SenderError(-2147483648).code == -2147483648
    assert FieldState.Invalid(91).code == 91
    assert FieldState.Invalid(None).code is None


def test_value_payload_is_any_python_object() -> None:
    assert FieldState.Value("EVER GIVEN").value == "EVER GIVEN"
    assert FieldState.Value(b"\x01").value == b"\x01"
    assert FieldState.Value(None).value is None
    assert FieldState.Value((1, 2)).value == (1, 2)


def test_variants_are_frozen() -> None:
    state = FieldState.Value(1.0)

    with pytest.raises(AttributeError, match="attribute 'value' of 'Value' objects is not writable"):
        state.value = 2.0  # type: ignore[misc]


# ---------- equality and hashing ----------


def test_equal_payloads_in_the_same_state_are_equal() -> None:
    assert FieldState.Value(1.0) == FieldState.Value(1.0)
    assert FieldState.Value(1.0) == FieldState.Value(1)
    assert FieldState.AtLeast(25.5) == FieldState.AtLeast(25.5)
    assert FieldState.NotAvailable() == FieldState.NotAvailable()
    assert FieldState.SenderError(-1) == FieldState.SenderError(-1)
    assert FieldState.Invalid(None) == FieldState.Invalid(None)
    assert FieldState.Invalid(91) == FieldState.Invalid(91)


def test_different_states_or_payloads_are_not_equal() -> None:
    assert FieldState.Value(1.0) != FieldState.AtLeast(1.0)
    assert FieldState.Value(1.0) != FieldState.Value(2.0)
    assert FieldState.Invalid(91) != FieldState.Invalid(None)
    assert FieldState.SenderError(-1) != FieldState.Invalid(-1)
    assert FieldState.NotAvailable() != None  # noqa: E711
    assert FieldState.Value(1.0) != 1.0


def test_equal_states_hash_alike_and_work_as_dict_keys() -> None:
    assert hash(FieldState.Value(1.0)) == hash(FieldState.Value(1.0))
    assert hash(FieldState.NotAvailable()) == hash(FieldState.NotAvailable())
    assert hash(FieldState.Invalid(91)) == hash(FieldState.Invalid(91))

    seen = {FieldState.Value(1.0): "a", FieldState.AtLeast(1.0): "b"}
    assert seen[FieldState.Value(1.0)] == "a"
    assert seen[FieldState.AtLeast(1.0)] == "b"
    assert len({FieldState.NotAvailable(), FieldState.NotAvailable()}) == 1


def test_unhashable_payload_makes_the_state_unhashable() -> None:
    with pytest.raises(TypeError, match="unhashable type: 'list'"):
        hash(FieldState.Value([1, 2]))


# ---------- truthiness ----------


def test_every_state_is_truthy_including_not_available() -> None:
    assert bool(FieldState.NotAvailable()) is True
    assert bool(FieldState.Value(0.0)) is True
    assert bool(FieldState.Invalid(None)) is True


# ---------- pattern matching ----------


def describe(state: FieldState[float]) -> str:
    match state:
        case FieldState.Value(v):
            return f"value {v}"
        case FieldState.AtLeast(b):
            return f"at least {b}"
        case FieldState.NotAvailable():
            return "not available"
        case FieldState.SenderError(code):
            return f"sender error {code}"
        case FieldState.Invalid(None):
            return "unparsable"
        case FieldState.Invalid(code):
            return f"undefined code {code}"
    raise AssertionError("unreachable")


def test_match_args_support_positional_patterns() -> None:
    assert FieldState.Value.__match_args__ == ("value",)
    assert FieldState.AtLeast.__match_args__ == ("bound",)
    assert FieldState.NotAvailable.__match_args__ == ()
    assert FieldState.SenderError.__match_args__ == ("code",)
    assert FieldState.Invalid.__match_args__ == ("code",)

    assert describe(FieldState.Value(10.2)) == "value 10.2"
    assert describe(FieldState.AtLeast(102.2)) == "at least 102.2"
    assert describe(FieldState.NotAvailable()) == "not available"
    assert describe(FieldState.SenderError(-128)) == "sender error -128"
    assert describe(FieldState.Invalid(None)) == "unparsable"
    assert describe(FieldState.Invalid(91)) == "undefined code 91"


# ---------- repr ----------


def test_repr_is_the_constructor_call() -> None:
    assert repr(FieldState.Value(10.2)) == "FieldState.Value(10.2)"
    assert repr(FieldState.Value("EVER GIVEN")) == "FieldState.Value('EVER GIVEN')"
    assert repr(FieldState.AtLeast(102.2)) == "FieldState.AtLeast(102.2)"
    assert repr(FieldState.NotAvailable()) == "FieldState.NotAvailable()"
    assert repr(FieldState.SenderError(-1)) == "FieldState.SenderError(-1)"
    assert repr(FieldState.Invalid(91)) == "FieldState.Invalid(91)"
    assert repr(FieldState.Invalid(None)) == "FieldState.Invalid(None)"


# ---------- payload types the wire codes constrain ----------


def test_sender_error_code_must_be_an_int() -> None:
    with pytest.raises(TypeError, match="'str' object cannot be interpreted as an integer"):
        FieldState.SenderError("x")  # type: ignore[arg-type]


def test_invalid_code_must_be_an_int_or_none() -> None:
    with pytest.raises(TypeError, match="'float' object cannot be interpreted as an integer"):
        FieldState.Invalid(1.5)  # type: ignore[arg-type]
