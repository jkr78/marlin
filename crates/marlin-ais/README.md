# marlin-ais

Sans-I/O typed decoders for AIS (AIVDM/AIVDO) messages. Built on top of
[`marlin-nmea-envelope`](../marlin-nmea-envelope).

## Supported message types

| Type | Struct | Fields |
| --- | --- | --- |
| 1, 2, 3 | `PositionReportA` | Class A position report: navigation status, rate of turn, SOG, position, COG, true heading, maneuver indicator |
| 5 | `StaticAndVoyageA` | Class A static and voyage data: IMO number, call sign, name, ship type, dimensions, EPFD, ETA, draught, destination |
| 9 | `SarAircraftPositionReport` | SAR aircraft position report: altitude, SOG in whole knots, position, COG, altitude sensor |
| 18 | `PositionReportB` | Class B position report: SOG, position, COG, true heading, Class B capability flags |
| 19 | `ExtendedPositionReportB` | Class B extended position report: the Type 18 position fields plus name, ship type, dimensions, EPFD |
| 21 | `AidToNavigationReport` | Aid-to-navigation report: AtoN type, name with its optional extension, position, dimensions, EPFD, off-position and virtual flags |
| 24 Part A | `StaticDataB24A` | Class B static data: vessel name |
| 24 Part B | `StaticDataB24B` | Class B static data: ship type, vendor ID, call sign, EPFD, and dimensions or the mother-ship MMSI of an auxiliary craft |

Every other message type decodes to `AisMessageBody::Other`, which
keeps the message type and the raw bit buffer so you can run your own
decoder with `BitReader`.

Field layouts follow ITU-R M.1371-5 Annex 8. A field whose wire code
means "not available" decodes to `None`. A code that means "this value
or higher" stays a value. The codes are public constants in
`marlin_ais::sentinel`.

`Parser` takes bytes and yields `AisMessage` values. It
reassembles multi-sentence messages keyed on `(channel,
sequential_id)`, evicts the oldest partial once 16 are open, and can
expire partials by age when you supply a clock.

## Quickstart

```rust
use marlin_ais::{AisMessageBody, Parser};

let mut parser = Parser::streaming();
parser.feed(b"!AIVDM,1,1,,A,13aGmP0P00PD;88MD5MTDww@2<0L,0*23\r\n");

while let Some(result) = parser.next_message() {
    match result {
        Ok(msg) => {
            if let AisMessageBody::Type1(report) = msg.body {
                println!("{} at {:?}, {:?}", report.mmsi, report.latitude_deg, report.longitude_deg);
            }
        }
        Err(err) => eprintln!("skipped: {err}"),
    }
}
```

`Parser::one_shot()` is the datagram variant. It takes one sentence per
`feed` and does not need a trailing `\r\n`.

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
  ↓ ASCII-armored (6 bits per character, IEC 61162-1)
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
