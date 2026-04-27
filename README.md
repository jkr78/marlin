# marlin

A Rust library suite for parsing NMEA 0183 and AIS (AIVDM/AIVDO) messages.

**Sans-I/O.** No sockets, no async runtime, no file handles — bytes in via
`feed`, parsed values out via `next_sentence` / `next_message`. The same
parser serves UDP datagrams, TCP streams, serial ports, file replay, and
unit-test byte slices.

## Crates

| Crate | Purpose | Status |
| --- | --- | --- |
| [`marlin-nmea-envelope`](./crates/marlin-nmea-envelope) | NMEA 0183 framing, checksum, TAG block recognition | feature-complete (v0.1) |
| [`marlin-nmea-0183`](./crates/marlin-nmea-0183) | Typed NMEA sentence decoders (GGA, VTG, HDT, PSXN, PRDID) | feature-complete (v0.1) |
| [`marlin-ais`](./crates/marlin-ais) | Typed AIS message decoders + multi-sentence reassembly | feature-complete (v0.1) |

`marlin-nmea-0183` and `marlin-ais` are siblings; both depend on
`marlin-nmea-envelope` but not on each other.

## Supported messages

### NMEA 0183 (non-AIS)

Typed decoders for these sentence types, regardless of talker prefix
(`GP` for GPS-only, `GN` for multi-GNSS fixes, `IN`, `HE`, etc.):

- **GGA** — GPS fix data: time, position, fix quality, satellites,
  HDOP, altitude, geoid separation, DGPS metadata
- **VTG** — course over ground and speed: course (true and magnetic),
  speed (knots and km/h), mode indicator; both pre-2.3 and 2.3+ forms
- **HDT** — true heading
- **PSXN** — Seapath / Kongsberg attitude with install-configurable
  6-slot layout (roll, pitch, heave, sine-encoded variants)
- **PRDID** — TSS / Teledyne motion in either pitch-roll-heading or
  roll-pitch-heading dialect

Anything else surfaces as `Unknown` carrying the framed `RawSentence`,
still parsed at the envelope layer (talker, type, checksum, TAG block)
so you can route it to your own decoder.

### AIS (AIVDM/AIVDO)

Typed decoders for these ITU-R M.1371 message types:

- **Type 1, 2, 3** — Class A position reports (lat/lon, COG, SOG,
  heading, navigation status, maneuver indicator, ROT)
- **Type 5** — Class A static and voyage data (name, IMO, callsign,
  ship type, dimensions, ETA, draught, destination)
- **Type 18** — Class B position report
- **Type 19** — Class B extended position report (position + name +
  ship type + dimensions)
- **Type 24 part A** — Class B static data: vessel name
- **Type 24 part B** — Class B static data: callsign, ship type,
  vendor ID, dimensions

Multi-fragment messages reassemble across `!AIVDM` line pairs. Other
types surface as `Other` carrying the raw bit buffer for downstream
decoding.

## Python bindings

`marlin-py` ships a Rust-backed Python interface to all three crates,
plus async iterator helpers and frozen dataclass mirrors for
serialization. See [`bindings/python/`](./bindings/python) for install
steps, the usage guide ([`GUIDE.md`](./bindings/python/GUIDE.md)), and
runnable examples.

## MSRV

Rust 1.82.

## License

Dual-licensed under MIT OR Apache-2.0.
