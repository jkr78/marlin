# marlin-nmea-0183

Typed decoders for NMEA 0183 sentences. Built on top of
[`marlin-nmea-envelope`](../marlin-nmea-envelope).

## Supported sentences

| Type | Status | Notes |
| --- | --- | --- |
| `$__GGA` | ✅ done | Global Positioning System Fix Data |
| `$__GLL` | ✅ done | Geographic Position, Latitude/Longitude (NMEA 2.3+ mode) |
| `$__HDT` | ✅ done | True Heading |
| `$__RMC` | ✅ done | Recommended Minimum Specific GNSS Data; pre-2.3 (11 fields), 2.3+ with mode (12 fields), and 4.10+ with nav status (13 fields) all accepted |
| `$__VTG` | ✅ done | Course Over Ground / Ground Speed (NMEA 2.3+ mode) |
| `$PSXN` | ✅ done | Proprietary motion sentence; 6 data slots whose meaning is install-configured via [`PsxnLayout`]. Default `rphx` (roll, pitch, heave, ignored ×3). Supports TSS sine-encoded variants. |
| `$PRDID` | ✅ done | Proprietary attitude; two dialects (`PitchRollHeading`, `RollPitchHeading`). Default dialect is `Unknown` → preserves raw fields. Select via [`DecodeOptions::with_prdid_dialect`]. |

(`__` is the 2-byte talker ID — `GP`, `IN`, `GN`, etc. The decoder
doesn't dispatch on talker; it's preserved as metadata.)

## What this crate adds over the envelope

- **Typed structs** (`GgaData`, `VtgData`, …) with decoded numeric
  fields, enum values for fix quality / mode indicators, and
  `Option<T>` for fields that may be empty ("no data available").
- **Coordinate conversion**: NMEA's `ddmm.mmmm` format → signed decimal
  degrees, with hemisphere characters folded in.
- **Strict dispatch**: unknown sentence types return
  `Nmea0183Message::Unknown(RawSentence)` so callers can decide whether
  to log, ignore, or decode further — the raw bytes are preserved.

## Extension points

Each sentence type has a **public** `decode_<type>` function
(`decode_gga`, `decode_hdt`, …). Downstream crates that need to handle
proprietary sentences not in this crate can build their own message
enum and delegate to our decoders for the standard types:

```rust
use marlin_nmea_envelope::RawSentence;
use marlin_nmea_0183::{decode_gga, decode_hdt, GgaData, HdtData};

pub enum MyMsg<'a> {
    Gga(GgaData),
    Hdt(HdtData),
    MyProprietary(/* ... */),
    Unknown(RawSentence<'a>),
}

pub fn decode(raw: &RawSentence) -> MyMsg<'_> {
    match raw.sentence_type {
        "GGA"   => decode_gga(raw).map(MyMsg::Gga).unwrap_or(MyMsg::Unknown(raw.clone())),
        "HDT"   => decode_hdt(raw).map(MyMsg::Hdt).unwrap_or(MyMsg::Unknown(raw.clone())),
        "MYPRO" => { /* downstream logic */ MyMsg::MyProprietary(...) }
        _       => MyMsg::Unknown(raw.clone()),
    }
}
```

Zero-cost: the dispatcher is just a `match`, the decoders are plain
functions, and no trait objects or runtime registries are involved.

## MSRV

1.82.

## License

Dual-licensed under MIT OR Apache-2.0.
