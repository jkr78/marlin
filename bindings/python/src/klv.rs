//! Python wrappers for `marlin-klv` (MISB ST 0601 KLV).

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use pyo3::wrap_pyfunction;

use marlin_klv::St0601 as RustSt0601;

use crate::errors::klv_err;

/// Mutable MISB ST 0601 local set. Construct, set fields, then `klv.encode(...)`;
/// or obtain one from `klv.decode(...)`. Each tag exposes an engineering-unit
/// accessor (e.g. `sensor_latitude_degrees`) and a `raw_*` wire-integer escape hatch.
#[pyclass(name = "St0601", module = "marlin.klv")]
#[derive(Clone, Debug, Default)]
pub struct PySt0601 {
    inner: RustSt0601,
}

#[pymethods]
impl PySt0601 {
    #[new]
    #[pyo3(signature = (timestamp_us = 0, version = None))]
    fn new(timestamp_us: u64, version: Option<u8>) -> Self {
        let inner = RustSt0601 {
            timestamp_us,
            version,
            ..RustSt0601::default()
        };
        Self { inner }
    }

    #[getter]
    fn timestamp_us(&self) -> u64 {
        self.inner.timestamp_us
    }
    #[setter]
    fn set_timestamp_us(&mut self, v: u64) {
        self.inner.timestamp_us = v;
    }

    #[getter]
    fn version(&self) -> Option<u8> {
        self.inner.version
    }
    #[setter]
    fn set_version(&mut self, v: Option<u8>) {
        self.inner.version = v;
    }

    // ----- Tag 5: platform heading (u16) -----
    #[getter]
    fn raw_platform_heading(&self) -> Option<u16> {
        self.inner.platform_heading
    }
    #[setter]
    fn set_raw_platform_heading(&mut self, v: Option<u16>) {
        self.inner.platform_heading = v;
    }
    #[getter]
    fn platform_heading_degrees(&self) -> Option<f64> {
        self.inner.platform_heading_degrees()
    }
    #[setter]
    fn set_platform_heading_degrees(&mut self, v: f64) {
        self.inner.set_platform_heading_degrees(v);
    }

    // ----- Tag 6: platform pitch (i16) -----
    #[getter]
    fn raw_platform_pitch(&self) -> Option<i16> {
        self.inner.platform_pitch
    }
    #[setter]
    fn set_raw_platform_pitch(&mut self, v: Option<i16>) {
        self.inner.platform_pitch = v;
    }
    #[getter]
    fn platform_pitch_degrees(&self) -> Option<f64> {
        self.inner.platform_pitch_degrees()
    }
    #[setter]
    fn set_platform_pitch_degrees(&mut self, v: f64) {
        self.inner.set_platform_pitch_degrees(v);
    }

    // ----- Tag 7: platform roll (i16) -----
    #[getter]
    fn raw_platform_roll(&self) -> Option<i16> {
        self.inner.platform_roll
    }
    #[setter]
    fn set_raw_platform_roll(&mut self, v: Option<i16>) {
        self.inner.platform_roll = v;
    }
    #[getter]
    fn platform_roll_degrees(&self) -> Option<f64> {
        self.inner.platform_roll_degrees()
    }
    #[setter]
    fn set_platform_roll_degrees(&mut self, v: f64) {
        self.inner.set_platform_roll_degrees(v);
    }

    // ----- Tag 8: platform true airspeed (u8) -----
    #[getter]
    fn raw_platform_true_airspeed(&self) -> Option<u8> {
        self.inner.platform_true_airspeed
    }
    #[setter]
    fn set_raw_platform_true_airspeed(&mut self, v: Option<u8>) {
        self.inner.platform_true_airspeed = v;
    }
    #[getter]
    fn platform_true_airspeed_mps(&self) -> Option<f64> {
        self.inner.platform_true_airspeed_mps()
    }
    #[setter]
    fn set_platform_true_airspeed_mps(&mut self, v: f64) {
        self.inner.set_platform_true_airspeed_mps(v);
    }

