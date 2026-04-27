//! Python wrappers for `marlin-nmea-envelope`.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule, PyTuple};
use pyo3::wrap_pyfunction;

use marlin_nmea_envelope::{
    parse as rust_parse, OneShot, RawSentence, SentenceSource, Streaming,
};

use crate::errors::envelope_err;

pub(crate) const DEFAULT_MAX_SIZE: usize = 65_536;

/// Python-visible owned copy of `RawSentence<'a>`.
///
/// Borrowed fields (`&[u8]`, `&str`, `Vec<&[u8]>`) are cloned into owned
/// equivalents (`Vec<u8>`, `String`, `Vec<Vec<u8>>`) at construction time.
#[pyclass(name = "RawSentence", frozen, eq, hash, module = "marlin.envelope")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyRawSentence {
    start_delimiter: u8,
    talker: Option<[u8; 2]>,
    sentence_type: String,
    fields: Vec<Vec<u8>>,
    tag_block: Option<Vec<u8>>,
    checksum_ok: bool,
    raw: Vec<u8>,
}

impl PyRawSentence {
    /// Clone from a borrowed Rust `RawSentence` into the owned Python form.
    pub(crate) fn from_borrowed(s: &RawSentence<'_>) -> Self {
        Self {
            start_delimiter: s.start_delimiter,
            talker: s.talker,
            sentence_type: s.sentence_type.to_string(),
            fields: s.fields.iter().map(|f| f.to_vec()).collect(),
            tag_block: s.tag_block.map(<[u8]>::to_vec),
            checksum_ok: s.checksum_ok,
            raw: s.raw.to_vec(),
        }
    }
}

#[pymethods]
impl PyRawSentence {
    #[getter]
    fn start_delimiter<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &[self.start_delimiter])
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }

    #[getter]
    fn sentence_type(&self) -> &str {
        &self.sentence_type
    }

    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let items: Vec<Bound<'py, PyBytes>> =
            self.fields.iter().map(|f| PyBytes::new(py, f)).collect();
        PyTuple::new(py, items)
    }

    #[getter]
    fn tag_block<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.tag_block.as_deref().map(|b| PyBytes::new(py, b))
    }

    #[getter]
    fn checksum_ok(&self) -> bool {
        self.checksum_ok
    }

    #[getter]
    fn raw<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.raw)
    }

    fn __repr__(&self) -> String {
        let talker_repr = match self.talker {
            Some(t) => format!("b{:?}", core::str::from_utf8(&t).unwrap_or("??")),
            None => "None".to_string(),
        };
        format!(
            "RawSentence(talker={}, sentence_type={:?}, fields={}, checksum_ok={})",
            talker_repr,
            self.sentence_type,
            self.fields.len(),
            self.checksum_ok
        )
    }

    /// Dict view of all fields — useful for JSON serialisation and tests.
    fn as_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        use pyo3::types::PyDict;
        let d = PyDict::new(py);
        d.set_item("start_delimiter", self.start_delimiter(py))?;
        d.set_item("talker", self.talker(py))?;
        d.set_item("sentence_type", self.sentence_type())?;
        d.set_item("fields", self.fields(py)?)?;
        d.set_item("tag_block", self.tag_block(py))?;
        d.set_item("checksum_ok", self.checksum_ok)?;
        d.set_item("raw", self.raw(py))?;
        Ok(d)
    }
}

/// One-shot envelope parser: designed for datagram-shaped input.
#[pyclass(name = "OneShotParser", module = "marlin.envelope")]
pub struct PyOneShotParser {
    inner: OneShot,
}

#[pymethods]
impl PyOneShotParser {
    #[new]
    fn new() -> Self {
        Self {
            inner: OneShot::new(),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.inner.feed(data);
    }

    /// Return the next parsed sentence, or `None` if none is buffered.
    /// Raises `EnvelopeError` if the buffered bytes failed to parse.
    fn next_sentence(&mut self, py: Python<'_>) -> PyResult<Option<PyRawSentence>> {
        match self.inner.next_sentence() {
            None => Ok(None),
            Some(Ok(raw)) => Ok(Some(PyRawSentence::from_borrowed(&raw))),
            Some(Err(e)) => Err(envelope_err(py, e)),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyEnvelopeIterator>> {
        // PyO3 0.27 idiom: `impl From<PyRef<T>> for Py<T>` lets us consume
        // the borrow directly into an owned handle. `Infallible`, no `?`.
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyEnvelopeIterator::new_oneshot(parser, false))
    }

    /// Explicit iterator with strict/lenient switch.
    #[pyo3(signature = (strict = false))]
    fn iter(slf: PyRef<'_, Self>, strict: bool) -> PyResult<Py<PyEnvelopeIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyEnvelopeIterator::new_oneshot(parser, strict))
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type, _exc_val, _exc_tb))]
    #[allow(clippy::unused_self)] // __exit__ protocol requires &self; body is stateless
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false // do not suppress exceptions
    }
}

