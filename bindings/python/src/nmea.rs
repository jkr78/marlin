//! Python wrappers for `marlin-nmea-0183`.
//!
//! Each typed message variant is its own `#[pyclass]`. The `Nmea0183Message`
//! Rust enum has no single Python class — variants are a structural union.

use core::str::FromStr;

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use pyo3::wrap_pyfunction;

use marlin_nmea_0183::{
    decode as rust_decode, decode_gga as rust_decode_gga, decode_hdt as rust_decode_hdt,
    decode_prdid as rust_decode_prdid, decode_psxn as rust_decode_psxn,
    decode_vtg as rust_decode_vtg, decode_with as rust_decode_with,
    DecodeOptions as RustDecodeOptions, GgaData, GgaFixQuality as RustGgaFixQuality, HdtData,
    Nmea0183Error, Nmea0183Message, Nmea0183Parser as RustNmea0183, PrdidData,
    PrdidDialect as RustPrdidDialect, PrdidPitchRollHeading as RustPrdidPitchRollHeading,
    PrdidRollPitchHeading as RustPrdidRollPitchHeading, PsxnData, PsxnLayout as RustPsxnLayout,
    PsxnSlot as RustPsxnSlot, UtcTime as RustUtcTime, VtgData, VtgMode as RustVtgMode,
};
use marlin_nmea_envelope::{OneShot, Streaming};

use crate::envelope::{PyRawSentence, DEFAULT_MAX_SIZE};
use crate::errors::{decode_err, envelope_err};

// ---------- Enums ----------

/// GPS fix quality indicator (mirrors `GgaFixQuality`).
///
/// `GgaFixQuality::Other(u8)` carries a raw byte this fieldless Python
/// enum cannot represent; it collapses to `INVALID` for v0.1.
#[pyclass(name = "GgaFixQuality", eq, eq_int, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyGgaFixQuality {
    #[pyo3(name = "INVALID")]
    Invalid = 0,
    #[pyo3(name = "GPS_FIX")]
    GpsFix = 1,
    #[pyo3(name = "DGPS_FIX")]
    DgpsFix = 2,
    #[pyo3(name = "PPS_FIX")]
    PpsFix = 3,
    #[pyo3(name = "RTK_FIXED")]
    RtkFixed = 4,
    #[pyo3(name = "RTK_FLOAT")]
    RtkFloat = 5,
    #[pyo3(name = "DEAD_RECKONING")]
    DeadReckoning = 6,
    #[pyo3(name = "MANUAL_INPUT")]
    ManualInput = 7,
    #[pyo3(name = "SIMULATOR")]
    Simulator = 8,
}

impl From<RustGgaFixQuality> for PyGgaFixQuality {
    // `Other(u8)` collapses to `Invalid` via the wildcard below — fieldless
    // Python enum cannot carry the raw byte. `match_same_arms` would fire on
    // the intentional `Invalid => Invalid` + `_ => Invalid` pair.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustGgaFixQuality) -> Self {
        match v {
            RustGgaFixQuality::Invalid => Self::Invalid,
            RustGgaFixQuality::GpsFix => Self::GpsFix,
            RustGgaFixQuality::DgpsFix => Self::DgpsFix,
            RustGgaFixQuality::PpsFix => Self::PpsFix,
            RustGgaFixQuality::RtkFixed => Self::RtkFixed,
            RustGgaFixQuality::RtkFloat => Self::RtkFloat,
            RustGgaFixQuality::DeadReckoning => Self::DeadReckoning,
            RustGgaFixQuality::ManualInput => Self::ManualInput,
            RustGgaFixQuality::Simulator => Self::Simulator,
            _ => Self::Invalid,
        }
    }
}

/// VTG mode indicator (mirrors `VtgMode`).
///
/// `VtgMode::Other(u8)` collapses to `NOT_VALID` for v0.1 — same
/// information-loss rationale as `PyGgaFixQuality`.
#[pyclass(name = "VtgMode", eq, eq_int, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyVtgMode {
    #[pyo3(name = "NOT_VALID")]
    NotValid = 0,
    #[pyo3(name = "AUTONOMOUS")]
    Autonomous = 1,
    #[pyo3(name = "DIFFERENTIAL")]
    Differential = 2,
    #[pyo3(name = "ESTIMATED")]
    Estimated = 3,
    #[pyo3(name = "MANUAL")]
    Manual = 4,
    #[pyo3(name = "SIMULATOR")]
    Simulator = 5,
}

impl From<RustVtgMode> for PyVtgMode {
    // `Other(u8)` collapses to `NotValid` via the wildcard — same rationale
    // as `PyGgaFixQuality`; silence `match_same_arms` for the pair.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustVtgMode) -> Self {
        match v {
            RustVtgMode::Autonomous => Self::Autonomous,
            RustVtgMode::Differential => Self::Differential,
            RustVtgMode::Estimated => Self::Estimated,
            RustVtgMode::NotValid => Self::NotValid,
            RustVtgMode::Manual => Self::Manual,
            RustVtgMode::Simulator => Self::Simulator,
            _ => Self::NotValid,
        }
    }
}

// ---------- UtcTime ----------

