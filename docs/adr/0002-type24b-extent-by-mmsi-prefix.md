# Type 24 Part B extent follows the MMSI prefix

marlin decodes wire fields without interpreting them. The 30-bit extent field of
Type 24 Part B is the single exception: for an MMSI of the form `98MIDxxxx`
(auxiliary craft) the bits hold the mother ship's MMSI, otherwise dimensions
A/B/C/D. `Type24BExtent` branches on that prefix because returning dimensions
regardless gave wrong numbers for every auxiliary craft, and the caller could not
repair them without re-packing the bits.

The rule comes from the NavCen MMSI-format convention as gpsd implements it;
ITU-R M.1371-5 Table 79 does not state it. If a real transmitter ever contradicts
the convention, nothing is lost: both arms keep the 30 bits re-packable
(`MothershipMmsi(u32)` holds them verbatim; `Dimensions` maps only 0 to `None`),
and the sentence can be re-read at bit level with `parse_aivdm_wrapper`,
`armor::decode` and `BitReader`, the crate's documented escape hatch. A Type 24
Part B payload never reaches `AisMessageBody::Other`; only reserved part codes 2
and 3 do.
