# Error types carry the crate prefix

Until 0.3.0 the workspace named its error enums three ways: bare `Error`
in the standard library's style (`marlin_nmea_envelope::Error`,
`marlin_klv::Error`), an operation prefix (`marlin_nmea_0183::DecodeError`,
`marlin_klv::EncodeError`) and a crate prefix (`AisError`,
`Nmea0183Error`). From 0.3.0 every public error type a crate's entry points
return starts with `<Crate>`, the prefix the crate's other flagship types
already use (`Ais` as in `AisMessage`, `Nmea0183` as in
`Nmea0183Parser`, `Envelope`, `Klv`). The unqualified `<Crate>Error` is
reserved for the umbrella error the crate's parser returns; when a crate
exports more than one error type, each narrower one carries its operation
word: `KlvDecodeError` and `KlvEncodeError`, `Nmea0183DecodeError` beside
`Nmea0183Error`. A value type's `FromStr` error is not a crate error and
keeps the standard library's verb-object order (`ParseUtcTimeError`,
`ParsePsxnLayoutError`).

The four crates are consumed together: an AIS client also imports the
envelope crate and usually the 0183 one, so a bare `Error` in each forces an
alias at every import, and `DecodeError` in three crates would collide the
same way. The crate prefix is unique across the workspace, matches the
Python exception names one for one (`EnvelopeError`, `AisError`,
`KlvEncodeError`), and its stutter (`marlin_ais::AisError`) is the one the
crates accepted for every flagship type. Two Python names stay as they
are: the bindings raise `KlvDecodeError` as `KlvError`, the base class of
`KlvEncodeError` in a hierarchy a Rust enum does not have, and
`Nmea0183DecodeError` as `DecodeError`, which `marlin.nmea` scopes already.

Consequences: `marlin_nmea_envelope::Error` becomes `EnvelopeError`,
`marlin_klv::Error` becomes `KlvDecodeError`, the `EncodeError` that
ADR-0008 names becomes `KlvEncodeError`, `marlin_nmea_0183::DecodeError`
becomes `Nmea0183DecodeError` and `PsxnLayoutParseError` becomes
`ParsePsxnLayoutError`; `AisError` and `Nmea0183Error` are unchanged.