/// Frozen UTC time-of-day value (mirrors `UtcTime`).
#[pyclass(name = "UtcTime", frozen, eq, hash, module = "marlin.nmea")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyUtcTime {
    #[pyo3(get)]
    hour: u8,
    #[pyo3(get)]
    minute: u8,
    #[pyo3(get)]
    second: u8,
    #[pyo3(get)]
    millisecond: u16,
}

#[pymethods]
impl PyUtcTime {
    #[new]
    fn new(hour: u8, minute: u8, second: u8, millisecond: u16) -> Self {
        Self {
            hour,
            minute,
            second,
            millisecond,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "UtcTime({:02}:{:02}:{:02}.{:03})",
            self.hour, self.minute, self.second, self.millisecond
        )
    }
}

impl From<RustUtcTime> for PyUtcTime {
    fn from(t: RustUtcTime) -> Self {
        Self {
            hour: t.hour,
            minute: t.minute,
            second: t.second,
            millisecond: t.millisecond,
        }
    }
}

// ---------- Gga ----------

/// Frozen `$__GGA` message (mirrors `GgaData`).
#[pyclass(name = "Gga", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyGga {
    talker: Option<[u8; 2]>,
    utc: Option<PyUtcTime>,
    latitude_deg: Option<f64>,
    longitude_deg: Option<f64>,
    fix_quality: PyGgaFixQuality,
    satellites_used: Option<u8>,
    hdop: Option<f32>,
    altitude_m: Option<f32>,
    geoid_separation_m: Option<f32>,
    dgps_age_s: Option<f32>,
    dgps_station_id: Option<u16>,
}

