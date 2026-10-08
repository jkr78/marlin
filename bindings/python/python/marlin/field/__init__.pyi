"""Type stubs for marlin.field — the decoded-field state.

One runtime class serves every payload type. The stub declares
`FieldState(Generic[_T])` with the five variant classes nested as `@final`
generics on their own TypeVars, so a message stub writes
`speed_over_ground: FieldState[float]`, pyright narrows an `isinstance`
check to `FieldState.Value[float]`, and both checkers type the base `value`
exactly.

A message attribute is typed asymmetrically: the getter returns
`FieldState[T]` only, while the setter and the constructor accept
`FieldState[T] | T | None` and coerce a bare value to `Value` and `None`
to `NotAvailable()`:

    @property
    def speed_over_ground(self) -> FieldState[float]: ...
    @speed_over_ground.setter
    def speed_over_ground(self, value: FieldState[float] | float | None) -> None: ...
"""

from __future__ import annotations

from typing import Generic, Literal, TypeVar, final

from typing_extensions import Self, TypeAlias, disjoint_base

_T = TypeVar("_T")
_V = TypeVar("_V")
_B = TypeVar("_B")
_N = TypeVar("_N")
_S = TypeVar("_S")
_I = TypeVar("_I")

# The `kind` of a `FieldState`: its snake-case state tag.
Kind: TypeAlias = Literal["value", "at_least", "not_available", "sender_error", "invalid"]

# The one binding class that is not `@final`: the five variant classes
# derive from it. `@disjoint_base` is what stubtest requires of a PyO3 base.
@disjoint_base
class FieldState(Generic[_T]):
    """The decoded outcome of one wire field: the field state.

    Not instantiable; construct one of the five variant classes. Every
    state is truthy, `NotAvailable()` included: test the state, never the
    object.
    """

    @property
    def kind(self) -> Kind: ...
    @property
    def value(self) -> _T | None: ...
    @property
    def value_or_bound(self) -> _T | None: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

    @final
    class Value(FieldState[_V]):
        """The wire gave the field a meaning and the decoder accepted it."""

        __match_args__ = ("value",)
        def __new__(cls, value: _V) -> Self: ...
        @property
        def value(self) -> _V: ...

    @final
    class AtLeast(FieldState[_B]):
        """Over-range: the true value is at or beyond `bound`."""

        __match_args__ = ("bound",)
        def __new__(cls, bound: _B) -> Self: ...
        @property
        def bound(self) -> _B: ...
        @property
        def value(self) -> None: ...
        @property
        def value_or_bound(self) -> _B: ...

    @final
    class NotAvailable(FieldState[_N]):
        """The sender supplied no value. Not an error."""

        __match_args__ = ()
        def __new__(cls) -> Self: ...
        @property
        def value(self) -> None: ...
        @property
        def value_or_bound(self) -> None: ...

    @final
    class SenderError(FieldState[_S]):
        """The sender knows it has no usable value; `code` is the raw code."""

        __match_args__ = ("code",)
        def __new__(cls, code: int) -> Self: ...
        @property
        def code(self) -> int: ...
        @property
        def value(self) -> None: ...
        @property
        def value_or_bound(self) -> None: ...

    @final
    class Invalid(FieldState[_I]):
        """The decoder could not give the wire field a meaning.

        `code` is the raw code of an undefined number, or `None` when no
        wire integer could be read.
        """

        __match_args__ = ("code",)
        def __new__(cls, code: int | None) -> Self: ...
        @property
        def code(self) -> int | None: ...
        @property
        def value(self) -> None: ...
        @property
        def value_or_bound(self) -> None: ...

__all__ = ["FieldState", "Kind"]
