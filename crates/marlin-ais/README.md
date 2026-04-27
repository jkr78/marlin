# marlin-ais

Sans-I/O typed decoders for AIS (AIVDM/AIVDO) messages. Built on top of
[`marlin-nmea-envelope`](../marlin-nmea-envelope).

## Status

**Under construction.** The building blocks (armor decoder, bit
reader, AIVDM wrapper parser) are implemented. Typed message decoders
(position reports, static/voyage data) and multi-sentence reassembly
are coming next.

## What AIS is

AIS (Automatic Identification System) is a maritime collision-
avoidance protocol defined by ITU-R M.1371. Ships, base stations, and
AtoNs broadcast binary messages over VHF; receivers translate them to
`!AIVDM` / `!AIVDO` NMEA-0183-framed sentences for consumption by
backend systems. This crate decodes those sentences into typed Rust
structs.

Wire stack:

```text
binary AIS message (ITU-R M.1371)
  ↓ ASCII-armored (6 bits per character)
!AIVDM/!AIVDO sentence (NMEA 0183)
  ↓ marlin-nmea-envelope
RawSentence
  ↓ marlin-ais (this crate)
typed AisMessage
```

## MSRV

1.82.

## License

Dual-licensed under MIT OR Apache-2.0.