#[pymethods]
impl PyGga {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        talker: Option<&[u8]>,
        utc: Option<PyUtcTime>,
        latitude_deg: Option<f64>,
        longitude_deg: Option<f64>,
        fix_quality: PyGgaFixQuality,
        satellites_used: Option<u8>,
        hdop: Option<f32>,
        altitude_m: Option<f32>,
        geoid_separation_m: Option<f32>,
        dgps_age_s: Option<f32>,
        dgps_station_id: Option<u16>,
    ) -> PyResult<Self> {
        let talker = normalize_talker(talker)?;
        Ok(Self {
            talker,
            utc,
            latitude_deg,
            longitude_deg,
            fix_quality,
            satellites_used,
            hdop,
            altitude_m,
            geoid_separation_m,
            dgps_age_s,
            dgps_station_id,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn utc(&self) -> Option<PyUtcTime> {
        self.utc.clone()
    }
    #[getter]
    fn latitude_deg(&self) -> Option<f64> {
        self.latitude_deg
    }
    #[getter]
    fn longitude_deg(&self) -> Option<f64> {
        self.longitude_deg
    }
    #[getter]
    fn fix_quality(&self) -> PyGgaFixQuality {
        self.fix_quality
    }
    #[getter]
    fn satellites_used(&self) -> Option<u8> {
        self.satellites_used
    }
    #[getter]
    fn hdop(&self) -> Option<f32> {
        self.hdop
    }
    #[getter]
    fn altitude_m(&self) -> Option<f32> {
        self.altitude_m
    }
    #[getter]
    fn geoid_separation_m(&self) -> Option<f32> {
        self.geoid_separation_m
    }
    #[getter]
    fn dgps_age_s(&self) -> Option<f32> {
        self.dgps_age_s
    }
    #[getter]
    fn dgps_station_id(&self) -> Option<u16> {
        self.dgps_station_id
    }

    fn __repr__(&self) -> String {
        format!(
            "Gga(talker={}, fix_quality={:?}, lat={:?}, lon={:?})",
            repr_talker(self.talker),
            self.fix_quality,
            self.latitude_deg,
            self.longitude_deg,
        )
    }
}

impl From<GgaData> for PyGga {
    fn from(d: GgaData) -> Self {
        Self {
            talker: d.talker,
            utc: d.utc.map(PyUtcTime::from),
            latitude_deg: d.latitude_deg,
            longitude_deg: d.longitude_deg,
            fix_quality: d.fix_quality.into(),
            satellites_used: d.satellites_used,
            hdop: d.hdop,
            altitude_m: d.altitude_m,
            geoid_separation_m: d.geoid_separation_m,
            dgps_age_s: d.dgps_age_s,
            dgps_station_id: d.dgps_station_id,
        }
    }
}

// ---------- Vtg ----------

/// Frozen `$__VTG` message (mirrors `VtgData`).
#[pyclass(name = "Vtg", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyVtg {
    talker: Option<[u8; 2]>,
    course_true_deg: Option<f32>,
    course_magnetic_deg: Option<f32>,
    speed_knots: Option<f32>,
    speed_kmh: Option<f32>,
    mode: Option<PyVtgMode>,
}

#[pymethods]
impl PyVtg {
    #[new]
    fn new(
        talker: Option<&[u8]>,
        course_true_deg: Option<f32>,
        course_magnetic_deg: Option<f32>,
        speed_knots: Option<f32>,
        speed_kmh: Option<f32>,
        mode: Option<PyVtgMode>,
    ) -> PyResult<Self> {
        let talker = normalize_talker(talker)?;
        Ok(Self {
            talker,
            course_true_deg,
            course_magnetic_deg,
            speed_knots,
            speed_kmh,
            mode,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn course_true_deg(&self) -> Option<f32> {
        self.course_true_deg
    }
    #[getter]
    fn course_magnetic_deg(&self) -> Option<f32> {
        self.course_magnetic_deg
    }
    #[getter]
    fn speed_knots(&self) -> Option<f32> {
        self.speed_knots
    }
    #[getter]
    fn speed_kmh(&self) -> Option<f32> {
        self.speed_kmh
    }
    #[getter]
    fn mode(&self) -> Option<PyVtgMode> {
        self.mode
    }

    fn __repr__(&self) -> String {
        format!(
            "Vtg(talker={}, course_true={:?}, speed_knots={:?}, mode={:?})",
            repr_talker(self.talker),
            self.course_true_deg,
            self.speed_knots,
            self.mode,
        )
    }
}

impl From<VtgData> for PyVtg {
    fn from(d: VtgData) -> Self {
        Self {
            talker: d.talker,
            course_true_deg: d.course_true_deg,
            course_magnetic_deg: d.course_magnetic_deg,
            speed_knots: d.speed_knots,
            speed_kmh: d.speed_kmh,
            mode: d.mode.map(PyVtgMode::from),
        }
    }
}

// ---------- Hdt ----------

/// Frozen `$__HDT` message (mirrors `HdtData`).
#[pyclass(name = "Hdt", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyHdt {
    talker: Option<[u8; 2]>,
    heading_true_deg: Option<f32>,
}

#[pymethods]
impl PyHdt {
    #[new]
    fn new(talker: Option<&[u8]>, heading_true_deg: Option<f32>) -> PyResult<Self> {
        let talker = normalize_talker(talker)?;
        Ok(Self {
            talker,
            heading_true_deg,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn heading_true_deg(&self) -> Option<f32> {
        self.heading_true_deg
    }

    fn __repr__(&self) -> String {
        format!(
            "Hdt(talker={}, heading_true_deg={:?})",
            repr_talker(self.talker),
            self.heading_true_deg,
        )
    }
}

impl From<HdtData> for PyHdt {
    fn from(d: HdtData) -> Self {
        Self {
            talker: d.talker,
            heading_true_deg: d.heading_true_deg,
        }
    }
}

// ---------- Unknown ----------

/// Frozen unknown-sentence marker — carries talker + `sentence_type` only.
#[pyclass(name = "Unknown", frozen, eq, hash, module = "marlin.nmea")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyUnknown {
    talker: Option<[u8; 2]>,
    sentence_type: String,
}

#[pymethods]
impl PyUnknown {
    #[new]
    fn new(talker: Option<&[u8]>, sentence_type: String) -> PyResult<Self> {
        let talker = normalize_talker(talker)?;
        Ok(Self {
            talker,
            sentence_type,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn sentence_type(&self) -> &str {
        &self.sentence_type
    }

    fn __repr__(&self) -> String {
        format!(
            "Unknown(talker={}, sentence_type={:?})",
            repr_talker(self.talker),
            self.sentence_type,
        )
    }
}

// ---------- PsxnSlot / PrdidDialect enums ----------

/// Meaning of one of the six `dataN` slots in a PSXN sentence
/// (mirrors `PsxnSlot`).
#[pyclass(name = "PsxnSlot", eq, eq_int, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyPsxnSlot {
    #[pyo3(name = "ROLL")]
    Roll = 0,
    #[pyo3(name = "PITCH")]
    Pitch = 1,
    #[pyo3(name = "HEAVE")]
    Heave = 2,
    #[pyo3(name = "ROLL_SINE_ENCODED")]
    RollSineEncoded = 3,
    #[pyo3(name = "PITCH_SINE_ENCODED")]
    PitchSineEncoded = 4,
    #[pyo3(name = "IGNORED")]
    Ignored = 5,
}

impl From<RustPsxnSlot> for PyPsxnSlot {
    // Wildcard collapses future `#[non_exhaustive]` variants onto
    // `Ignored`; silence `match_same_arms` for the `Ignored => Ignored` +
    // `_ => Ignored` pair — same pattern as PyGgaFixQuality.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustPsxnSlot) -> Self {
        match v {
            RustPsxnSlot::Roll => Self::Roll,
            RustPsxnSlot::Pitch => Self::Pitch,
            RustPsxnSlot::Heave => Self::Heave,
            RustPsxnSlot::RollSineEncoded => Self::RollSineEncoded,
            RustPsxnSlot::PitchSineEncoded => Self::PitchSineEncoded,
            RustPsxnSlot::Ignored => Self::Ignored,
            _ => Self::Ignored,
        }
    }
}

impl From<PyPsxnSlot> for RustPsxnSlot {
    fn from(v: PyPsxnSlot) -> Self {
        match v {
            PyPsxnSlot::Roll => Self::Roll,
            PyPsxnSlot::Pitch => Self::Pitch,
            PyPsxnSlot::Heave => Self::Heave,
            PyPsxnSlot::RollSineEncoded => Self::RollSineEncoded,
            PyPsxnSlot::PitchSineEncoded => Self::PitchSineEncoded,
            PyPsxnSlot::Ignored => Self::Ignored,
        }
    }
}

/// Runtime selector for PRDID field ordering (mirrors `PrdidDialect`).
#[pyclass(name = "PrdidDialect", eq, eq_int, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyPrdidDialect {
    #[pyo3(name = "UNKNOWN")]
    Unknown = 0,
    #[pyo3(name = "PITCH_ROLL_HEADING")]
    PitchRollHeading = 1,
    #[pyo3(name = "ROLL_PITCH_HEADING")]
    RollPitchHeading = 2,
}

impl From<RustPrdidDialect> for PyPrdidDialect {
    // Wildcard collapses future `#[non_exhaustive]` variants onto
    // `Unknown`; silence `match_same_arms` for the pair — same pattern
    // as PyGgaFixQuality.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustPrdidDialect) -> Self {
        match v {
            RustPrdidDialect::Unknown => Self::Unknown,
            RustPrdidDialect::PitchRollHeading => Self::PitchRollHeading,
            RustPrdidDialect::RollPitchHeading => Self::RollPitchHeading,
            _ => Self::Unknown,
        }
    }
}

impl From<PyPrdidDialect> for RustPrdidDialect {
    fn from(v: PyPrdidDialect) -> Self {
        match v {
            PyPrdidDialect::Unknown => Self::Unknown,
            PyPrdidDialect::PitchRollHeading => Self::PitchRollHeading,
            PyPrdidDialect::RollPitchHeading => Self::RollPitchHeading,
        }
    }
}

// ---------- PsxnLayout / DecodeOptions ----------

/// Frozen PSXN layout descriptor (mirrors `PsxnLayout`).
///
/// Construct via the `from_str()` staticmethod with a legacy layout
/// string like `"rphx"` or `"rphx1"` (case-insensitive).
#[pyclass(name = "PsxnLayout", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPsxnLayout {
    pub(crate) inner: RustPsxnLayout,
}

#[pymethods]
impl PyPsxnLayout {
    /// Parse a legacy layout string. Raises `ValueError` on unrecognised
    /// characters or more than 6 slots.
    //
    // The `from_str` name mirrors the Rust `FromStr` impl for Python-side
    // symmetry; silence `should_implement_trait` since the trait is
    // inherently not available in Python.
    #[staticmethod]
    #[allow(clippy::should_implement_trait)]
    fn from_str(s: &str) -> PyResult<Self> {
        RustPsxnLayout::from_str(s)
            .map(|inner| Self { inner })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        // Reconstruct the legacy layout string from slot array + raw_radians flag.
        let mut layout_str = String::with_capacity(7);
        for slot in &self.inner.slots {
            let ch = match slot {
                RustPsxnSlot::Roll => 'r',
                RustPsxnSlot::Pitch => 'p',
                RustPsxnSlot::Heave => 'h',
                RustPsxnSlot::RollSineEncoded => 's',
                RustPsxnSlot::PitchSineEncoded => 'q',
                // Ignored + any future non-exhaustive variants map to 'x'
                _ => 'x',
            };
            layout_str.push(ch);
        }
        // Trim trailing 'x' slots for readability (e.g. "rphx" not "rphxxx").
        let trimmed = layout_str.trim_end_matches('x');
        if self.inner.raw_radians {
            format!("PsxnLayout(\"{trimmed}1\")")
        } else {
            format!("PsxnLayout(\"{trimmed}\")")
        }
    }
}

/// Frozen runtime configuration for ambiguous decodings (mirrors
/// `DecodeOptions`). Construct with `DecodeOptions()` and chain
/// `.with_psxn_layout(...)` / `.with_prdid_dialect(...)`.
#[pyclass(name = "DecodeOptions", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug, Default)]
pub struct PyDecodeOptions {
    pub(crate) inner: RustDecodeOptions,
}

#[pymethods]
impl PyDecodeOptions {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    fn with_psxn_layout(&self, layout: PyPsxnLayout) -> Self {
        Self {
            inner: self.inner.clone().with_psxn_layout(layout.inner),
        }
    }

    fn with_prdid_dialect(&self, dialect: PyPrdidDialect) -> Self {
        Self {
            inner: self.inner.clone().with_prdid_dialect(dialect.into()),
        }
    }

    fn __repr__(&self) -> String {
        let layout_repr = PyPsxnLayout { inner: self.inner.psxn_layout }.__repr__();
        let dialect = match self.inner.prdid_dialect {
            RustPrdidDialect::PitchRollHeading => "PitchRollHeading",
            RustPrdidDialect::RollPitchHeading => "RollPitchHeading",
            // Unknown + any future variants: render as "Unknown"
            _ => "Unknown",
        };
        format!("DecodeOptions(psxn_layout={layout_repr}, prdid_dialect={dialect})")
    }
}

// ---------- Psxn ----------

/// Frozen `$PSXN` payload (mirrors `PsxnData`).
///
/// PSXN is proprietary — there is no talker. `PsxnLayout` describes
/// how the six on-wire slots decode into these five motion quantities;
/// the output shape is fixed regardless of layout.
#[pyclass(name = "Psxn", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPsxn {
    id: Option<u16>,
    token: Option<Vec<u8>>,
    roll_deg: Option<f32>,
    pitch_deg: Option<f32>,
    heave_m: Option<f32>,
}

#[pymethods]
impl PyPsxn {
    #[new]
    #[pyo3(signature = (id = None, token = None, roll_deg = None, pitch_deg = None, heave_m = None))]
    fn new(
        id: Option<u16>,
        token: Option<Vec<u8>>,
        roll_deg: Option<f32>,
        pitch_deg: Option<f32>,
        heave_m: Option<f32>,
    ) -> Self {
        Self {
            id,
            token,
            roll_deg,
            pitch_deg,
            heave_m,
        }
    }

    #[getter]
    fn id(&self) -> Option<u16> {
        self.id
    }
    #[getter]
    fn token<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.token.as_deref().map(|t| PyBytes::new(py, t))
    }
    #[getter]
    fn roll_deg(&self) -> Option<f32> {
        self.roll_deg
    }
    #[getter]
    fn pitch_deg(&self) -> Option<f32> {
        self.pitch_deg
    }
    #[getter]
    fn heave_m(&self) -> Option<f32> {
        self.heave_m
    }

    fn __repr__(&self) -> String {
        format!(
            "Psxn(id={:?}, roll={:?}, pitch={:?}, heave={:?})",
            self.id, self.roll_deg, self.pitch_deg, self.heave_m
        )
    }
}

impl From<PsxnData> for PyPsxn {
    fn from(d: PsxnData) -> Self {
        Self {
            id: d.id,
            token: d.token,
            roll_deg: d.roll_deg,
            pitch_deg: d.pitch_deg,
            heave_m: d.heave_m,
        }
    }
}

// ---------- Prdid (tagged union) ----------

/// Frozen `$PRDID` body for the `pitch, roll, heading` dialect
/// (mirrors `PrdidPitchRollHeading`).
// Field names mirror the Rust struct and are Python-visible via getters;
// `_deg` conveys the unit and must stay.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "PrdidPitchRollHeading", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdidPitchRollHeading {
    pitch_deg: Option<f32>,
    roll_deg: Option<f32>,
    heading_deg: Option<f32>,
}

