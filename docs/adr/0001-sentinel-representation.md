# Sentinels: not available is `None`, over-range stays a value

AIS fields reserve codes for "not available" (SOG 1023, ROT −128, altitude 4095,
position 181°/91°) and for "this value or more" (SOG 1022 = 102.2 kn on vessels and
1022 kn on SAR aircraft, altitude 4094 m, dimensions 511 m / 63 m). We map only the
not-available codes to `None` and pass over-range codes through as the value they
state. The decoded field never carries the raw code; the codes themselves
are public constants in `marlin_ais::sentinel`. Rate of turn is the one exception:
±127 means "turning right/left at more than 5° per 30 s (no TI available)", a
status, so it becomes a `RateOfTurn` variant. The ITU formula would turn it into a
fabricated ±720 °/min.

## Considered options

A consumer asked for a three-way reading (value / not available / invalid) on every
field. It would change every numeric field type in every message type and revise
this decision, so it gets its own design instead of landing piecemeal on the new types.
