//! Python bindings for the marlin NMEA 0183 + AIS parser suite.

#![allow(clippy::needless_pass_by_value)]

use pyo3::prelude::*;

mod ais;
mod envelope;
mod errors;
mod field;
mod klv;
mod nmea;

#[pymodule]
fn _core(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(py, m)?;
    ais::register(py, m)?;
    envelope::register(py, m)?;
    nmea::register(py, m)?;
    field::register(py, m)?;
    klv::register(py, m)?;
    Ok(())
}