#[pymethods]
impl PyPrdidPitchRollHeading {
    #[new]
    #[pyo3(signature = (pitch_deg = None, roll_deg = None, heading_deg = None))]
    fn new(
        pitch_deg: Option<f32>,
        roll_deg: Option<f32>,
        heading_deg: Option<f32>,
    ) -> Self {
        Self {
            pitch_deg,
            roll_deg,
            heading_deg,
        }
    }

    #[getter]
    fn pitch_deg(&self) -> Option<f32> {
        self.pitch_deg
    }
    #[getter]
    fn roll_deg(&self) -> Option<f32> {
        self.roll_deg
    }
    #[getter]
    fn heading_deg(&self) -> Option<f32> {
        self.heading_deg
    }
}

impl From<RustPrdidPitchRollHeading> for PyPrdidPitchRollHeading {
    fn from(d: RustPrdidPitchRollHeading) -> Self {
        Self {
            pitch_deg: d.pitch_deg,
            roll_deg: d.roll_deg,
            heading_deg: d.heading_deg,
        }
    }
}

/// Frozen `$PRDID` body for the `roll, pitch, heading` dialect
/// (mirrors `PrdidRollPitchHeading`).
// Field names mirror the Rust struct and are Python-visible via getters;
// `_deg` conveys the unit and must stay.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "PrdidRollPitchHeading", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdidRollPitchHeading {
    roll_deg: Option<f32>,
    pitch_deg: Option<f32>,
    heading_deg: Option<f32>,
}

