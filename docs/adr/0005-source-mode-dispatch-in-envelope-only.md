# Source-mode dispatch lives in the envelope crate only

`SentenceSource` returns a borrowed `RawSentence<'_>` through a GAT, so it is
not object-safe, and the PRD (U3, Decision 5) answered with an
`enum Parser { OneShot, Streaming }` in every crate. Through 0.1.4 the two
typed-crate copies offered fewer constructors than `Nmea0183Parser<P>` and
`AisFragmentParser<P>` (no capacity plus options, no `with_reassembler`), so
the Python bindings carried a third and fourth private copy, and each new
wrapper option had to be mirrored by hand. From 0.2.0 only
`marlin_nmea_envelope::Parser` remains and it implements `SentenceSource`; a
caller who picks the source mode at runtime wraps it,
`AisFragmentParser::with_reassembler(Parser::streaming(), reasm)`, and gets
the wrapper's whole API. Object safety only forces an enum where the trait is
implemented, at the envelope layer; above it, composition over a concrete type
parameter sidesteps the HRTB nested-call trap and leaves nothing to mirror.
The typed crates re-export `Parser`, `OneShot` and `Streaming` so this costs
the caller no extra dependency.

Widening the three enums was rejected because it keeps the mode decision in
two places per layer. Deleting all three was rejected because choosing the
mode from configuration without generics is a stated requirement (U3) and the
Python bindings need it.
