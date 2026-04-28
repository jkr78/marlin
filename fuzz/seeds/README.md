# Fuzz regression seeds

This directory holds a small set of libfuzzer-discovered inputs that
must continue to parse without panic on every release. It is the
regression corpus called for in PRD §F3.

## What lives here

Five inputs per target, picked as the smallest libfuzzer-minimized
items in the working corpus at the time of curation. Smallest tends to
correlate with "fundamental edge case" — single-byte sentence-start
markers, two-byte truncations, control characters that exercise the
state machine without any payload to distract from it.

| Target | Examples |
| --- | --- |
| `envelope` | `!` (truncated AIS start), `$` (truncated NMEA start), `$*` (start + checksum, no body), `!!` (double start) |
| `ais_armor` | `\0`, `\003` (control bytes through the 6-bit alphabet) |
| `ais_bit_reader` | 2-byte payloads exercising past-end saturation |
| `ais_parser` | Truncations of the full envelope→armor→decode pipeline |

Filenames are SHA1 hashes of the input content. The hashes are stable
across hosts; renaming would lose the link back to the libfuzzer entry
that originally discovered each case.

## How they get used

`just fuzz <target>` automatically copies the seeds for that target
into `fuzz/corpus/<target>/` before running. The copy is `cp -n`
(no-clobber), so a later libfuzzer run that produces a better
minimization of an existing seed does not silently overwrite the
curated version.

After `just fuzz-clean` wipes `fuzz/corpus/`, the next `just fuzz` run
re-seeds from this directory. The regression corpus survives clean.

## Curating new seeds

When a fuzz run discovers a previously unknown panic and you fix it,
add the minimized reproducer here under the appropriate target. Keep
the SHA1 filename. The size budget for this directory is small — if
the suite grows past ~20 inputs per target, prune the oldest items
that the current code already covers via unit tests.