#[pymethods]
impl PyPrdidRollPitchHeading {
    #[new]
    #[pyo3(signature = (roll_deg = None, pitch_deg = None, heading_deg = None))]
    fn new(
        roll_deg: Option<f32>,
        pitch_deg: Option<f32>,
        heading_deg: Option<f32>,
    ) -> Self {
        Self {
            roll_deg,
            pitch_deg,
            heading_deg,
        }
    }

    #[getter]
    fn roll_deg(&self) -> Option<f32> {
        self.roll_deg
    }
    #[getter]
    fn pitch_deg(&self) -> Option<f32> {
        self.pitch_deg
    }
    #[getter]
    fn heading_deg(&self) -> Option<f32> {
        self.heading_deg
    }
}

impl From<RustPrdidRollPitchHeading> for PyPrdidRollPitchHeading {
    fn from(d: RustPrdidRollPitchHeading) -> Self {
        Self {
            roll_deg: d.roll_deg,
            pitch_deg: d.pitch_deg,
            heading_deg: d.heading_deg,
        }
    }
}

/// Frozen `$PRDID` raw-bytes body (mirrors `PrdidData::Raw`).
///
/// Emitted when no dialect is configured (default
/// `PrdidDialect.UNKNOWN`). The `fields` getter returns a tuple of
/// `bytes`.
#[pyclass(name = "PrdidRaw", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdidRaw {
    fields: Vec<Vec<u8>>,
}

