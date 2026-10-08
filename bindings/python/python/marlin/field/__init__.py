"""Decoded-field state: the five states one wire field decodes to."""

from typing import Literal

from .. import _core

FieldState = _core.field.FieldState

# The `kind` of a `FieldState`: its snake-case state tag.
Kind = Literal["value", "at_least", "not_available", "sender_error", "invalid"]

__all__ = ["FieldState", "Kind"]