    // ----- Tag 13: sensor latitude (i32) -----
    #[getter]
    fn raw_sensor_latitude(&self) -> Option<i32> {
        self.inner.sensor_latitude
    }
    #[setter]
    fn set_raw_sensor_latitude(&mut self, v: Option<i32>) {
        self.inner.sensor_latitude = v;
    }
    #[getter]
    fn sensor_latitude_degrees(&self) -> Option<f64> {
        self.inner.sensor_latitude_degrees()
    }
    #[setter]
    fn set_sensor_latitude_degrees(&mut self, v: f64) {
        self.inner.set_sensor_latitude_degrees(v);
    }

    // ----- Tag 14: sensor longitude (i32) -----
    #[getter]
    fn raw_sensor_longitude(&self) -> Option<i32> {
        self.inner.sensor_longitude
    }
    #[setter]
    fn set_raw_sensor_longitude(&mut self, v: Option<i32>) {
        self.inner.sensor_longitude = v;
    }
    #[getter]
    fn sensor_longitude_degrees(&self) -> Option<f64> {
        self.inner.sensor_longitude_degrees()
    }
    #[setter]
    fn set_sensor_longitude_degrees(&mut self, v: f64) {
        self.inner.set_sensor_longitude_degrees(v);
    }

    // ----- Tag 15: sensor true altitude (u16) -----
    #[getter]
    fn raw_sensor_true_altitude(&self) -> Option<u16> {
        self.inner.sensor_true_altitude
    }
    #[setter]
    fn set_raw_sensor_true_altitude(&mut self, v: Option<u16>) {
        self.inner.sensor_true_altitude = v;
    }
    #[getter]
    fn sensor_true_altitude_meters(&self) -> Option<f64> {
        self.inner.sensor_true_altitude_meters()
    }
    #[setter]
    fn set_sensor_true_altitude_meters(&mut self, v: f64) {
        self.inner.set_sensor_true_altitude_meters(v);
    }

    // ----- Tag 16: sensor horizontal field of view (u16) -----
    #[getter]
    fn raw_sensor_horizontal_fov(&self) -> Option<u16> {
        self.inner.sensor_horizontal_fov
    }
    #[setter]
    fn set_raw_sensor_horizontal_fov(&mut self, v: Option<u16>) {
        self.inner.sensor_horizontal_fov = v;
    }
    #[getter]
    fn sensor_horizontal_fov_degrees(&self) -> Option<f64> {
        self.inner.sensor_horizontal_fov_degrees()
    }
    #[setter]
    fn set_sensor_horizontal_fov_degrees(&mut self, v: f64) {
        self.inner.set_sensor_horizontal_fov_degrees(v);
    }

    // ----- Tag 17: sensor vertical field of view (u16) -----
    #[getter]
    fn raw_sensor_vertical_fov(&self) -> Option<u16> {
        self.inner.sensor_vertical_fov
    }
    #[setter]
    fn set_raw_sensor_vertical_fov(&mut self, v: Option<u16>) {
        self.inner.sensor_vertical_fov = v;
    }
    #[getter]
    fn sensor_vertical_fov_degrees(&self) -> Option<f64> {
        self.inner.sensor_vertical_fov_degrees()
    }
    #[setter]
    fn set_sensor_vertical_fov_degrees(&mut self, v: f64) {
        self.inner.set_sensor_vertical_fov_degrees(v);
    }

    // ----- Tag 18: sensor relative azimuth (u32) -----
    #[getter]
    fn raw_sensor_relative_azimuth(&self) -> Option<u32> {
        self.inner.sensor_relative_azimuth
    }
    #[setter]
    fn set_raw_sensor_relative_azimuth(&mut self, v: Option<u32>) {
        self.inner.sensor_relative_azimuth = v;
    }
    #[getter]
    fn sensor_relative_azimuth_degrees(&self) -> Option<f64> {
        self.inner.sensor_relative_azimuth_degrees()
    }
    #[setter]
    fn set_sensor_relative_azimuth_degrees(&mut self, v: f64) {
        self.inner.set_sensor_relative_azimuth_degrees(v);
    }

