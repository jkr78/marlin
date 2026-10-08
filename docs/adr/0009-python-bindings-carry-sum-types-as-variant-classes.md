# Python bindings carry sum types as variant classes

Supersedes [ADR-0003](0003-python-binding-classes-flatten-sum-types.md)
from 0.3.0.

ADR-0003 flattened a Rust enum in a field position into sibling optional
attributes because the bindings carried only int-backed enums and a
payload-carrying class would have broken that convention. From 0.3.0 every
decoded field that can be in a non-value state is a `FieldState`
(ADR-0008), a payload-carrying sum type on nearly every message, so the
convention it protected is gone and the flattening rule goes with it.

`marlin.field.FieldState` is a frozen tagged value class with five nested
variant classes: `FieldState.Value(value)`, `FieldState.AtLeast(bound)`,
`FieldState.NotAvailable()`, `FieldState.SenderError(code)` and
`FieldState.Invalid(code)`, where `Invalid.code` is `None` for a field with
no wire integer. A client reads a field through `isinstance`, `match`, or
the base-class surface: `kind` (the snake-case tag from the Rust
`Kind::name()`, so the two languages share one table), `value` and
`value_or_bound` (the Rust accessors, `None` outside their states), `__eq__`
and `__hash__`. There is no `__bool__` override: `if msg.speed_over_ground:`
is always true, which the changelog names as a trap, because a falsy
`NotAvailable` would make `Value(0.0)` indistinguishable from it.

One runtime class serves every payload. The stub declares
`FieldState(Generic[_T])` with the variants as nested `@final` classes on
their own TypeVars, so a message stub says `speed_over_ground:
FieldState[float]`; pyright narrows an `isinstance` check to
`Value[float]`, mypy to `Value[Any]`, and both type the base `value`
exactly. Payload type safety lives on the write side: constructors and
setters accept `FieldState[T] | T | None`, coerce a bare value to `Value`
and `None` to `NotAvailable()`, and raise `TypeError` when the payload does
not convert to the field's Rust type. Getters return `FieldState[T]` only.

The same shape carries the field-position enums ADR-0003 flattened:
`RateOfTurn.DegPerMin` / `RateOfTurn.NoIndicator`, `Timestamp.Second` /
`Timestamp.PositioningStatus`, and `Type24BExtent.Dimensions` /
`Type24BExtent.MothershipMmsi` replace the `rate_of_turn` +
`turn_direction` and `dimensions` + `mothership_mmsi` sibling pairs. The
dataclass mirrors follow: five generic frozen dataclasses with a `kind`
literal, so `asdict` yields `{"kind": "value", "value": 10.2}` for a field.
Message-level enums keep one binding class per message struct plus
`type_tag`.

The Python floor rises from 3.9 to 3.10 so the documentation and tests can
show the `match` idiom the design is built around; 3.9 is past end of life.

## Considered options

- An `Optional` value plus a parallel status attribute per field: cheap
  reads, but the two attributes are coupled by name only and a client can
  read the value without seeing the status.
- A class family stamped per payload type (`FloatField`, `IntField`, ...):
  exact under mypy, but a caller must know the payload class to construct
  one, and the family is about twenty classes for one concept.
- A raising `__bool__` as `pandas.NA` does: surprising in `and`/`or`
  chains; an always-true object plus a named trap was judged safer.
- An int-backed `Kind` enum: the house discriminator is a string
  (`type_tag`, `variant`) and the tag is shared with Rust by name.
- Flattening in the mirrors only: two shapes for one field.
