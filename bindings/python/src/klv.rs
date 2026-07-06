//! Python wrappers for `marlin-klv` (MISB ST 0601 KLV).

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "klv")?;
    parent.add_submodule(&m)?;
    Ok(())
}
