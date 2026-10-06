# The reassembly clock is reassembler state

Sans-I/O crates usually pass time as an argument to every call that needs it
(`smoltcp` `poll(timestamp, ..)`, `quinn` `handle_timeout(now)`), and `marlin-ais`
0.1 did the same with `feed_fragment_at` and `next_message_at`. From 0.2.0
`AisReassembler::tick(now_ms)` stores the time as well as evicting, and
`feed_fragment` stamps partials with the last ticked time, so there is one
`feed` / `next_message` shape shared with every other parser in the workspace
and one `next_message` loop. The argument style cost a second copy of that
loop: the loop pulls from the HRTB-bound `SentenceSource`, and rustc rejects
one such bounded method delegating to another, so `next_message` and
`next_message_at` could not share a body. The convention's purpose, that the
library never reads a clock behind the caller's back, still holds: `tick` is
the only way time enters.

A partial opened before the first `tick` has no stamp and is retired only by
the slot cap; the first tick does not stamp it. One rule, no special case in
`tick`, and it only affects partials fed before a caller who uses the timeout
has started ticking.
