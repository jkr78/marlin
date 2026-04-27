//! The unified sentence-source trait.

use crate::Error;

/// Push-bytes-in, pull-sentences-out abstraction over an NMEA envelope parser.
///
/// One trait covers both single-sentence (UDP-style) and streaming
/// (TCP-style) parsers. The two implementations shipped with this crate,
/// [`OneShot`](crate::OneShot) and [`Streaming`](crate::Streaming), differ
/// only in how they manage their internal buffer; both route through the
/// same parser core.
///
/// # Usage shape
///
/// ```text
/// parser.feed(&bytes);
/// while let Some(result) = parser.next_sentence() {
///     match result {
///         Ok(sentence) => { /* ... */ }
///         Err(e) => { /* log and continue; parser has recovered */ }
///     }
/// }
/// ```
///
/// This loop is identical between modes. The choice of implementation is a
/// construction-time decision, typically driven by the caller's transport
/// (datagram vs. byte stream).
///
/// # Why a GAT, and why not object-safe
///
/// The associated type `Item<'a>` is a *generic associated type* (GAT). It
/// allows `next_sentence` to return a [`RawSentence<'_>`](crate::RawSentence)
/// that borrows from the parser's internal buffer, avoiding per-sentence
/// allocation.
///
/// The trade-off is that `SentenceSource` is **not** object-safe: the
/// lifetime-parameterized associated type cannot be expressed in a fixed
/// vtable, so `Box<dyn SentenceSource>` does not compile. This is
/// intentional. Use this trait in generic code (`fn drain<P:
/// SentenceSource>(p: &mut P)`). For runtime dispatch (config-driven mode
/// selection), use the concrete [`Parser`](crate::Parser) enum instead —
/// it is zero-cost and exhaustive at compile time. See PRD §4.4 and §4.5.
///
/// # Contract
///
/// An implementor must uphold:
///
/// - `feed` never panics, regardless of the bytes supplied.
/// - `next_sentence` never panics, regardless of the state left by `feed`.
/// - After `next_sentence` returns `Some(_)`, the implementation has
///   advanced past the yielded (or failed) sentence and is ready to produce
///   the next one on the following call.
/// - After `next_sentence` returns `None`, no complete sentence is yet
///   available in the buffer; the caller should `feed` more bytes.
pub trait SentenceSource {
    /// The item produced by [`next_sentence`](Self::next_sentence). For the
    /// stock impls this is [`RawSentence<'a>`](crate::RawSentence); higher
    /// crates wrap this trait with typed-message items.
    type Item<'a>
    where
        Self: 'a;

    /// Push raw bytes into the parser.
    ///
    /// In `OneShot` mode the bytes are accumulated until one complete
    /// sentence has been parsed; in `Streaming` mode they are appended to
    /// an internal buffer and scanned lazily by `next_sentence`.
    ///
    /// This method performs no I/O and never panics. It may reallocate the
    /// internal buffer if capacity is exceeded; in `Streaming` mode the
    /// buffer has a configurable maximum size beyond which it will not grow
    /// (see [`Streaming::with_capacity`](crate::Streaming::with_capacity)).
    fn feed(&mut self, bytes: &[u8]);

    /// Attempt to pull the next complete sentence out of the parser.
    ///
    /// Returns:
    ///
    /// - `Some(Ok(item))` — a well-formed sentence.
    /// - `Some(Err(e))` — a malformed or unverifiable sentence; the parser
    ///   has advanced past it and is ready to continue.
    /// - `None` — no complete sentence is available yet; feed more bytes.
    ///
    /// The returned item borrows from the parser's internal buffer, so
    /// `next_sentence` takes `&mut self` and the item's lifetime is tied to
    /// that borrow. Only one item can be held at a time per parser.
    fn next_sentence(&mut self) -> Option<Result<Self::Item<'_>, Error>>;
}