#[pymethods]
impl PyPrdidRaw {
    #[new]
    fn new(fields: Vec<Vec<u8>>) -> Self {
        Self { fields }
    }

    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        let items: Vec<Bound<'py, PyBytes>> =
            self.fields.iter().map(|f| PyBytes::new(py, f)).collect();
        pyo3::types::PyTuple::new(py, items)
    }
}

#[derive(Clone, Debug)]
enum PrdidVariant {
    PitchRollHeading(PyPrdidPitchRollHeading),
    RollPitchHeading(PyPrdidRollPitchHeading),
    Raw(PyPrdidRaw),
}

/// Frozen `$PRDID` payload wrapper (tagged union over `PrdidData`).
///
/// Construct via the `pitch_roll_heading`, `roll_pitch_heading`, or
/// `raw` staticmethod factories. The `.variant` property returns a
/// stable snake-case tag; `.body` returns the typed inner wrapper.
#[pyclass(name = "Prdid", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdid {
    variant: PrdidVariant,
}

#[pymethods]
impl PyPrdid {
    #[staticmethod]
    #[pyo3(signature = (pitch_deg = None, roll_deg = None, heading_deg = None))]
    fn pitch_roll_heading(
        pitch_deg: Option<f32>,
        roll_deg: Option<f32>,
        heading_deg: Option<f32>,
    ) -> Self {
        Self {
            variant: PrdidVariant::PitchRollHeading(PyPrdidPitchRollHeading::new(
                pitch_deg,
                roll_deg,
                heading_deg,
            )),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (roll_deg = None, pitch_deg = None, heading_deg = None))]
    fn roll_pitch_heading(
        roll_deg: Option<f32>,
        pitch_deg: Option<f32>,
        heading_deg: Option<f32>,
    ) -> Self {
        Self {
            variant: PrdidVariant::RollPitchHeading(PyPrdidRollPitchHeading::new(
                roll_deg,
                pitch_deg,
                heading_deg,
            )),
        }
    }

    #[staticmethod]
    fn raw(fields: Vec<Vec<u8>>) -> Self {
        Self {
            variant: PrdidVariant::Raw(PyPrdidRaw::new(fields)),
        }
    }

    #[getter]
    fn variant(&self) -> &'static str {
        match self.variant {
            PrdidVariant::PitchRollHeading(_) => "pitch_roll_heading",
            PrdidVariant::RollPitchHeading(_) => "roll_pitch_heading",
            PrdidVariant::Raw(_) => "raw",
        }
    }

    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObject;
        match &self.variant {
            PrdidVariant::PitchRollHeading(b) => {
                Ok(b.clone().into_pyobject(py)?.into_any().unbind())
            }
            PrdidVariant::RollPitchHeading(b) => {
                Ok(b.clone().into_pyobject(py)?.into_any().unbind())
            }
            PrdidVariant::Raw(b) => Ok(b.clone().into_pyobject(py)?.into_any().unbind()),
        }
    }

    fn __repr__(&self) -> String {
        format!("Prdid(variant={})", self.variant())
    }
}

impl From<PrdidData> for PyPrdid {
    fn from(d: PrdidData) -> Self {
        let variant = match d {
            PrdidData::PitchRollHeading(inner) => PrdidVariant::PitchRollHeading(inner.into()),
            PrdidData::RollPitchHeading(inner) => PrdidVariant::RollPitchHeading(inner.into()),
            PrdidData::Raw { fields } => PrdidVariant::Raw(PyPrdidRaw { fields }),
            // `#[non_exhaustive]` wildcard: collapse any future variant
            // onto an empty-field Raw so older Python bindings don't
            // panic on newer Rust wire types.
            _ => PrdidVariant::Raw(PyPrdidRaw { fields: Vec::new() }),
        };
        Self { variant }
    }
}

// ---------- Helpers ----------

#[allow(clippy::indexing_slicing)] // length-checked by the `if t.len() == 2` guard
fn normalize_talker(talker: Option<&[u8]>) -> PyResult<Option<[u8; 2]>> {
    match talker {
        None => Ok(None),
        Some(t) if t.len() == 2 => Ok(Some([t[0], t[1]])),
        Some(_) => Err(pyo3::exceptions::PyValueError::new_err(
            "talker must be exactly 2 bytes or None",
        )),
    }
}

fn repr_talker(talker: Option<[u8; 2]>) -> String {
    match talker {
        Some(t) => format!("b{:?}", core::str::from_utf8(&t).unwrap_or("??")),
        None => "None".to_string(),
    }
}

// ---------- Nmea0183Parser ----------

/// Internal mode-enum so the HRTB generic on `Nmea0183Parser<P>` doesn't
/// infect the Python class surface — the `Parser` runtime-dispatch enum
/// from marlin-nmea-0183 solves the same problem, but constructing it
/// directly gives us matching shape without bringing a second enum in.
///
/// Do **not** "simplify" this into a `Nmea0183Parser<P>` field with a
/// generic bound. The HRTB nested-call trap (see the repo CLAUDE.md)
/// means `&mut self` methods bounded by
/// `for<'a> SentenceSource<Item<'a> = RawSentence<'a>>` can't delegate to
/// other such methods — rustc emits "P does not live long enough". The
/// mode-enum dodges the trap by matching on a concrete parser in each arm.
#[derive(Debug)]
enum Nmea0183Inner {
    OneShot(RustNmea0183<OneShot>),
    Streaming(RustNmea0183<Streaming>),
}

