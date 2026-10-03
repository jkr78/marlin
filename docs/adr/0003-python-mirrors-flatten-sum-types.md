# Python mirrors flatten Rust field-level sum types

A Rust enum with data in a *field* position (`RateOfTurn`, `Type24BExtent`) becomes
sibling optional attributes on the Python class (`rate_of_turn` + `turn_direction`;
`dimensions` + `mothership_mmsi`). On parser output at most one is set: exactly one
for `Type24BExtent`, none when the Rust field is itself `None` (ROT −128). The
constructors do not validate hand-built instances. Message-level enums keep the
existing convention: one class per message struct (Types 1, 2, and 3 share
`PositionReportA`) plus a `type_tag` string that names the variant.

## Considered options

PyO3 0.22+ supports data-carrying ("complex") enums, so `RateOfTurn` could have been
one. Rejected: the bindings' enums are int-backed `eq, eq_int` classes mirrored as
`int` in `marlin.dataclasses`; a payload-carrying enum breaks that convention and
forces `isinstance` or `match` on consumers, and a nested value class for a single
field adds an unwrap to every read.
