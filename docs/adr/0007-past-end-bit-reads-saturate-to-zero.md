# Past-end bit reads saturate to zero

A `BitReader` read past the end of its buffer returns zero bits rather than
an error. Each typed decoder first rejects a payload shorter than its fixed
minimum with `PayloadTooShort`; past that check, a payload that ends inside
a variable-length tail decodes with the remaining fields at their zero
values instead of failing the whole message, and no input-driven path can
panic. The alternative, a `Result` on every read, was turned down: it would
thread error handling through every field decoder for a condition the
minimum-length check already catches where it matters, and the zero reads
are deterministic and fuzz-verified. A caller who needs to know whether a
field was present checks `remaining()` before reading.
