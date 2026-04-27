# Envelope fixtures

Golden-file test inputs for `marlin-nmea-envelope`. Each file is a
byte-exact NMEA 0183 payload; corresponding assertions live in
`../golden.rs`.

Per PRD §G3, v1 envelope fixtures may be synthetic. Higher-layer crates
(`marlin-nmea-0183`, `marlin-ais`) will use real captures when those are
implemented.

## Catalogue

| File | Purpose | Source |
| --- | --- | --- |
| `01_gga_basic.nmea` | Classic `$GPGGA` with CRLF terminator and 14 fields (two trailing empty) | NMEA 0183 specification example (widely cited; checksum `*47`) |
| `02_hdt_no_terminator.nmea` | `$INHDT` with **no** terminator — the UDP-datagram case | Synthetic |
| `03_aivdm_encapsulation.nmea` | `!AIVDM` encapsulation sentence with CRLF | Synthetic (payload follows ITU-R M.1371 armor) |
| `04_rmc_lowercase_checksum.nmea` | `$GPRMC` with the checksum rendered in **lowercase** hex (PRD §E2) | Synthetic |
| `05_stream_mixed_terminators.nmea` | Three back-to-back sentences terminated by CRLF / LF / CR respectively | Synthetic |
| `06_tagged_sentence.nmea` | `\c:1577836800*XX\$GPGGA...` — NMEA 4.10 TAG block with valid checksum | Synthetic (timestamp tag format per IEC 61162-450) |
| `07_streaming_with_garbage.nmea` | Garbage bytes → `$GPGGA` → more garbage → `$INHDT` (streaming-recovery test) | Synthetic |

> **Note on fixture 07:** the "garbage" bytes deliberately avoid `$` and `!`,
> since both are valid sentence-start delimiters (`$` for data, `!` for
> encapsulation). A `!` embedded in otherwise-garbage bytes would cause
> the scanner to latch onto it and swallow data up to the next `*` — this
> is correct parser behavior per PRD §E1, just rarely what tests want.

## Checksum verification

Every fixture's declared checksum (the two hex digits after `*`) equals
the XOR of the body bytes between the start delimiter and `*`, exclusive
of both. The golden tests re-verify this for every fixture — if any byte
in a fixture is accidentally changed, the corresponding test fails with
`ChecksumMismatch`.

## Regenerating

The Python snippet under `scripts/` (or inline in the commit that
introduced this directory) computes checksums and writes these files.
Hand-editing is discouraged — edit the generator and re-run.