/// Typed NMEA 0183 parser that wraps an envelope parser and exposes
/// [`Nmea0183Message`] variants. Construct via `streaming()` or
/// `one_shot()` staticmethods; both accept optional `DecodeOptions`.
#[pyclass(name = "Nmea0183Parser", module = "marlin.nmea")]
pub struct PyNmea0183Parser {
    inner: Nmea0183Inner,
}

#[pymethods]
impl PyNmea0183Parser {
    #[staticmethod]
    #[pyo3(signature = (options = None))]
    fn one_shot(options: Option<PyDecodeOptions>) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        Self {
            inner: Nmea0183Inner::OneShot(RustNmea0183::with_options(OneShot::new(), opts)),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (options = None, max_size = DEFAULT_MAX_SIZE))]
    fn streaming(options: Option<PyDecodeOptions>, max_size: usize) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        Self {
            inner: Nmea0183Inner::Streaming(RustNmea0183::with_options(
                Streaming::with_capacity(max_size),
                opts,
            )),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        match &mut self.inner {
            Nmea0183Inner::OneShot(p) => p.feed(data),
            Nmea0183Inner::Streaming(p) => p.feed(data),
        }
    }

    /// Return the next decoded message, `None` if no complete sentence is
    /// buffered, or raise `EnvelopeError` / `DecodeError` on failure.
    fn next_message(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        // Convert the borrowed Nmea0183Message<'_> into an owned IR before
        // dropping the &mut borrow on `self.inner`. Otherwise the borrow
        // checker flags the subsequent PyO3 construction as overlapping
        // with the `next_message()` borrow.
        let result = match &mut self.inner {
            Nmea0183Inner::OneShot(p) => p
                .next_message()
                .map(|r| r.map(owned_message_from_borrowed)),
            Nmea0183Inner::Streaming(p) => p
                .next_message()
                .map(|r| r.map(owned_message_from_borrowed)),
        };
        match result {
            None => Ok(None),
            Some(Ok(owned)) => Ok(Some(owned.into_pyany(py)?)),
            Some(Err(e)) => Err(convert_nmea_err(py, e)),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyNmeaIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(
            py,
            PyNmeaIterator {
                parser,
                strict: false,
            },
        )
    }

    /// Explicit iterator with strict/lenient switch.
    #[pyo3(signature = (strict = false))]
    fn iter(slf: PyRef<'_, Self>, strict: bool) -> PyResult<Py<PyNmeaIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyNmeaIterator { parser, strict })
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

/// Owned mirror of `Nmea0183Message<'a>` — lifetime-stripped so we can
/// hand the result back across `PyO3` borrows without holding the
/// underlying envelope buffer borrow open.
enum OwnedMessage {
    Gga(GgaData),
    Vtg(VtgData),
    Hdt(HdtData),
    Psxn(PsxnData),
    Prdid(PrdidData),
    Unknown {
        talker: Option<[u8; 2]>,
        sentence_type: String,
    },
    // `#[non_exhaustive]` wildcard — future variants the Python layer
    // doesn't know about yet surface as a ValueError at materialization
    // time rather than silently panicking.
    Unsupported,
}

fn owned_message_from_borrowed(msg: Nmea0183Message<'_>) -> OwnedMessage {
    match msg {
        Nmea0183Message::Gga(d) => OwnedMessage::Gga(d),
        Nmea0183Message::Vtg(d) => OwnedMessage::Vtg(d),
        Nmea0183Message::Hdt(d) => OwnedMessage::Hdt(d),
        Nmea0183Message::Psxn(d) => OwnedMessage::Psxn(d),
        Nmea0183Message::Prdid(d) => OwnedMessage::Prdid(d),
        Nmea0183Message::Unknown(raw) => OwnedMessage::Unknown {
            talker: raw.talker,
            sentence_type: raw.sentence_type.to_string(),
        },
        _ => OwnedMessage::Unsupported,
    }
}

impl OwnedMessage {
    fn into_pyany(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match self {
            Self::Gga(d) => Py::new(py, PyGga::from(d))?.into_any(),
            Self::Vtg(d) => Py::new(py, PyVtg::from(d))?.into_any(),
            Self::Hdt(d) => Py::new(py, PyHdt::from(d))?.into_any(),
            Self::Psxn(d) => Py::new(py, PyPsxn::from(d))?.into_any(),
            Self::Prdid(d) => Py::new(py, PyPrdid::from(d))?.into_any(),
            Self::Unknown {
                talker,
                sentence_type,
            } => Py::new(
                py,
                PyUnknown {
                    talker,
                    sentence_type,
                },
            )?
            .into_any(),
            Self::Unsupported => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "marlin Python bindings encountered an unsupported Nmea0183Message variant — bindings need updating",
                ));
            }
        })
    }
}