    // ----- Tag 19: sensor relative elevation (i32) -----
    #[getter]
    fn raw_sensor_relative_elevation(&self) -> Option<i32> {
        self.inner.sensor_relative_elevation
    }
    #[setter]
    fn set_raw_sensor_relative_elevation(&mut self, v: Option<i32>) {
        self.inner.sensor_relative_elevation = v;
    }
    #[getter]
    fn sensor_relative_elevation_degrees(&self) -> Option<f64> {
        self.inner.sensor_relative_elevation_degrees()
    }
    #[setter]
    fn set_sensor_relative_elevation_degrees(&mut self, v: f64) {
        self.inner.set_sensor_relative_elevation_degrees(v);
    }

    // ----- Tag 20: sensor relative roll (u32) -----
    #[getter]
    fn raw_sensor_relative_roll(&self) -> Option<u32> {
        self.inner.sensor_relative_roll
    }
    #[setter]
    fn set_raw_sensor_relative_roll(&mut self, v: Option<u32>) {
        self.inner.sensor_relative_roll = v;
    }
    #[getter]
    fn sensor_relative_roll_degrees(&self) -> Option<f64> {
        self.inner.sensor_relative_roll_degrees()
    }
    #[setter]
    fn set_sensor_relative_roll_degrees(&mut self, v: f64) {
        self.inner.set_sensor_relative_roll_degrees(v);
    }

    // ----- Tag 21: slant range (u32) -----
    #[getter]
    fn raw_slant_range(&self) -> Option<u32> {
        self.inner.slant_range
    }
    #[setter]
    fn set_raw_slant_range(&mut self, v: Option<u32>) {
        self.inner.slant_range = v;
    }
    #[getter]
    fn slant_range_meters(&self) -> Option<f64> {
        self.inner.slant_range_meters()
    }
    #[setter]
    fn set_slant_range_meters(&mut self, v: f64) {
        self.inner.set_slant_range_meters(v);
    }

    // ----- Tag 22: target width (u16) -----
    #[getter]
    fn raw_target_width(&self) -> Option<u16> {
        self.inner.target_width
    }
    #[setter]
    fn set_raw_target_width(&mut self, v: Option<u16>) {
        self.inner.target_width = v;
    }
    #[getter]
    fn target_width_meters(&self) -> Option<f64> {
        self.inner.target_width_meters()
    }
    #[setter]
    fn set_target_width_meters(&mut self, v: f64) {
        self.inner.set_target_width_meters(v);
    }

    // ----- Tag 23: frame center latitude (i32) -----
    #[getter]
    fn raw_frame_center_latitude(&self) -> Option<i32> {
        self.inner.frame_center_latitude
    }
    #[setter]
    fn set_raw_frame_center_latitude(&mut self, v: Option<i32>) {
        self.inner.frame_center_latitude = v;
    }
    #[getter]
    fn frame_center_latitude_degrees(&self) -> Option<f64> {
        self.inner.frame_center_latitude_degrees()
    }
    #[setter]
    fn set_frame_center_latitude_degrees(&mut self, v: f64) {
        self.inner.set_frame_center_latitude_degrees(v);
    }

    // ----- Tag 24: frame center longitude (i32) -----
    #[getter]
    fn raw_frame_center_longitude(&self) -> Option<i32> {
        self.inner.frame_center_longitude
    }
    #[setter]
    fn set_raw_frame_center_longitude(&mut self, v: Option<i32>) {
        self.inner.frame_center_longitude = v;
    }
    #[getter]
    fn frame_center_longitude_degrees(&self) -> Option<f64> {
        self.inner.frame_center_longitude_degrees()
    }
    #[setter]
    fn set_frame_center_longitude_degrees(&mut self, v: f64) {
        self.inner.set_frame_center_longitude_degrees(v);
    }

