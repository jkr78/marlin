//! Python exception hierarchy for marlin errors.
//!
//! The exceptions are created at the `_core` top-level module by
//! `create_exception!`, then re-bound in each submodule's `__init__.py`
//! (the pure-Python layer) so that `from marlin.ais import AisError`
//! works as expected.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use marlin_ais::AisError as RustAisError;
use marlin_klv::EncodeError as RustKlvEncodeError;
use marlin_klv::Error as RustKlvError;
use marlin_nmea_0183::DecodeError as RustDecodeError;
use marlin_nmea_envelope::Error as RustEnvelopeError;

create_exception!(_core, MarlinError, PyException);
create_exception!(_core, EnvelopeError, MarlinError);
create_exception!(
    _core,
    DecodeError,
    MarlinError,
    "A typed NMEA 0183 decode failed: the sentence has fewer fields than its decoder's floor. The one reason a typed decode fails; a field's value never does, it decodes to a FieldState."
);
create_exception!(_core, AisError, MarlinError);
create_exception!(_core, ReassemblyError, AisError);
create_exception!(
    _core,
    KlvError,
    MarlinError,
    "A KLV decode failed for a structural reason: framing, the checksum, or the mandatory Tag 2 timestamp absent or malformed. A field's value never fails the set, it decodes to a FieldState. `variant` names the reason."
);
create_exception!(
    _core,
    KlvEncodeError,
    KlvError,
    "A KLV encode failed: `tag` is in a field state the wire cannot carry (`variant` \"unencodable\", `kind` the state's kind), or holds a value outside the tag's range or NaN (`variant` \"out_of_range\", `kind` None)."
);

/// Register all exceptions on the given module. Called from `lib.rs`.
pub(crate) fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("MarlinError", py.get_type::<MarlinError>())?;
    m.add("EnvelopeError", py.get_type::<EnvelopeError>())?;
    m.add("DecodeError", py.get_type::<DecodeError>())?;
    m.add("AisError", py.get_type::<AisError>())?;
    m.add("ReassemblyError", py.get_type::<ReassemblyError>())?;
    m.add("KlvError", py.get_type::<KlvError>())?;
    m.add("KlvEncodeError", py.get_type::<KlvEncodeError>())?;
    Ok(())
}

/// Convert a marlin-nmea-envelope `Error` into an `EnvelopeError`
/// with a `variant` attribute set to a stable snake-case tag.
pub(crate) fn envelope_err(py: Python<'_>, err: RustEnvelopeError) -> PyErr {
    let (variant, msg) = envelope_variant(&err);
    let pyerr = EnvelopeError::new_err(msg);
    // `setattr` on a freshly-constructed PyException with a static-str key
    // and value cannot fail in practice (no Python-level __setattr__ hook,
    // no allocation path that can fail meaningfully). Discarding the
    // Result keeps the converter infallible for callers.
    let _ = pyerr.value(py).setattr("variant", variant);
    pyerr
}

fn envelope_variant(err: &RustEnvelopeError) -> (&'static str, String) {
    match err {
        RustEnvelopeError::MissingStartDelimiter => ("missing_start_delimiter", err.to_string()),
        RustEnvelopeError::MissingChecksumDelimiter => {
            ("missing_checksum_delimiter", err.to_string())
        }
        RustEnvelopeError::InvalidChecksumDigits => ("invalid_checksum_digits", err.to_string()),
        RustEnvelopeError::ChecksumMismatch { .. } => ("checksum_mismatch", err.to_string()),
        RustEnvelopeError::InvalidUtf8InSentenceType => {
            ("invalid_utf8_in_sentence_type", err.to_string())
        }
        RustEnvelopeError::TalkerTooShort => ("talker_too_short", err.to_string()),
        RustEnvelopeError::MalformedTagBlock => ("malformed_tag_block", err.to_string()),
        RustEnvelopeError::Truncated => ("truncated", err.to_string()),
        RustEnvelopeError::BufferOverflow => ("buffer_overflow", err.to_string()),
        _ => ("other", err.to_string()),
    }
}

/// Convert a marlin-nmea-0183 `DecodeError` into a `DecodeError` (Py).
pub(crate) fn decode_err(err: RustDecodeError) -> PyErr {
    DecodeError::new_err(err.to_string())
}

/// Convert a marlin-ais `AisError` into the appropriate Py exception.
/// Reassembly-related variants map to `ReassemblyError`; everything else
/// to `AisError`. Envelope-wrapped failures preserve `__cause__`.
pub(crate) fn ais_err(py: Python<'_>, err: RustAisError) -> PyErr {
    use marlin_ais::AisError::{Envelope, ReassemblyOutOfOrder, ReassemblyTimeout};
    let msg = err.to_string();
    match err {
        ReassemblyTimeout | ReassemblyOutOfOrder => ReassemblyError::new_err(msg),
        Envelope(inner) => {
            let pyerr = AisError::new_err(msg);
            let cause = envelope_err(py, inner);
            // Same setattr-infallibility rationale as envelope_err above.
            let _ = pyerr.value(py).setattr("__cause__", cause.value(py));
            pyerr
        }
        _ => AisError::new_err(msg),
    }
}

/// Convert a marlin-klv `Error` into a `KlvError` with a `variant`
/// attribute set to a stable snake-case tag.
pub(crate) fn klv_err(py: Python<'_>, err: RustKlvError) -> PyErr {
    let variant = match err {
        RustKlvError::Truncated { .. } => "truncated",
        RustKlvError::LengthOverflow => "length_overflow",
        RustKlvError::BadChecksum { .. } => "bad_checksum",
        RustKlvError::MissingChecksum => "missing_checksum",
        RustKlvError::BadTimestamp => "bad_timestamp",
        RustKlvError::BadKey => "bad_key",
        _ => "other",
    };
    let pyerr = KlvError::new_err(err.to_string());
    // Same setattr-infallibility rationale as envelope_err above.
    let _ = pyerr.value(py).setattr("variant", variant);
    pyerr
}

/// Convert a marlin-klv `EncodeError` into a `KlvEncodeError` with
/// `variant`, `tag` and `kind` attributes. The enum is `#[non_exhaustive]`;
/// a variant these bindings do not know is a binding bug, never an "other".
pub(crate) fn klv_encode_err(py: Python<'_>, err: RustKlvEncodeError) -> PyErr {
    let (variant, tag, kind) = match err {
        RustKlvEncodeError::Unencodable { tag, kind } => ("unencodable", tag, Some(kind.name())),
        RustKlvEncodeError::OutOfRange { tag } => ("out_of_range", tag, None),
        _ => return crate::field::unsupported_variant("EncodeError", &err),
    };
    let pyerr = KlvEncodeError::new_err(err.to_string());
    // Same setattr-infallibility rationale as envelope_err above.
    let value = pyerr.value(py);
    let _ = value.setattr("variant", variant);
    let _ = value.setattr("tag", tag);
    let _ = value.setattr("kind", kind);
    pyerr
}
