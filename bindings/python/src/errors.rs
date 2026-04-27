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
use marlin_nmea_0183::DecodeError as RustDecodeError;
use marlin_nmea_envelope::Error as RustEnvelopeError;

create_exception!(_core, MarlinError, PyException);
create_exception!(_core, EnvelopeError, MarlinError);
create_exception!(_core, DecodeError, MarlinError);
create_exception!(_core, AisError, MarlinError);
create_exception!(_core, ReassemblyError, AisError);

/// Register all exceptions on the given module. Called from `lib.rs`.
pub(crate) fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("MarlinError", py.get_type::<MarlinError>())?;
    m.add("EnvelopeError", py.get_type::<EnvelopeError>())?;
    m.add("DecodeError", py.get_type::<DecodeError>())?;
    m.add("AisError", py.get_type::<AisError>())?;
    m.add("ReassemblyError", py.get_type::<ReassemblyError>())?;
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
    use marlin_ais::AisError::{
        Envelope, ReassemblyChannelMismatch, ReassemblyOutOfOrder, ReassemblyTimeout,
    };
    let msg = err.to_string();
    match err {
        ReassemblyTimeout | ReassemblyOutOfOrder | ReassemblyChannelMismatch => {
            ReassemblyError::new_err(msg)
        }
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
