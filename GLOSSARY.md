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

### Field state

**Field state**:
The decoded outcome of one wire field: a value, not available, sender error,
invalid, or over-range.
_Avoid_: reading, measurement, tri-state, three-way, slot

**Value**:
The field state holding a value the wire gave a meaning to and the decoder
accepted.
_Avoid_: valid, ok, present

**Sentinel**:
A reserved wire value that carries a status instead of a measurement.
_Avoid_: magic number, placeholder

**Not available**:
The field state meaning the sender supplied no value for the field, by
sentinel code, empty 0183 field or omitted optional KLV tag.
_Avoid_: unknown, invalid, missing, null, absent

**Sender error**:
The field state for a sentinel meaning the sender knows it has no usable
value: a sensor fault or an unrepresentable reading. An out-of-range code
with no known bound is a sender error, not over-range.
_Avoid_: error indicator (MISB's wording for one case), flagged, fault

**Invalid**:
The field state for a wire value the decoder could not give a meaning,
whether unparsable text or a number the spec leaves undefined.
_Avoid_: rejected, malformed (reserved for sentence framing), unparsable,
corrupt

**Over-range**:
The field state for a sentinel meaning the true value is at or beyond a
known bound the field carries, so the decoded number is that bound (speed
102.2 kn or more, altitude 4094 m or more). Carries information; not
available does not.
_Avoid_: saturated (past-end bit reads), floor (minimum payload length),
clamped, capped

**Raw code**:
The undecoded wire integer a sentinel state retains.
_Avoid_: wire value, magic number, raw (alone, in prose; fine as an
accessor name)

**Wire field**:
One encoded item as the wire carries it: an AIS bit span, a comma-delimited
0183 field, a KLV tag. The unit at which a field state is assigned; a struct
that groups several wire fields has no state of its own, unless the fields
form a paired field.
_Avoid_: slot, element, sub-field (for a wire field inside a grouping)

**Paired field**:
Two adjacent 0183 wire fields that together encode one quantity, a magnitude
and its sign letter, such as latitude with its `N`/`S` hemisphere or magnetic
variation with its `E`/`W` direction. The pair receives one field state; a
half-filled pair is invalid, not not available.
_Avoid_: composite field, coordinate pair (in prose), hemisphere field (for
the pair)

**Plain field**:
A wire field the sender cannot put in any state but value, such as a one-bit
flag or an opaque bit block, so it is decoded without a field state.
_Avoid_: raw field, bare field, flag (as a category)

**Status field**:
A field whose value qualifies other fields in the same message rather than
measuring anything, such as the 0183 `A`/`V` data status or GGA fix quality.
A void status does not change the field state of the fields it qualifies.
_Avoid_: flag field, validity field

**Status-carrying field**:
A measurement field whose code space also carries defined statuses, such as
the AIS timestamp (61–63: manual input, dead reckoning, inoperative) or rate
of turn (±127: turning with no indicator). A status is a value of the field,
not a field state; only the not-available code is a field state.
_Avoid_: mixed field, sentinel field

**Message failure**:
The decode outcome where no message is produced: the input could not be
framed, its checksum did not verify, its layout differs from what the
decoder expects, or a mandatory field is absent. A field's value never
causes one.
_Avoid_: decode error (the Rust type), rejection, drop

**Layout**:
The field count or bit length a decoder expects of a message. Fewer than
the floor fails the message; more is ignored.
_Avoid_: schema, format, shape (reserved for the Rust type of a field)

**Floor**:
The shortest payload length at which a message type's layout is still
identifiable, taken from traffic seen in the wild rather than the standard's
table. Fields past the transmitted end are not available.
_Avoid_: minimum length, table length, nominal length

**Turn indicator (TI)**:
The onboard rate-of-turn sensor behind the ROT field. Codes ±127 mean "turning
right/left at more than 5° per 30 s (no TI available)": a status, not a rate.
_Avoid_: ROT sensor, gyro

**Timestamp**:
The UTC second of a report's position fix. A status-carrying field: 60 is
not available, 61–63 are positioning-system statuses (manual input, dead
reckoning, inoperative).
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
