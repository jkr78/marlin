# marlin-klv

Sans-I/O MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder.

Part of the [marlin](../../README.md) suite. `#![no_std]` + `alloc`, no I/O:
bytes in via `decode`, a typed `St0601` out; a typed `St0601` in via `encode`,
framed KLV bytes out.

See the crate docs on [docs.rs](https://docs.rs/marlin-klv).
