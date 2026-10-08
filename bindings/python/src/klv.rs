//! Python wrappers for `marlin-klv` (MISB ST 0601 KLV).

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use pyo3::wrap_pyfunction;

use marlin_klv::St0601 as RustSt0601;

use crate::errors::{klv_encode_err, klv_err};
use crate::field::{field_arg, from_py, repr_state, to_py, PyFieldState};

/// Mutable MISB ST 0601 local set. Construct with the mandatory `timestamp_us`,
/// assign fields, then `klv.encode(...)`; or obtain one from `klv.decode(...)`.
///
/// Every scaled tag is a `FieldState[float]` property in engineering units
/// (`sensor_latitude_degrees`, `slant_range_meters`, ...): an omitted tag reads
/// `FieldState.NotAvailable()`, the ST 0601 sentinel on a signed tag
/// `FieldState.SenderError(code)`, a known tag with the wrong wire length
/// `FieldState.Invalid(None)` with its bytes kept in `unknown`. A setter takes a
/// `FieldState`, a bare number (which becomes `FieldState.Value`) or `None`
/// (`FieldState.NotAvailable()`); nothing is clamped, `encode` rejects a value
/// outside the tag's range.
#[pyclass(name = "St0601", module = "marlin.klv")]
#[derive(Clone, Debug)]
pub struct PySt0601 {
    inner: RustSt0601,
}

/// The `#[pymethods]` block of [`PySt0601`]: the hand-written members plus one
/// `FieldState[float]` property per scaled tag, generated from the list.
macro_rules! st0601_methods {
    ($({ $field:ident, $setter:ident, $doc:literal }),+ $(,)?) => {
        #[pymethods]
        impl PySt0601 {
            #[new]
            #[pyo3(signature = (timestamp_us, version = None))]
            fn new(timestamp_us: u64, version: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
                let mut inner = RustSt0601::new(timestamp_us);
                inner.version = field_arg(version)?;
                Ok(Self { inner })
            }

            /// Tag 2: precision timestamp, microseconds since the UNIX epoch (UTC).
            /// Mandatory: always encoded, always present in a decoded set.
            #[getter]
            fn timestamp_us(&self) -> u64 {
                self.inner.timestamp_us
            }
            #[setter]
            fn set_timestamp_us(&mut self, v: u64) {
                self.inner.timestamp_us = v;
            }

            /// Tag 65: UAS LS document version number, a `FieldState[int]`.
            #[getter]
            fn version(&self, py: Python<'_>) -> PyResult<PyFieldState> {
                to_py(py, self.inner.version)
            }
            #[setter]
            fn set_version(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
                self.inner.version = from_py(v)?;
                Ok(())
            }

            $(
                #[doc = $doc]
                #[getter]
                fn $field(&self, py: Python<'_>) -> PyResult<PyFieldState> {
                    to_py(py, self.inner.$field)
                }
                #[setter]
                fn $setter(&mut self, v: &Bound<'_, PyAny>) -> PyResult<()> {
                    self.inner.$field = from_py(v)?;
                    Ok(())
                }
            )+

            /// Tags the codec does not type, as `(tag, bytes)` in wire order, plus
            /// the bytes of any known tag that arrived with the wrong wire length.
            /// Read-only.
            #[getter]
            fn unknown(&self, py: Python<'_>) -> Vec<(u8, Py<PyAny>)> {
                self.inner
                    .unknown
                    .iter()
                    .map(|(t, v)| (*t, PyBytes::new(py, v).into_any().unbind()))
                    .collect()
            }

            fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
                Ok(format!(
                    "St0601(timestamp_us={}, version={})",
                    self.inner.timestamp_us,
                    repr_state(py, self.inner.version)?,
                ))
            }
        }
    };
}

st0601_methods! {
    { platform_heading_degrees, set_platform_heading_degrees,
      "Tag 5: platform heading in degrees (0..=360)." },
    { platform_pitch_degrees, set_platform_pitch_degrees,
      "Tag 6: platform pitch in degrees (-20..=20); \
       the sentinel reads `FieldState.SenderError(-32768)`." },
    { platform_roll_degrees, set_platform_roll_degrees,
      "Tag 7: platform roll in degrees (-50..=50); \
       the sentinel reads `FieldState.SenderError(-32768)`." },
    { platform_true_airspeed_mps, set_platform_true_airspeed_mps,
      "Tag 8: platform true airspeed in m/s (0..=255)." },
    { sensor_latitude_degrees, set_sensor_latitude_degrees,
      "Tag 13: sensor latitude in degrees WGS84 (-90..=90); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { sensor_longitude_degrees, set_sensor_longitude_degrees,
      "Tag 14: sensor longitude in degrees WGS84 (-180..=180); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { sensor_true_altitude_meters, set_sensor_true_altitude_meters,
      "Tag 15: sensor true altitude in meters MSL (-900..=19000)." },
    { sensor_horizontal_fov_degrees, set_sensor_horizontal_fov_degrees,
      "Tag 16: sensor horizontal field of view in degrees (0..=180)." },
    { sensor_vertical_fov_degrees, set_sensor_vertical_fov_degrees,
      "Tag 17: sensor vertical field of view in degrees (0..=180)." },
    { sensor_relative_azimuth_degrees, set_sensor_relative_azimuth_degrees,
      "Tag 18: sensor relative azimuth in degrees (0..=360)." },
    { sensor_relative_elevation_degrees, set_sensor_relative_elevation_degrees,
      "Tag 19: sensor relative elevation in degrees (-180..=180, negative = below horizon); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { sensor_relative_roll_degrees, set_sensor_relative_roll_degrees,
      "Tag 20: sensor relative roll in degrees (0..=360, clockwise from behind the camera)." },
    { slant_range_meters, set_slant_range_meters,
      "Tag 21: slant range in meters (0..=5000000)." },
    { target_width_meters, set_target_width_meters,
      "Tag 22: target width in meters (0..=10000)." },
    { frame_center_latitude_degrees, set_frame_center_latitude_degrees,
      "Tag 23: frame center latitude in degrees WGS84 (-90..=90); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { frame_center_longitude_degrees, set_frame_center_longitude_degrees,
      "Tag 24: frame center longitude in degrees WGS84 (-180..=180); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { frame_center_elevation_meters, set_frame_center_elevation_meters,
      "Tag 25: frame center elevation in meters MSL (-900..=19000)." },
    { target_location_latitude_degrees, set_target_location_latitude_degrees,
      "Tag 40: target location latitude in degrees WGS84 (-90..=90); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { target_location_longitude_degrees, set_target_location_longitude_degrees,
      "Tag 41: target location longitude in degrees WGS84 (-180..=180); \
       the sentinel reads `FieldState.SenderError(-2147483648)`." },
    { target_location_elevation_meters, set_target_location_elevation_meters,
      "Tag 42: target location elevation in meters MSL (-900..=19000)." },
}

