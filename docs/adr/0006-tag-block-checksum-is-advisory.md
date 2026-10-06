# TAG block checksum is advisory

An NMEA 4.10 TAG block carries its own `*hh` checksum. The envelope does
not enforce it: a sentence whose TAG block checksum mismatches is accepted,
and the mismatch is reported only as a `tracing` debug event when that
feature is enabled; with the feature off the TAG checksum is not computed
at all. The sentence's own checksum is the authoritative integrity check;
TAG block encoders in the wild are sometimes buggy, and discarding
otherwise valid navigation data over a metadata checksum would lose real
positions. Rejecting on mismatch was the alternative and was turned down
for that reason.

`RawSentence::tag_block` carries the TAG content without its `*hh`, so a
caller cannot re-check the checksum from the parsed sentence today.
Exposing the TAG block checksum status to strict callers is open work,
carded in TODO.md.