/// Streaming envelope parser: designed for byte-stream transports.
#[pyclass(name = "StreamingParser", module = "marlin.envelope")]
pub struct PyStreamingParser {
    inner: Streaming,
}

#[pymethods]
impl PyStreamingParser {
    #[new]
    #[pyo3(signature = (max_size = DEFAULT_MAX_SIZE))]
    fn new(max_size: usize) -> Self {
        Self {
            inner: Streaming::with_capacity(max_size),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.inner.feed(data);
    }

    fn next_sentence(&mut self, py: Python<'_>) -> PyResult<Option<PyRawSentence>> {
        match self.inner.next_sentence() {
            None => Ok(None),
            Some(Ok(raw)) => Ok(Some(PyRawSentence::from_borrowed(&raw))),
            Some(Err(e)) => Err(envelope_err(py, e)),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyEnvelopeIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyEnvelopeIterator::new_streaming(parser, false))
    }

    #[pyo3(signature = (strict = false))]
    fn iter(slf: PyRef<'_, Self>, strict: bool) -> PyResult<Py<PyEnvelopeIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyEnvelopeIterator::new_streaming(parser, strict))
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type, _exc_val, _exc_tb))]
    #[allow(clippy::unused_self)] // __exit__ protocol requires &self; body is stateless
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false // do not suppress exceptions
    }
}

/// Iterator wrapper; swallows errors by default, raises in strict mode.
#[pyclass(module = "marlin.envelope")]
pub struct PyEnvelopeIterator {
    source: ParserRef,
    strict: bool,
}

enum ParserRef {
    OneShot(Py<PyOneShotParser>),
    Streaming(Py<PyStreamingParser>),
}

impl PyEnvelopeIterator {
    fn new_oneshot(p: Py<PyOneShotParser>, strict: bool) -> Self {
        Self {
            source: ParserRef::OneShot(p),
            strict,
        }
    }

    fn new_streaming(p: Py<PyStreamingParser>, strict: bool) -> Self {
        Self {
            source: ParserRef::Streaming(p),
            strict,
        }
    }
}

#[pymethods]
impl PyEnvelopeIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<PyRawSentence> {
        loop {
            let result = match &self.source {
                ParserRef::OneShot(p) => {
                    let mut borrow = p.borrow_mut(py);
                    borrow.next_sentence(py)
                }
                ParserRef::Streaming(p) => {
                    let mut borrow = p.borrow_mut(py);
                    borrow.next_sentence(py)
                }
            };
            match result {
                Ok(Some(s)) => return Ok(s),
                Ok(None) => {
                    return Err(pyo3::exceptions::PyStopIteration::new_err(()));
                }
                Err(e) => {
                    if self.strict {
                        return Err(e);
                    }
                    // lenient — swallow and loop for the next sentence.
                }
            }
        }
    }
}

impl PyRawSentence {
    /// Reconstruct a borrowed `RawSentence<'_>` that borrows from `self`.
    /// Used by the `decode_*` pyfunctions in `nmea.rs` to hand the owned
    /// Python copy back to the Rust decoders as a `&RawSentence<'_>`.
    ///
    /// The returned view's lifetime is tied to `&self`, so the caller must
    /// consume it before the `PyRawSentence` is dropped.
    pub(crate) fn to_rust(&self) -> RawSentence<'_> {
        RawSentence {
            start_delimiter: self.start_delimiter,
            talker: self.talker,
            sentence_type: &self.sentence_type,
            fields: self.fields.iter().map(Vec::as_slice).collect(),
            tag_block: self.tag_block.as_deref(),
            checksum_ok: self.checksum_ok,
            raw: &self.raw,
        }
    }
}

/// Convenience: parse a single complete sentence from `bytes`.
/// Equivalent to constructing an `OneShotParser`, feeding the bytes, and
/// taking the first result. Raises `EnvelopeError` on parse failure.
#[pyfunction]
#[pyo3(name = "parse")]
fn py_parse(py: Python<'_>, data: &[u8]) -> PyResult<PyRawSentence> {
    match rust_parse(data) {
        Ok(raw) => Ok(PyRawSentence::from_borrowed(&raw)),
        Err(e) => Err(envelope_err(py, e)),
    }
}

/// Register the envelope submodule on the `_core` module.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "envelope")?;
    m.add_class::<PyRawSentence>()?;
    m.add_class::<PyOneShotParser>()?;
    m.add_class::<PyStreamingParser>()?;
    m.add_class::<PyEnvelopeIterator>()?;
    m.add_function(wrap_pyfunction!(py_parse, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