    // ----- Tag 25: frame center elevation (u16) -----
    #[getter]
    fn raw_frame_center_elevation(&self) -> Option<u16> {
        self.inner.frame_center_elevation
    }
    #[setter]
    fn set_raw_frame_center_elevation(&mut self, v: Option<u16>) {
        self.inner.frame_center_elevation = v;
    }
    #[getter]
    fn frame_center_elevation_meters(&self) -> Option<f64> {
        self.inner.frame_center_elevation_meters()
    }
    #[setter]
    fn set_frame_center_elevation_meters(&mut self, v: f64) {
        self.inner.set_frame_center_elevation_meters(v);
    }

    // ----- Tag 40: target location latitude (i32) -----
    #[getter]
    fn raw_target_location_latitude(&self) -> Option<i32> {
        self.inner.target_location_latitude
    }
    #[setter]
    fn set_raw_target_location_latitude(&mut self, v: Option<i32>) {
        self.inner.target_location_latitude = v;
    }
    #[getter]
    fn target_location_latitude_degrees(&self) -> Option<f64> {
        self.inner.target_location_latitude_degrees()
    }
    #[setter]
    fn set_target_location_latitude_degrees(&mut self, v: f64) {
        self.inner.set_target_location_latitude_degrees(v);
    }

    // ----- Tag 41: target location longitude (i32) -----
    #[getter]
    fn raw_target_location_longitude(&self) -> Option<i32> {
        self.inner.target_location_longitude
    }
    #[setter]
    fn set_raw_target_location_longitude(&mut self, v: Option<i32>) {
        self.inner.target_location_longitude = v;
    }
    #[getter]
    fn target_location_longitude_degrees(&self) -> Option<f64> {
        self.inner.target_location_longitude_degrees()
    }
    #[setter]
    fn set_target_location_longitude_degrees(&mut self, v: f64) {
        self.inner.set_target_location_longitude_degrees(v);
    }

    // ----- Tag 42: target location elevation (u16) -----
    #[getter]
    fn raw_target_location_elevation(&self) -> Option<u16> {
        self.inner.target_location_elevation
    }
    #[setter]
    fn set_raw_target_location_elevation(&mut self, v: Option<u16>) {
        self.inner.target_location_elevation = v;
    }
    #[getter]
    fn target_location_elevation_meters(&self) -> Option<f64> {
        self.inner.target_location_elevation_meters()
    }
    #[setter]
    fn set_target_location_elevation_meters(&mut self, v: f64) {
        self.inner.set_target_location_elevation_meters(v);
    }

    // ----- Unrecognized tags: list of (tag, bytes) in wire order -----
    #[getter]
    fn unknown(&self, py: Python<'_>) -> Vec<(u8, Py<PyAny>)> {
        self.inner
            .unknown
            .iter()
            .map(|(t, v)| (*t, PyBytes::new(py, v).into_any().unbind()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "St0601(timestamp_us={}, version={:?})",
            self.inner.timestamp_us, self.inner.version,
        )
    }
}

/// Decode a KLV datagram into an `St0601`. Raises `KlvError` on malformed input.
#[pyfunction]
fn decode(data: &[u8]) -> PyResult<PySt0601> {
    marlin_klv::decode(data)
        .map(|inner| PySt0601 { inner })
        .map_err(klv_err)
}

/// Encode an `St0601` into a KLV datagram (`bytes`).
#[pyfunction]
fn encode<'py>(py: Python<'py>, set: &PySt0601) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = Vec::new();
    marlin_klv::encode(&set.inner, &mut out).map_err(klv_err)?;
    Ok(PyBytes::new(py, &out))
}

/// Cheap Tag 2 peek: return the precision timestamp (microseconds) without
/// verifying the checksum. `None` when Tag 2 is absent.
#[pyfunction]
fn precision_timestamp(data: &[u8]) -> PyResult<Option<u64>> {
    marlin_klv::precision_timestamp(data).map_err(klv_err)
}

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "klv")?;
    m.add_class::<PySt0601>()?;
    m.add_function(wrap_pyfunction!(decode, &m)?)?;
    m.add_function(wrap_pyfunction!(encode, &m)?)?;
    m.add_function(wrap_pyfunction!(precision_timestamp, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
