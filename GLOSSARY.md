# marlin

Sans-I/O decoders for marine wire formats: NMEA 0183 sentences, AIS
(AIVDM/AIVDO) messages and MISB ST 0601 KLV. One context; the terms
below are the ones the code, docs and tests use.

## Language

### Framing

**Armor**:
The 6-bit ASCII packing of an AIS payload inside a sentence.
_Avoid_: sixbit, 6-bit encoding

**Sentence**:
One NMEA 0183 line, from `$` or `!` to the checksum.
_Avoid_: line, record, packet

**TAG block**:
The NMEA 4.10 metadata prefix carried before a sentence, between
backslashes and with its own checksum.
_Avoid_: tag prefix, header, TAG header

**Talker**:
The two-letter source identifier that opens a standard sentence; a
proprietary `$P…` sentence has none.
_Avoid_: talker ID, source id, sender

**Fragment**:
One sentence carrying part of a multi-sentence AIS message.
_Avoid_: part (reserved for Type 24 Part A/B), chunk, segment

**Reassembly**:
Joining the fragments of one AIS message, keyed by channel and sequential id.
_Avoid_: defragmentation, stitching

**Partial**:
A multi-fragment AIS message whose fragments have not all arrived.
_Avoid_: in-flight message, pending message, incomplete message

**Eviction**:
Discarding a partial before it completes, because it aged out or the
reassembler's slots were full.
_Avoid_: expiry (one cause only), drop

**Source mode**:
Whether a parser takes one complete sentence per feed or buffers a byte
stream; chosen when the parser is built. The modes are one-shot and streaming.
_Avoid_: parser mode, runtime mode, dispatch mode

**One-shot**:
The source mode for framed transports such as UDP datagrams: each feed
carries exactly one sentence.
_Avoid_: single-shot, datagram mode, oneshot

**Streaming**:
The source mode for byte streams such as TCP or serial: feeds carry any
number of whole or partial sentences and the parser finds the boundaries.
_Avoid_: buffered mode, stream mode

**Own ship**:
The receiving station's own AIS transmissions, framed as `!AIVDO`.
_Avoid_: ownship, self, local vessel

### Field values

**Sentinel**:
A reserved wire value that carries a status instead of a measurement.
_Avoid_: magic number, placeholder

**Not available**:
The sentinel meaning the sender has no value for the field.
_Avoid_: unknown, invalid, missing, null

**Over-range**:
A sentinel meaning the true value is at or above the field's maximum, so the
decoded number is a lower bound (speed 102.2 kn or more, altitude 4094 m or
more). Carries information; not available does not.
_Avoid_: saturated (past-end bit reads), floor (minimum payload length),
clamped, capped

**Turn indicator (TI)**:
The onboard rate-of-turn sensor behind the ROT field. Codes ±127 mean "turning
right/left at more than 5° per 30 s (no TI available)": a status, not a rate.
_Avoid_: ROT sensor, gyro

**Timestamp**:
The UTC second of a report's position fix; codes 60–63 are sentinels (not
available, manual input, dead reckoning, inoperative).
_Avoid_: time stamp, UTC second

**Radio status**:
The communication-state tail of a position report: the SOTDMA/ITDMA
slot-allocation bits, with the selector bit where the message has one.
_Avoid_: communication state (ITU's wording), sync state

### Stations and reports

**Class A / Class B**:
Shipborne AIS equipment classes. Class A reports with Types 1–3 and 5 and answers
a Type 24 interrogation with Part B; Class B reports with Types 18, 19 and 24.

**Part A / Part B**:
The two halves of the Type 24 static data report, sent as separate messages,
often minutes apart.
_Avoid_: fragment, 24A/24B in prose (fine in identifiers)

**Auxiliary craft**:
A craft tied to a mother ship, MMSI `98MIDxxxx`. Its Part B extent carries
the mother ship's MMSI instead of dimensions.
_Avoid_: tender, daughter craft

**Mother ship**:
The parent vessel of an auxiliary craft.
_Avoid_: mothership in prose (fine in identifiers), parent vessel

**Extent**:
The 30-bit dimension field of Types 5, 19, 21 and 24 Part B: distances from the
reported position to bow, stern, port and starboard. In an auxiliary craft's Part B
it holds the mother ship's MMSI instead.
_Avoid_: size, footprint

**SAR aircraft**:
A search-and-rescue aircraft reporting via Type 9, MMSI `111MIDxxx`. Reports
altitude; has no heading, rate of turn or navigational status.
_Avoid_: aircraft (too broad), helicopter

**Aid to Navigation (AtoN)**:
A buoy, beacon, light or other mark reporting via Type 21, MMSI `99MIDxxxx`.
ITU-R M.1371-5 Table 74 groups AtoN types 5–19 as fixed and 20–31 as
floating; types 0–4 (not specified, reference point, RACON, offshore
structure, emergency wreck marking buoy) carry no such label.
_Avoid_: buoy (one kind of AtoN), navaid, beacon (one kind of AtoN)

**Virtual AtoN**:
An AtoN with no physical object: a position broadcast on its behalf. Has
zero dimensions.
_Avoid_: synthetic AtoN, phantom

**Off-position**:
A floating AtoN reporting that it has drifted from its charted position.
Says nothing for a fixed AtoN.
_Avoid_: adrift, displaced

**AtoN status**:
The 8-bit status byte of an AtoN report; its layout is regional and not part of
ITU-R M.1371.
_Avoid_: regional, regional reserved

**Name extension**:
The optional tail of an AtoN name beyond its first 20 characters. One name
on the wire in two pieces, never a second field.
_Avoid_: extended name, name part 2

### Python bindings

**Binding class**:
The Python class a Rust type is exposed as, such as `marlin.ais.PositionReportA`.
One per message struct, value type, coded enum, parser, or tool.
_Avoid_: mirror, wrapper class, Py class

**Dataclass mirror**:
The frozen dataclass in `marlin.dataclasses` that holds the same fields as a
binding class, with enum fields as plain integers.
_Avoid_: mirror (alone), dataclass copy
