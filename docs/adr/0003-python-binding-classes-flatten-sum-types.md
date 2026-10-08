# Python binding classes flatten Rust field-level sum types

_Superseded by [ADR-0009](0009-python-bindings-carry-sum-types-as-variant-classes.md) from 0.3.0._

A Rust enum with data in a *field* position (`RateOfTurn`, `Type24BExtent`) becomes
sibling optional attributes on the binding class and on its dataclass mirror
(`rate_of_turn` + `turn_direction`; `dimensions` + `mothership_mmsi`). On parser
output at most one is set: exactly one for `Type24BExtent`, none when the Rust
field is itself `None` (ROT −128). The constructors do not validate hand-built instances. Message-level enums keep the
existing convention: one binding class per message struct (Types 1, 2, and 3 share
`PositionReportA`) plus a `type_tag` string that names the variant.

## Considered options

PyO3 0.22+ supports data-carrying ("complex") enums, so `RateOfTurn` could have been
one. Rejected: the bindings' enums are int-backed `eq, eq_int` binding classes that
the dataclass mirrors store as `int`; a payload-carrying enum breaks that convention and
forces `isinstance` or `match` on consumers, and a nested value class for a single
field adds an unwrap to every read.