/// Read-only metadata for one ST 0601 tag: its wire `number`, the `St0601`
/// field base `name` (e.g. `"sensor_latitude"`), and its engineering `unit`
/// (`"degrees"` / `"meters"` / `"mps"` / `"microseconds"`, or `None`).
#[pyclass(name = "TagInfo", frozen, eq, hash, module = "marlin.klv")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyTagInfo {
    #[pyo3(get)]
    number: u8,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    unit: Option<String>,
}

#[pymethods]
impl PyTagInfo {
    fn __repr__(&self) -> String {
        let unit = match &self.unit {
            Some(u) => format!("'{u}'"),
            None => "None".into(),
        };
        format!(
            "TagInfo(number={}, name='{}', unit={unit})",
            self.number, self.name
        )
    }
}

impl From<marlin_klv::TagInfo> for PyTagInfo {
    fn from(t: marlin_klv::TagInfo) -> Self {
        Self {
            number: t.number,
            name: t.name.into(),
            unit: t.unit.map(Into::into),
        }
    }
}

/// Every ST 0601 tag the codec decodes into a typed field, in ascending tag
/// order (Tag 2 timestamp and Tag 65 version included; Tag 1 checksum is not).
#[pyfunction]
fn tags() -> Vec<PyTagInfo> {
    marlin_klv::tags().iter().copied().map(Into::into).collect()
}

/// Wire tag number for a field base name (e.g. `"sensor_latitude"`), or `None`.
#[pyfunction]
fn tag_number(name: &str) -> Option<u8> {
    marlin_klv::tag_number(name)
}

/// Field base name for a wire tag number, or `None` if untyped by this codec.
#[pyfunction]
fn tag_name(number: u8) -> Option<String> {
    marlin_klv::tag_name(number).map(Into::into)
}

/// Decode a KLV datagram into an `St0601`. Raises `KlvError` for a structural
/// reason only: framing, the checksum, or the mandatory Tag 2 timestamp absent
/// or malformed. A field's value never fails the set.
#[pyfunction]
fn decode(py: Python<'_>, data: &[u8]) -> PyResult<PySt0601> {
    marlin_klv::decode(data)
        .map(|inner| PySt0601 { inner })
        .map_err(|err| klv_err(py, err))
}

/// Encode an `St0601` into a KLV datagram (`bytes`). Raises `KlvEncodeError`
/// for a field in a state the wire cannot carry, or a value outside its tag's
/// range or NaN.
#[pyfunction]
fn encode<'py>(py: Python<'py>, set: &PySt0601) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = Vec::new();
    marlin_klv::encode(&set.inner, &mut out).map_err(|err| klv_encode_err(py, err))?;
    Ok(PyBytes::new(py, &out))
}

/// Cheap Tag 2 peek: return the precision timestamp (microseconds) without
/// verifying the checksum. `None` when Tag 2 is absent.
#[pyfunction]
fn precision_timestamp(py: Python<'_>, data: &[u8]) -> PyResult<Option<u64>> {
    marlin_klv::precision_timestamp(data).map_err(|err| klv_err(py, err))
}

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "klv")?;
    m.add_class::<PySt0601>()?;
    m.add_class::<PyTagInfo>()?;
    m.add("UAS_LS_KEY", PyBytes::new(py, &marlin_klv::UAS_LS_KEY))?;
    m.add_function(wrap_pyfunction!(decode, &m)?)?;
    m.add_function(wrap_pyfunction!(encode, &m)?)?;
    m.add_function(wrap_pyfunction!(precision_timestamp, &m)?)?;
    m.add_function(wrap_pyfunction!(tags, &m)?)?;
    m.add_function(wrap_pyfunction!(tag_number, &m)?)?;
    m.add_function(wrap_pyfunction!(tag_name, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