fn convert_nmea_err(py: Python<'_>, e: Nmea0183Error) -> PyErr {
    match e {
        Nmea0183Error::Envelope(inner) => envelope_err(py, inner),
        Nmea0183Error::Decode(inner) => decode_err(inner),
        // `#[non_exhaustive]` wildcard — future variants surface as
        // DecodeError(display) so downstream code still catches them.
        other => crate::errors::DecodeError::new_err(other.to_string()),
    }
}

/// Iterator wrapper; swallows errors by default, raises in strict mode.
#[pyclass(module = "marlin.nmea")]
pub struct PyNmeaIterator {
    parser: Py<PyNmea0183Parser>,
    strict: bool,
}

#[pymethods]
impl PyNmeaIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        loop {
            let result = {
                let mut borrow = self.parser.borrow_mut(py);
                borrow.next_message(py)
            };
            match result {
                Ok(Some(obj)) => return Ok(obj),
                Ok(None) => return Err(pyo3::exceptions::PyStopIteration::new_err(())),
                Err(e) => {
                    if self.strict {
                        return Err(e);
                    }
                    // lenient — swallow and loop for the next message.
                }
            }
        }
    }
}

// ---------- Per-sentence decode extension points ----------
//
// Each #[pyfunction] wraps the Rust free function of the same name.
// `decode` / `decode_with` route through the OwnedMessage IR so the
// Nmea0183Message<'a> borrow on `raw` drops before the Python object
// is materialized (same trick PyNmea0183Parser::next_message uses).

#[pyfunction]
#[pyo3(name = "decode")]
fn py_decode(py: Python<'_>, raw: &PyRawSentence) -> PyResult<Py<PyAny>> {
    let rust_raw = raw.to_rust();
    match rust_decode(&rust_raw) {
        Ok(msg) => owned_message_from_borrowed(msg).into_pyany(py),
        Err(e) => Err(decode_err(e)),
    }
}

#[pyfunction]
#[pyo3(name = "decode_with")]
fn py_decode_with(
    py: Python<'_>,
    raw: &PyRawSentence,
    options: &PyDecodeOptions,
) -> PyResult<Py<PyAny>> {
    let rust_raw = raw.to_rust();
    match rust_decode_with(&rust_raw, &options.inner) {
        Ok(msg) => owned_message_from_borrowed(msg).into_pyany(py),
        Err(e) => Err(decode_err(e)),
    }
}

#[pyfunction]
#[pyo3(name = "decode_gga")]
fn py_decode_gga(raw: &PyRawSentence) -> PyResult<PyGga> {
    let rust_raw = raw.to_rust();
    rust_decode_gga(&rust_raw)
        .map(PyGga::from)
        .map_err(decode_err)
}

#[pyfunction]
#[pyo3(name = "decode_vtg")]
fn py_decode_vtg(raw: &PyRawSentence) -> PyResult<PyVtg> {
    let rust_raw = raw.to_rust();
    rust_decode_vtg(&rust_raw)
        .map(PyVtg::from)
        .map_err(decode_err)
}

#[pyfunction]
#[pyo3(name = "decode_hdt")]
fn py_decode_hdt(raw: &PyRawSentence) -> PyResult<PyHdt> {
    let rust_raw = raw.to_rust();
    rust_decode_hdt(&rust_raw)
        .map(PyHdt::from)
        .map_err(decode_err)
}

#[pyfunction]
#[pyo3(name = "decode_psxn")]
fn py_decode_psxn(raw: &PyRawSentence, layout: &PyPsxnLayout) -> PyResult<PyPsxn> {
    let rust_raw = raw.to_rust();
    rust_decode_psxn(&rust_raw, &layout.inner)
        .map(PyPsxn::from)
        .map_err(decode_err)
}

#[pyfunction]
#[pyo3(name = "decode_prdid")]
fn py_decode_prdid(raw: &PyRawSentence, dialect: PyPrdidDialect) -> PyResult<PyPrdid> {
    let rust_raw = raw.to_rust();
    rust_decode_prdid(&rust_raw, dialect.into())
        .map(PyPrdid::from)
        .map_err(decode_err)
}

// ---------- Registration ----------

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "nmea")?;
    m.add_class::<PyGgaFixQuality>()?;
    m.add_class::<PyVtgMode>()?;
    m.add_class::<PyPsxnSlot>()?;
    m.add_class::<PyPrdidDialect>()?;
    m.add_class::<PyUtcTime>()?;
    m.add_class::<PyPsxnLayout>()?;
    m.add_class::<PyDecodeOptions>()?;
    m.add_class::<PyGga>()?;
    m.add_class::<PyVtg>()?;
    m.add_class::<PyHdt>()?;
    m.add_class::<PyPsxn>()?;
    m.add_class::<PyPrdidPitchRollHeading>()?;
    m.add_class::<PyPrdidRollPitchHeading>()?;
    m.add_class::<PyPrdidRaw>()?;
    m.add_class::<PyPrdid>()?;
    m.add_class::<PyUnknown>()?;
    m.add_class::<PyNmea0183Parser>()?;
    m.add_class::<PyNmeaIterator>()?;
    m.add_function(wrap_pyfunction!(py_decode, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_with, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_gga, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_vtg, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_hdt, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_psxn, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_prdid, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
