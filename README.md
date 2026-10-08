# marlin

A Rust library suite for parsing NMEA 0183 and AIS (AIVDM/AIVDO) messages.

**Sans-I/O.** No sockets, no async runtime, no file handles — bytes in via
`feed`, parsed values out via `next_sentence` / `next_message`. The same
parser serves UDP datagrams, TCP streams, serial ports, file replay, and
unit-test byte slices.

## Crates

| Crate | Purpose | Status |
| --- | --- | --- |
| [`marlin-field`](./crates/marlin-field) | `FieldState<T>`: the decoded-field state type shared by the decoders | feature-complete |
| [`marlin-nmea-envelope`](./crates/marlin-nmea-envelope) | NMEA 0183 framing, checksum, TAG block recognition | feature-complete |
| [`marlin-nmea-0183`](./crates/marlin-nmea-0183) | Typed NMEA sentence decoders (GGA, GLL, HDG, HDT, RMC, TLL, TTM, VTG, PSXN, PRDID) | feature-complete |
| [`marlin-ais`](./crates/marlin-ais) | Typed AIS message decoders + multi-sentence reassembly | feature-complete |
| [`marlin-klv`](./crates/marlin-klv) | MISB ST 0601 (KLV) encoder/decoder | feature-complete |

`marlin-nmea-0183` and `marlin-ais` are siblings; both depend on
`marlin-nmea-envelope` but not on each other. `marlin-klv` is a
standalone crate with no dependency on `marlin-nmea-envelope` — KLV is
not NMEA-framed. `marlin-field` is the leaf under all three decoders,
which re-export its `FieldState<T>` from 0.3.0; `marlin-nmea-envelope`
does not depend on it.

## Supported messages

### NMEA 0183 (non-AIS)

Typed decoders for these sentence types, regardless of talker prefix
(`GP` for GPS-only, `GN` for multi-GNSS fixes, `IN`, `HE`, etc.):

- **GGA** — GPS fix data: time, position, fix quality, satellites,
  HDOP, altitude, geoid separation, DGPS metadata
- **GLL** — geographic position with UTC time, validity status, and
  optional 2.3+ mode indicator
- **HDG** — magnetic heading with deviation and variation as signed
  degrees (`E` positive, `W` negative)
- **HDT** — true heading
- **RMC** — recommended minimum: UTC time + date + position + speed
  + course + magnetic variation, with validity status; pre-2.3, 2.3+
  with mode indicator, and 4.10+ with nav status all decode through
  one path
- **TLL** — radar/ARPA target position: target number, latitude,
  longitude, name, UTC time, status
- **TTM** — radar/ARPA tracked target: distance, bearing, speed,
  course, CPA, TCPA, name, status, acquisition type
- **VTG** — course over ground and speed: course (true and magnetic),
  speed (knots and km/h), mode indicator; both pre-2.3 and 2.3+ forms
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
- **Type 9** — SAR aircraft position report (altitude, SOG in whole
  knots, lat/lon, COG, altitude sensor)
- **Type 18** — Class B position report
- **Type 19** — Class B extended position report (position + name +
  ship type + dimensions)
- **Type 21** — aid-to-navigation report (AtoN type, name with its
  optional extension, lat/lon, dimensions, EPFD, off-position and
  virtual flags)
- **Type 24 part A** — Class B static data: vessel name
- **Type 24 part B** — Class B static data: callsign, ship type,
  vendor ID, dimensions (or mother-ship MMSI for an auxiliary craft), EPFD

Multi-fragment messages reassemble across `!AIVDM` line pairs. Other
types surface as `Other` carrying the raw bit buffer for downstream
decoding.

### MISB ST 0601 (KLV)

Sans-I/O encoder and decoder for the UAS Datalink Local Set
(`marlin-klv`). 20 scaled tags:

- **Platform** — heading, pitch, roll, true airspeed
- **Sensor** — latitude, longitude, true altitude, horizontal field of
  view, vertical field of view, relative azimuth, relative elevation,
  relative roll
- **Slant range**
- **Target width**
- **Frame center** — latitude, longitude, elevation
- **Target location** — latitude, longitude, elevation

Framing tags: Tag 2 (precision timestamp, mandatory), Tag 65 (LS
version), Tag 1 (16-bit BCC checksum, mandatory, last). Unknown tags
round-trip verbatim; a known tag with the wrong wire length falls back
to the same unknown-tag path instead of erroring.

`marlin-klv` is the suite's first **encoder** — every other crate here
is decode-only. It's also a standalone leaf: no dependency on
`marlin-nmea-envelope`, since KLV is not NMEA-framed.

## Python bindings

`marlin-py` ships a Rust-backed Python interface to all four crates,
plus async iterator helpers and frozen dataclass mirrors for
serialization. See [`bindings/python/`](./bindings/python) for install
steps, the usage guide ([`GUIDE.md`](./bindings/python/GUIDE.md)), and
runnable examples.

## MSRV

Rust 1.82.

## License

Dual-licensed under MIT OR Apache-2.0.
