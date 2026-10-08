//! Python wrappers for `marlin-nmea-0183`.
//!
//! Each typed message variant is its own `#[pyclass]`. The `Nmea0183Message`
//! Rust enum has no single Python class — variants are a structural union.
//!
//! Every field the wire can leave empty, or fill with text the decoder
//! cannot read, is a `marlin.field.FieldState`: a getter returns the
//! variant class, and a constructor accepts a `FieldState`, a bare value
//! (coerced to `FieldState.Value`) or `None` (coerced to
//! `FieldState.NotAvailable()`).

use core::str::FromStr;

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use pyo3::wrap_pyfunction;

use marlin_field::FieldState;
use marlin_nmea_0183::{
    decode as rust_decode, decode_gga as rust_decode_gga, decode_gll as rust_decode_gll,
    decode_hdg as rust_decode_hdg, decode_hdt as rust_decode_hdt,
    decode_prdid as rust_decode_prdid, decode_psxn as rust_decode_psxn,
    decode_rmc as rust_decode_rmc, decode_tll as rust_decode_tll, decode_ttm as rust_decode_ttm,
    decode_vtg as rust_decode_vtg, decode_with as rust_decode_with,
    AcquisitionType as RustAcquisitionType, AngleReference as RustAngleReference,
    DataStatus as RustDataStatus, DecodeOptions as RustDecodeOptions,
    DistanceUnits as RustDistanceUnits, GgaData, GgaFixQuality as RustGgaFixQuality, GllData,
    HdgData, HdtData, Nmea0183Error, Nmea0183Message, Nmea0183Parser as RustNmea0183, PrdidData,
    PrdidDialect as RustPrdidDialect, PrdidPitchRollHeading as RustPrdidPitchRollHeading,
    PrdidRollPitchHeading as RustPrdidRollPitchHeading, PsxnData, PsxnLayout as RustPsxnLayout,
    PsxnSlot as RustPsxnSlot, RmcData, RmcNavStatus as RustRmcNavStatus,
    TargetStatus as RustTargetStatus, TllData, TtmData, UtcDate as RustUtcDate,
    UtcTime as RustUtcTime, VtgData, VtgMode as RustVtgMode,
};
use marlin_nmea_envelope::Parser;

use crate::envelope::{PyRawSentence, DEFAULT_MAX_SIZE};
use crate::errors::{decode_err, envelope_err};
use crate::field::{from_py, repr_state, to_py, PyFieldState};

// ---------- Enums ----------

/// GPS fix quality indicator (binding class for `GgaFixQuality`).
///
/// The sender's own statement of fix quality; `NO_FIX` is a value the
/// sender reported. A digit the standard leaves undefined decodes to
/// `FieldState.Invalid(digit)` on the message instead of a member here.
#[pyclass(
    name = "GgaFixQuality",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyGgaFixQuality {
    #[pyo3(name = "NO_FIX")]
    NoFix = 0,
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

impl TryFrom<RustGgaFixQuality> for PyGgaFixQuality {
    type Error = PyErr;

    fn try_from(v: RustGgaFixQuality) -> PyResult<Self> {
        Ok(match v {
            RustGgaFixQuality::NoFix => Self::NoFix,
            RustGgaFixQuality::GpsFix => Self::GpsFix,
            RustGgaFixQuality::DgpsFix => Self::DgpsFix,
            RustGgaFixQuality::PpsFix => Self::PpsFix,
            RustGgaFixQuality::RtkFixed => Self::RtkFixed,
            RustGgaFixQuality::RtkFloat => Self::RtkFloat,
            RustGgaFixQuality::DeadReckoning => Self::DeadReckoning,
            RustGgaFixQuality::ManualInput => Self::ManualInput,
            RustGgaFixQuality::Simulator => Self::Simulator,
            other => return Err(unsupported_variant("GgaFixQuality", &other)),
        })
    }
}

/// VTG mode indicator (binding class for `VtgMode`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(name = "VtgMode", frozen, eq, eq_int, hash, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl TryFrom<RustVtgMode> for PyVtgMode {
    type Error = PyErr;

    fn try_from(v: RustVtgMode) -> PyResult<Self> {
        Ok(match v {
            RustVtgMode::Autonomous => Self::Autonomous,
            RustVtgMode::Differential => Self::Differential,
            RustVtgMode::Estimated => Self::Estimated,
            RustVtgMode::NotValid => Self::NotValid,
            RustVtgMode::Manual => Self::Manual,
            RustVtgMode::Simulator => Self::Simulator,
            other => return Err(unsupported_variant("VtgMode", &other)),
        })
    }
}

/// A/V validity status carried by RMC and GLL (binding class for `DataStatus`).
///
/// A status field: it qualifies the other fields of its sentence and
/// does not change their state. An empty byte is
/// `FieldState.NotAvailable()` on the message; an unnamed byte is
/// `FieldState.Invalid(byte)`.
#[pyclass(name = "DataStatus", frozen, eq, eq_int, hash, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyDataStatus {
    #[pyo3(name = "ACTIVE")]
    Active = 0,
    #[pyo3(name = "VOID")]
    Void = 1,
}

impl TryFrom<RustDataStatus> for PyDataStatus {
    type Error = PyErr;

    fn try_from(v: RustDataStatus) -> PyResult<Self> {
        Ok(match v {
            RustDataStatus::Active => Self::Active,
            RustDataStatus::Void => Self::Void,
            other => return Err(unsupported_variant("DataStatus", &other)),
        })
    }
}

/// RMC nav-status indicator (NMEA 4.10+, binding class for `RmcNavStatus`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(
    name = "RmcNavStatus",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyRmcNavStatus {
    #[pyo3(name = "SAFE")]
    Safe = 0,
    #[pyo3(name = "CAUTION")]
    Caution = 1,
    #[pyo3(name = "UNSAFE")]
    Unsafe = 2,
    #[pyo3(name = "NOT_VALID")]
    NotValid = 3,
}

impl TryFrom<RustRmcNavStatus> for PyRmcNavStatus {
    type Error = PyErr;

    fn try_from(v: RustRmcNavStatus) -> PyResult<Self> {
        Ok(match v {
            RustRmcNavStatus::Safe => Self::Safe,
            RustRmcNavStatus::Caution => Self::Caution,
            RustRmcNavStatus::Unsafe => Self::Unsafe,
            RustRmcNavStatus::NotValid => Self::NotValid,
            other => return Err(unsupported_variant("RmcNavStatus", &other)),
        })
    }
}

/// Radar target tracking state (binding class for `TargetStatus`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(
    name = "TargetStatus",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyTargetStatus {
    #[pyo3(name = "LOST")]
    Lost = 0,
    #[pyo3(name = "QUERY")]
    Query = 1,
    #[pyo3(name = "TRACKING")]
    Tracking = 2,
}

impl TryFrom<RustTargetStatus> for PyTargetStatus {
    type Error = PyErr;

    fn try_from(v: RustTargetStatus) -> PyResult<Self> {
        Ok(match v {
            RustTargetStatus::Lost => Self::Lost,
            RustTargetStatus::Query => Self::Query,
            RustTargetStatus::Tracking => Self::Tracking,
            other => return Err(unsupported_variant("TargetStatus", &other)),
        })
    }
}

/// Bearing/course reference (binding class for `AngleReference`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(
    name = "AngleReference",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAngleReference {
    #[pyo3(name = "TRUE")]
    True = 0,
    #[pyo3(name = "RELATIVE")]
    Relative = 1,
}

impl TryFrom<RustAngleReference> for PyAngleReference {
    type Error = PyErr;

    fn try_from(v: RustAngleReference) -> PyResult<Self> {
        Ok(match v {
            RustAngleReference::True => Self::True,
            RustAngleReference::Relative => Self::Relative,
            other => return Err(unsupported_variant("AngleReference", &other)),
        })
    }
}

/// Speed/distance units (binding class for `DistanceUnits`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(
    name = "DistanceUnits",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyDistanceUnits {
    #[pyo3(name = "NAUTICAL")]
    Nautical = 0,
    #[pyo3(name = "KILOMETERS")]
    Kilometers = 1,
    #[pyo3(name = "STATUTE")]
    Statute = 2,
}

impl TryFrom<RustDistanceUnits> for PyDistanceUnits {
    type Error = PyErr;

    fn try_from(v: RustDistanceUnits) -> PyResult<Self> {
        Ok(match v {
            RustDistanceUnits::Nautical => Self::Nautical,
            RustDistanceUnits::Kilometers => Self::Kilometers,
            RustDistanceUnits::Statute => Self::Statute,
            other => return Err(unsupported_variant("DistanceUnits", &other)),
        })
    }
}

/// Target acquisition type (binding class for `AcquisitionType`).
///
/// An unnamed letter decodes to `FieldState.Invalid(byte)` on the
/// message instead of a member here.
#[pyclass(
    name = "AcquisitionType",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAcquisitionType {
    #[pyo3(name = "AUTOMATIC")]
    Automatic = 0,
    #[pyo3(name = "MANUAL")]
    Manual = 1,
    #[pyo3(name = "REPORTED")]
    Reported = 2,
}

impl TryFrom<RustAcquisitionType> for PyAcquisitionType {
    type Error = PyErr;

    fn try_from(v: RustAcquisitionType) -> PyResult<Self> {
        Ok(match v {
            RustAcquisitionType::Automatic => Self::Automatic,
            RustAcquisitionType::Manual => Self::Manual,
            RustAcquisitionType::Reported => Self::Reported,
            other => return Err(unsupported_variant("AcquisitionType", &other)),
        })
    }
}

// ---------- UtcTime ----------

/// Frozen UTC time-of-day value (binding class for `UtcTime`).
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

// ---------- UtcDate ----------

/// Frozen UTC calendar date carried by RMC's `ddmmyy` field
/// (binding class for `UtcDate`).
///
/// `year_yy` is the raw two-digit year — callers apply their own
/// century-resolution rule (the spec does not pin a pivot year).
#[pyclass(name = "UtcDate", frozen, eq, hash, module = "marlin.nmea")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyUtcDate {
    #[pyo3(get)]
    day: u8,
    #[pyo3(get)]
    month: u8,
    #[pyo3(get)]
    year_yy: u8,
}

#[pymethods]
impl PyUtcDate {
    #[new]
    fn new(day: u8, month: u8, year_yy: u8) -> Self {
        Self {
            day,
            month,
            year_yy,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "UtcDate({:02}-{:02}-{:02})",
            self.day, self.month, self.year_yy
        )
    }
}

impl From<RustUtcDate> for PyUtcDate {
    fn from(d: RustUtcDate) -> Self {
        Self {
            day: d.day,
            month: d.month,
            year_yy: d.year_yy,
        }
    }
}

// ---------- Field-state helpers ----------

/// A constructor argument for a field-state attribute: absent or `None`
/// is `NotAvailable`, a `FieldState` passes through, a bare value is
/// `Value`; the payload is extracted to `T` and the extraction error is
/// raised as is.
fn field_arg<'py, T: FromPyObjectOwned<'py>>(
    obj: Option<&Bound<'py, PyAny>>,
) -> PyResult<FieldState<T>> {
    obj.map_or(Ok(FieldState::NotAvailable), from_py)
}

/// A decoded state whose payload is a Rust enum, converted to the
/// binding enum. The Rust enums are `#[non_exhaustive]`, so a variant
/// these bindings do not know is an error, never a fabricated member.
fn enum_state<R, P: TryFrom<R, Error = PyErr>>(state: FieldState<R>) -> PyResult<FieldState<P>> {
    Ok(match state {
        FieldState::Value(v) => FieldState::Value(P::try_from(v)?),
        FieldState::AtLeast(v) => FieldState::AtLeast(P::try_from(v)?),
        FieldState::NotAvailable => FieldState::NotAvailable,
        FieldState::SenderError(code) => FieldState::SenderError(code),
        FieldState::Invalid(why) => FieldState::Invalid(why),
    })
}

fn unsupported_variant(enum_name: &str, variant: &dyn core::fmt::Debug) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(format!(
        "marlin Python bindings encountered an unsupported {enum_name} variant {variant:?} — bindings need updating"
    ))
}

// ---------- Gga ----------

/// Frozen `$__GGA` message (binding class for `GgaData`).
///
/// Every field but `talker` is a `FieldState`; the constructor coerces a
/// bare value to `FieldState.Value` and `None` to
/// `FieldState.NotAvailable()`.
#[pyclass(name = "Gga", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyGga {
    talker: Option<[u8; 2]>,
    utc: FieldState<PyUtcTime>,
    latitude_deg: FieldState<f64>,
    longitude_deg: FieldState<f64>,
    fix_quality: FieldState<PyGgaFixQuality>,
    satellites_used: FieldState<u8>,
    hdop: FieldState<f32>,
    altitude_m: FieldState<f32>,
    geoid_separation_m: FieldState<f32>,
    dgps_age_s: FieldState<f32>,
    dgps_station_id: FieldState<u16>,
}

#[pymethods]
impl PyGga {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        talker,
        utc = None,
        latitude_deg = None,
        longitude_deg = None,
        fix_quality = None,
        satellites_used = None,
        hdop = None,
        altitude_m = None,
        geoid_separation_m = None,
        dgps_age_s = None,
        dgps_station_id = None,
    ))]
    fn new(
        talker: Option<&[u8]>,
        utc: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        fix_quality: Option<&Bound<'_, PyAny>>,
        satellites_used: Option<&Bound<'_, PyAny>>,
        hdop: Option<&Bound<'_, PyAny>>,
        altitude_m: Option<&Bound<'_, PyAny>>,
        geoid_separation_m: Option<&Bound<'_, PyAny>>,
        dgps_age_s: Option<&Bound<'_, PyAny>>,
        dgps_station_id: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            utc: field_arg(utc)?,
            latitude_deg: field_arg(latitude_deg)?,
            longitude_deg: field_arg(longitude_deg)?,
            fix_quality: field_arg(fix_quality)?,
            satellites_used: field_arg(satellites_used)?,
            hdop: field_arg(hdop)?,
            altitude_m: field_arg(altitude_m)?,
            geoid_separation_m: field_arg(geoid_separation_m)?,
            dgps_age_s: field_arg(dgps_age_s)?,
            dgps_station_id: field_arg(dgps_station_id)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn utc(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.utc.clone())
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn fix_quality(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.fix_quality)
    }
    #[getter]
    fn satellites_used(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.satellites_used)
    }
    #[getter]
    fn hdop(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.hdop)
    }
    #[getter]
    fn altitude_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.altitude_m)
    }
    #[getter]
    fn geoid_separation_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.geoid_separation_m)
    }
    #[getter]
    fn dgps_age_s(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.dgps_age_s)
    }
    #[getter]
    fn dgps_station_id(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.dgps_station_id)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Gga(talker={}, fix_quality={}, lat={}, lon={})",
            repr_talker(self.talker),
            repr_state(py, self.fix_quality)?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl TryFrom<GgaData> for PyGga {
    type Error = PyErr;

    fn try_from(d: GgaData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            utc: d.utc.map(PyUtcTime::from),
            latitude_deg: d.latitude_deg,
            longitude_deg: d.longitude_deg,
            fix_quality: enum_state(d.fix_quality)?,
            satellites_used: d.satellites_used,
            hdop: d.hdop,
            altitude_m: d.altitude_m,
            geoid_separation_m: d.geoid_separation_m,
            dgps_age_s: d.dgps_age_s,
            dgps_station_id: d.dgps_station_id,
        })
    }
}

// ---------- Vtg ----------

/// Frozen `$__VTG` message (binding class for `VtgData`).
///
/// Every field but `talker` is a `FieldState`; the constructor coerces a
/// bare value to `FieldState.Value` and `None` to
/// `FieldState.NotAvailable()`.
#[pyclass(name = "Vtg", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyVtg {
    talker: Option<[u8; 2]>,
    course_true_deg: FieldState<f32>,
    course_magnetic_deg: FieldState<f32>,
    speed_knots: FieldState<f32>,
    speed_kmh: FieldState<f32>,
    mode: FieldState<PyVtgMode>,
}

#[pymethods]
impl PyVtg {
    #[new]
    #[pyo3(signature = (
        talker,
        course_true_deg = None,
        course_magnetic_deg = None,
        speed_knots = None,
        speed_kmh = None,
        mode = None,
    ))]
    fn new(
        talker: Option<&[u8]>,
        course_true_deg: Option<&Bound<'_, PyAny>>,
        course_magnetic_deg: Option<&Bound<'_, PyAny>>,
        speed_knots: Option<&Bound<'_, PyAny>>,
        speed_kmh: Option<&Bound<'_, PyAny>>,
        mode: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            course_true_deg: field_arg(course_true_deg)?,
            course_magnetic_deg: field_arg(course_magnetic_deg)?,
            speed_knots: field_arg(speed_knots)?,
            speed_kmh: field_arg(speed_kmh)?,
            mode: field_arg(mode)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn course_true_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_true_deg)
    }
    #[getter]
    fn course_magnetic_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_magnetic_deg)
    }
    #[getter]
    fn speed_knots(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_knots)
    }
    #[getter]
    fn speed_kmh(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_kmh)
    }
    #[getter]
    fn mode(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.mode)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Vtg(talker={}, course_true={}, speed_knots={}, mode={})",
            repr_talker(self.talker),
            repr_state(py, self.course_true_deg)?,
            repr_state(py, self.speed_knots)?,
            repr_state(py, self.mode)?,
        ))
    }
}

impl TryFrom<VtgData> for PyVtg {
    type Error = PyErr;

    fn try_from(d: VtgData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            course_true_deg: d.course_true_deg,
            course_magnetic_deg: d.course_magnetic_deg,
            speed_knots: d.speed_knots,
            speed_kmh: d.speed_kmh,
            mode: enum_state(d.mode)?,
        })
    }
}

// ---------- Hdt ----------

/// Frozen `$__HDT` message (binding class for `HdtData`).
///
/// `heading_true_deg` is a `FieldState`; the constructor coerces a bare
/// value to `FieldState.Value` and `None` to `FieldState.NotAvailable()`.
#[pyclass(name = "Hdt", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyHdt {
    talker: Option<[u8; 2]>,
    heading_true_deg: FieldState<f32>,
}

#[pymethods]
impl PyHdt {
    #[new]
    #[pyo3(signature = (talker, heading_true_deg = None))]
    fn new(talker: Option<&[u8]>, heading_true_deg: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            heading_true_deg: field_arg(heading_true_deg)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn heading_true_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.heading_true_deg)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Hdt(talker={}, heading_true_deg={})",
            repr_talker(self.talker),
            repr_state(py, self.heading_true_deg)?,
        ))
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

// ---------- Hdg ----------

/// Frozen `$__HDG` message (binding class for `HdgData`).
///
/// Every field but `talker` is a `FieldState`; the deviation and the
/// variation are paired fields, invalid when only the magnitude or only
/// the `E`/`W` letter arrived.
#[pyclass(name = "Hdg", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyHdg {
    talker: Option<[u8; 2]>,
    heading_magnetic_deg: FieldState<f32>,
    deviation_deg: FieldState<f32>,
    variation_deg: FieldState<f32>,
}

#[pymethods]
impl PyHdg {
    #[new]
    #[pyo3(signature = (talker, heading_magnetic_deg = None, deviation_deg = None, variation_deg = None))]
    fn new(
        talker: Option<&[u8]>,
        heading_magnetic_deg: Option<&Bound<'_, PyAny>>,
        deviation_deg: Option<&Bound<'_, PyAny>>,
        variation_deg: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            heading_magnetic_deg: field_arg(heading_magnetic_deg)?,
            deviation_deg: field_arg(deviation_deg)?,
            variation_deg: field_arg(variation_deg)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn heading_magnetic_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.heading_magnetic_deg)
    }
    #[getter]
    fn deviation_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.deviation_deg)
    }
    #[getter]
    fn variation_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.variation_deg)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Hdg(talker={}, heading_magnetic={}, deviation={}, variation={})",
            repr_talker(self.talker),
            repr_state(py, self.heading_magnetic_deg)?,
            repr_state(py, self.deviation_deg)?,
            repr_state(py, self.variation_deg)?,
        ))
    }
}

impl From<HdgData> for PyHdg {
    fn from(d: HdgData) -> Self {
        Self {
            talker: d.talker,
            heading_magnetic_deg: d.heading_magnetic_deg,
            deviation_deg: d.deviation_deg,
            variation_deg: d.variation_deg,
        }
    }
}

// ---------- Ttm ----------

/// Frozen `$__TTM` message (binding class for `TtmData`).
///
/// Every field but `talker` is a `FieldState`. `reference_target` is
/// `FieldState.Value(False)` for an empty or absent field, never
/// `FieldState.NotAvailable()`.
#[pyclass(name = "Ttm", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyTtm {
    talker: Option<[u8; 2]>,
    target_number: FieldState<u16>,
    distance: FieldState<f32>,
    bearing_deg: FieldState<f32>,
    bearing_reference: FieldState<PyAngleReference>,
    speed: FieldState<f32>,
    course_deg: FieldState<f32>,
    course_reference: FieldState<PyAngleReference>,
    cpa: FieldState<f32>,
    tcpa: FieldState<f32>,
    units: FieldState<PyDistanceUnits>,
    name: FieldState<String>,
    status: FieldState<PyTargetStatus>,
    reference_target: FieldState<bool>,
    utc_time: FieldState<PyUtcTime>,
    acquisition: FieldState<PyAcquisitionType>,
}

#[pymethods]
impl PyTtm {
    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn target_number(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.target_number)
    }
    #[getter]
    fn distance(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.distance)
    }
    #[getter]
    fn bearing_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.bearing_deg)
    }
    #[getter]
    fn bearing_reference(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.bearing_reference)
    }
    #[getter]
    fn speed(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed)
    }
    #[getter]
    fn course_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_deg)
    }
    #[getter]
    fn course_reference(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_reference)
    }
    #[getter]
    fn cpa(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.cpa)
    }
    #[getter]
    fn tcpa(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.tcpa)
    }
    #[getter]
    fn units(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.units)
    }
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.name.clone())
    }
    #[getter]
    fn status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.status)
    }
    #[getter]
    fn reference_target(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.reference_target)
    }
    #[getter]
    fn utc_time(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.utc_time.clone())
    }
    #[getter]
    fn acquisition(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.acquisition)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Ttm(talker={}, target_number={}, name={}, status={})",
            repr_talker(self.talker),
            repr_state(py, self.target_number)?,
            repr_state(py, self.name.clone())?,
            repr_state(py, self.status)?,
        ))
    }
}

impl TryFrom<TtmData> for PyTtm {
    type Error = PyErr;

    fn try_from(d: TtmData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            target_number: d.target_number,
            distance: d.distance,
            bearing_deg: d.bearing_deg,
            bearing_reference: enum_state(d.bearing_reference)?,
            speed: d.speed,
            course_deg: d.course_deg,
            course_reference: enum_state(d.course_reference)?,
            cpa: d.cpa,
            tcpa: d.tcpa,
            units: enum_state(d.units)?,
            name: d.name,
            status: enum_state(d.status)?,
            reference_target: d.reference_target,
            utc_time: d.utc_time.map(PyUtcTime::from),
            acquisition: enum_state(d.acquisition)?,
        })
    }
}

// ---------- Tll ----------

/// Frozen `$__TLL` message (binding class for `TllData`).
///
/// Every field but `talker` is a `FieldState`. `reference_target` is
/// `FieldState.Value(False)` for an empty or absent field, never
/// `FieldState.NotAvailable()`.
#[pyclass(name = "Tll", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyTll {
    talker: Option<[u8; 2]>,
    target_number: FieldState<u16>,
    latitude_deg: FieldState<f64>,
    longitude_deg: FieldState<f64>,
    name: FieldState<String>,
    utc_time: FieldState<PyUtcTime>,
    status: FieldState<PyTargetStatus>,
    reference_target: FieldState<bool>,
}

#[pymethods]
impl PyTll {
    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn target_number(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.target_number)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.name.clone())
    }
    #[getter]
    fn utc_time(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.utc_time.clone())
    }
    #[getter]
    fn status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.status)
    }
    #[getter]
    fn reference_target(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.reference_target)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Tll(talker={}, target_number={}, lat={}, lon={})",
            repr_talker(self.talker),
            repr_state(py, self.target_number)?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl TryFrom<TllData> for PyTll {
    type Error = PyErr;

    fn try_from(d: TllData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            target_number: d.target_number,
            latitude_deg: d.latitude_deg,
            longitude_deg: d.longitude_deg,
            name: d.name,
            utc_time: d.utc_time.map(PyUtcTime::from),
            status: enum_state(d.status)?,
            reference_target: d.reference_target,
        })
    }
}

// ---------- Rmc ----------

/// Frozen `$__RMC` message (binding class for `RmcData`).
///
/// Single-sentence carrier of UTC time + date + position + speed +
/// course + magnetic variation. Every field but `talker` is a
/// `FieldState`. Safety-critical consumers should reject a `status` of
/// `FieldState.Value(DataStatus.VOID)` before using the position or
/// velocity values; the status does not change their state.
#[pyclass(name = "Rmc", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyRmc {
    talker: Option<[u8; 2]>,
    utc: FieldState<PyUtcTime>,
    status: FieldState<PyDataStatus>,
    latitude_deg: FieldState<f64>,
    longitude_deg: FieldState<f64>,
    speed_knots: FieldState<f32>,
    course_true_deg: FieldState<f32>,
    date: FieldState<PyUtcDate>,
    magnetic_variation_deg: FieldState<f32>,
    mode: FieldState<PyVtgMode>,
    nav_status: FieldState<PyRmcNavStatus>,
}

#[pymethods]
impl PyRmc {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        talker,
        utc = None,
        status = None,
        latitude_deg = None,
        longitude_deg = None,
        speed_knots = None,
        course_true_deg = None,
        date = None,
        magnetic_variation_deg = None,
        mode = None,
        nav_status = None,
    ))]
    fn new(
        talker: Option<&[u8]>,
        utc: Option<&Bound<'_, PyAny>>,
        status: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        speed_knots: Option<&Bound<'_, PyAny>>,
        course_true_deg: Option<&Bound<'_, PyAny>>,
        date: Option<&Bound<'_, PyAny>>,
        magnetic_variation_deg: Option<&Bound<'_, PyAny>>,
        mode: Option<&Bound<'_, PyAny>>,
        nav_status: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            utc: field_arg(utc)?,
            status: field_arg(status)?,
            latitude_deg: field_arg(latitude_deg)?,
            longitude_deg: field_arg(longitude_deg)?,
            speed_knots: field_arg(speed_knots)?,
            course_true_deg: field_arg(course_true_deg)?,
            date: field_arg(date)?,
            magnetic_variation_deg: field_arg(magnetic_variation_deg)?,
            mode: field_arg(mode)?,
            nav_status: field_arg(nav_status)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn utc(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.utc.clone())
    }
    #[getter]
    fn status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.status)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn speed_knots(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_knots)
    }
    #[getter]
    fn course_true_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_true_deg)
    }
    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.date.clone())
    }
    #[getter]
    fn magnetic_variation_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.magnetic_variation_deg)
    }
    #[getter]
    fn mode(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.mode)
    }
    #[getter]
    fn nav_status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.nav_status)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Rmc(talker={}, status={}, lat={}, lon={}, sog_kn={}, cog_true={})",
            repr_talker(self.talker),
            repr_state(py, self.status)?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
            repr_state(py, self.speed_knots)?,
            repr_state(py, self.course_true_deg)?,
        ))
    }
}

impl TryFrom<RmcData> for PyRmc {
    type Error = PyErr;

    fn try_from(d: RmcData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            utc: d.utc.map(PyUtcTime::from),
            status: enum_state(d.status)?,
            latitude_deg: d.latitude_deg,
            longitude_deg: d.longitude_deg,
            speed_knots: d.speed_knots,
            course_true_deg: d.course_true_deg,
            date: d.date.map(PyUtcDate::from),
            magnetic_variation_deg: d.magnetic_variation_deg,
            mode: enum_state(d.mode)?,
            nav_status: enum_state(d.nav_status)?,
        })
    }
}

// ---------- Gll ----------

/// Frozen `$__GLL` message (binding class for `GllData`).
///
/// Position-only sentence with UTC time and an A/V validity status.
/// Every field but `talker` is a `FieldState`. Safety-critical
/// consumers should reject a `status` of
/// `FieldState.Value(DataStatus.VOID)` before using the position
/// values; the status does not change their state.
#[pyclass(name = "Gll", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyGll {
    talker: Option<[u8; 2]>,
    latitude_deg: FieldState<f64>,
    longitude_deg: FieldState<f64>,
    utc: FieldState<PyUtcTime>,
    status: FieldState<PyDataStatus>,
    mode: FieldState<PyVtgMode>,
}

#[pymethods]
impl PyGll {
    #[new]
    #[pyo3(signature = (
        talker,
        latitude_deg = None,
        longitude_deg = None,
        utc = None,
        status = None,
        mode = None,
    ))]
    fn new(
        talker: Option<&[u8]>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        utc: Option<&Bound<'_, PyAny>>,
        status: Option<&Bound<'_, PyAny>>,
        mode: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            talker: normalize_talker(talker)?,
            latitude_deg: field_arg(latitude_deg)?,
            longitude_deg: field_arg(longitude_deg)?,
            utc: field_arg(utc)?,
            status: field_arg(status)?,
            mode: field_arg(mode)?,
        })
    }

    #[getter]
    fn talker<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.talker.map(|t| PyBytes::new(py, &t))
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn utc(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.utc.clone())
    }
    #[getter]
    fn status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.status)
    }
    #[getter]
    fn mode(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.mode)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Gll(talker={}, status={}, lat={}, lon={})",
            repr_talker(self.talker),
            repr_state(py, self.status)?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl TryFrom<GllData> for PyGll {
    type Error = PyErr;

    fn try_from(d: GllData) -> PyResult<Self> {
        Ok(Self {
            talker: d.talker,
            latitude_deg: d.latitude_deg,
            longitude_deg: d.longitude_deg,
            utc: d.utc.map(PyUtcTime::from),
            status: enum_state(d.status)?,
            mode: enum_state(d.mode)?,
        })
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
/// (binding class for `PsxnSlot`).
#[pyclass(name = "PsxnSlot", frozen, eq, eq_int, hash, module = "marlin.nmea")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    // `_ => Ignored` pair.
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

/// Runtime selector for PRDID field ordering (binding class for
/// `PrdidDialect`).
#[pyclass(
    name = "PrdidDialect",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.nmea"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    // `Unknown`; silence `match_same_arms` for the pair.
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

/// Frozen PSXN layout descriptor (binding class for `PsxnLayout`).
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
    // The `from_str` name follows the Rust `FromStr` impl for Python-side
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

/// Frozen runtime configuration for ambiguous decodings (binding class for
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
        let layout_repr = PyPsxnLayout {
            inner: self.inner.psxn_layout,
        }
        .__repr__();
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

/// Frozen `$PSXN` payload (binding class for `PsxnData`).
///
/// PSXN is proprietary — there is no talker. `PsxnLayout` describes
/// how the six on-wire data fields decode into these five motion
/// quantities; the output shape is fixed regardless of layout. Every
/// field is a `FieldState`: a quantity no data field carries under the
/// layout is `FieldState.NotAvailable()`, and a derived angle the data
/// fields cannot produce is `FieldState.Invalid(None)`.
#[pyclass(name = "Psxn", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPsxn {
    id: FieldState<u16>,
    token: FieldState<Vec<u8>>,
    roll_deg: FieldState<f32>,
    pitch_deg: FieldState<f32>,
    heave_m: FieldState<f32>,
}

#[pymethods]
impl PyPsxn {
    #[new]
    #[pyo3(signature = (id = None, token = None, roll_deg = None, pitch_deg = None, heave_m = None))]
    fn new(
        id: Option<&Bound<'_, PyAny>>,
        token: Option<&Bound<'_, PyAny>>,
        roll_deg: Option<&Bound<'_, PyAny>>,
        pitch_deg: Option<&Bound<'_, PyAny>>,
        heave_m: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            id: field_arg(id)?,
            token: field_arg(token)?,
            roll_deg: field_arg(roll_deg)?,
            pitch_deg: field_arg(pitch_deg)?,
            heave_m: field_arg(heave_m)?,
        })
    }

    #[getter]
    fn id(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.id)
    }
    #[getter]
    fn token(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.token.as_ref().map(|t| PyBytes::new(py, t)))
    }
    #[getter]
    fn roll_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.roll_deg)
    }
    #[getter]
    fn pitch_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.pitch_deg)
    }
    #[getter]
    fn heave_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.heave_m)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Psxn(id={}, roll={}, pitch={}, heave={})",
            repr_state(py, self.id)?,
            repr_state(py, self.roll_deg)?,
            repr_state(py, self.pitch_deg)?,
            repr_state(py, self.heave_m)?,
        ))
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
/// (binding class for `PrdidPitchRollHeading`). Every field is a
/// `FieldState`.
// Field names match the Rust struct and are Python-visible via getters;
// `_deg` conveys the unit and must stay.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "PrdidPitchRollHeading", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdidPitchRollHeading {
    pitch_deg: FieldState<f32>,
    roll_deg: FieldState<f32>,
    heading_deg: FieldState<f32>,
}

#[pymethods]
impl PyPrdidPitchRollHeading {
    #[new]
    #[pyo3(signature = (pitch_deg = None, roll_deg = None, heading_deg = None))]
    fn new(
        pitch_deg: Option<&Bound<'_, PyAny>>,
        roll_deg: Option<&Bound<'_, PyAny>>,
        heading_deg: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            pitch_deg: field_arg(pitch_deg)?,
            roll_deg: field_arg(roll_deg)?,
            heading_deg: field_arg(heading_deg)?,
        })
    }

    #[getter]
    fn pitch_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.pitch_deg)
    }
    #[getter]
    fn roll_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.roll_deg)
    }
    #[getter]
    fn heading_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.heading_deg)
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
/// (binding class for `PrdidRollPitchHeading`). Every field is a
/// `FieldState`.
// Field names match the Rust struct and are Python-visible via getters;
// `_deg` conveys the unit and must stay.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "PrdidRollPitchHeading", frozen, module = "marlin.nmea")]
#[derive(Clone, Debug)]
pub struct PyPrdidRollPitchHeading {
    roll_deg: FieldState<f32>,
    pitch_deg: FieldState<f32>,
    heading_deg: FieldState<f32>,
}

#[pymethods]
impl PyPrdidRollPitchHeading {
    #[new]
    #[pyo3(signature = (roll_deg = None, pitch_deg = None, heading_deg = None))]
    fn new(
        roll_deg: Option<&Bound<'_, PyAny>>,
        pitch_deg: Option<&Bound<'_, PyAny>>,
        heading_deg: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            roll_deg: field_arg(roll_deg)?,
            pitch_deg: field_arg(pitch_deg)?,
            heading_deg: field_arg(heading_deg)?,
        })
    }

    #[getter]
    fn roll_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.roll_deg)
    }
    #[getter]
    fn pitch_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.pitch_deg)
    }
    #[getter]
    fn heading_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.heading_deg)
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

/// Frozen `$PRDID` raw-bytes body (binding class for `PrdidData::Raw`).
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
        pitch_deg: Option<&Bound<'_, PyAny>>,
        roll_deg: Option<&Bound<'_, PyAny>>,
        heading_deg: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            variant: PrdidVariant::PitchRollHeading(PyPrdidPitchRollHeading::new(
                pitch_deg,
                roll_deg,
                heading_deg,
            )?),
        })
    }

    #[staticmethod]
    #[pyo3(signature = (roll_deg = None, pitch_deg = None, heading_deg = None))]
    fn roll_pitch_heading(
        roll_deg: Option<&Bound<'_, PyAny>>,
        pitch_deg: Option<&Bound<'_, PyAny>>,
        heading_deg: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            variant: PrdidVariant::RollPitchHeading(PyPrdidRollPitchHeading::new(
                roll_deg,
                pitch_deg,
                heading_deg,
            )?),
        })
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

/// Typed NMEA 0183 parser that wraps an envelope parser and exposes
/// [`Nmea0183Message`] variants. Construct via `streaming()` or
/// `one_shot()` staticmethods; both accept optional `DecodeOptions`.
#[pyclass(name = "Nmea0183Parser", module = "marlin.nmea")]
pub struct PyNmea0183Parser {
    inner: RustNmea0183<Parser>,
}

#[pymethods]
impl PyNmea0183Parser {
    #[staticmethod]
    #[pyo3(signature = (options = None))]
    fn one_shot(options: Option<PyDecodeOptions>) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        Self {
            inner: RustNmea0183::with_options(Parser::one_shot(), opts),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (options = None, max_size = DEFAULT_MAX_SIZE))]
    fn streaming(options: Option<PyDecodeOptions>, max_size: usize) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        Self {
            inner: RustNmea0183::with_options(Parser::streaming_with_capacity(max_size), opts),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.inner.feed(data);
    }

    /// Return the next decoded message, `None` if no complete sentence is
    /// buffered, or raise `EnvelopeError` / `DecodeError` on failure.
    fn next_message(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        // Convert the borrowed Nmea0183Message<'_> into an owned IR before
        // dropping the &mut borrow on `self.inner`. Otherwise the borrow
        // checker flags the subsequent PyO3 construction as overlapping
        // with the `next_message()` borrow.
        let result = self
            .inner
            .next_message()
            .map(|r| r.map(owned_message_from_borrowed));
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

/// Owned copy of `Nmea0183Message<'a>` — lifetime-stripped so we can
/// hand the result back across `PyO3` borrows without holding the
/// underlying envelope buffer borrow open.
enum OwnedMessage {
    Gga(GgaData),
    Gll(GllData),
    Hdt(HdtData),
    Rmc(RmcData),
    Vtg(VtgData),
    Hdg(HdgData),
    Ttm(TtmData),
    Tll(TllData),
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
        Nmea0183Message::Gll(d) => OwnedMessage::Gll(d),
        Nmea0183Message::Hdt(d) => OwnedMessage::Hdt(d),
        Nmea0183Message::Rmc(d) => OwnedMessage::Rmc(d),
        Nmea0183Message::Vtg(d) => OwnedMessage::Vtg(d),
        Nmea0183Message::Hdg(d) => OwnedMessage::Hdg(d),
        Nmea0183Message::Ttm(d) => OwnedMessage::Ttm(d),
        Nmea0183Message::Tll(d) => OwnedMessage::Tll(d),
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
            Self::Gga(d) => Py::new(py, PyGga::try_from(d)?)?.into_any(),
            Self::Gll(d) => Py::new(py, PyGll::try_from(d)?)?.into_any(),
            Self::Hdt(d) => Py::new(py, PyHdt::from(d))?.into_any(),
            Self::Rmc(d) => Py::new(py, PyRmc::try_from(d)?)?.into_any(),
            Self::Vtg(d) => Py::new(py, PyVtg::try_from(d)?)?.into_any(),
            Self::Hdg(d) => Py::new(py, PyHdg::from(d))?.into_any(),
            Self::Ttm(d) => Py::new(py, PyTtm::try_from(d)?)?.into_any(),
            Self::Tll(d) => Py::new(py, PyTll::try_from(d)?)?.into_any(),
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
        .map_err(decode_err)
        .and_then(PyGga::try_from)
}

#[pyfunction]
#[pyo3(name = "decode_vtg")]
fn py_decode_vtg(raw: &PyRawSentence) -> PyResult<PyVtg> {
    let rust_raw = raw.to_rust();
    rust_decode_vtg(&rust_raw)
        .map_err(decode_err)
        .and_then(PyVtg::try_from)
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
#[pyo3(name = "decode_hdg")]
fn py_decode_hdg(raw: &PyRawSentence) -> PyResult<PyHdg> {
    let rust_raw = raw.to_rust();
    rust_decode_hdg(&rust_raw)
        .map(PyHdg::from)
        .map_err(decode_err)
}

#[pyfunction]
#[pyo3(name = "decode_ttm")]
fn py_decode_ttm(raw: &PyRawSentence) -> PyResult<PyTtm> {
    let rust_raw = raw.to_rust();
    rust_decode_ttm(&rust_raw)
        .map_err(decode_err)
        .and_then(PyTtm::try_from)
}

#[pyfunction]
#[pyo3(name = "decode_tll")]
fn py_decode_tll(raw: &PyRawSentence) -> PyResult<PyTll> {
    let rust_raw = raw.to_rust();
    rust_decode_tll(&rust_raw)
        .map_err(decode_err)
        .and_then(PyTll::try_from)
}

#[pyfunction]
#[pyo3(name = "decode_rmc")]
fn py_decode_rmc(raw: &PyRawSentence) -> PyResult<PyRmc> {
    let rust_raw = raw.to_rust();
    rust_decode_rmc(&rust_raw)
        .map_err(decode_err)
        .and_then(PyRmc::try_from)
}

#[pyfunction]
#[pyo3(name = "decode_gll")]
fn py_decode_gll(raw: &PyRawSentence) -> PyResult<PyGll> {
    let rust_raw = raw.to_rust();
    rust_decode_gll(&rust_raw)
        .map_err(decode_err)
        .and_then(PyGll::try_from)
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
    m.add_class::<PyDataStatus>()?;
    m.add_class::<PyRmcNavStatus>()?;
    m.add_class::<PyPsxnSlot>()?;
    m.add_class::<PyPrdidDialect>()?;
    m.add_class::<PyUtcTime>()?;
    m.add_class::<PyUtcDate>()?;
    m.add_class::<PyPsxnLayout>()?;
    m.add_class::<PyDecodeOptions>()?;
    m.add_class::<PyGga>()?;
    m.add_class::<PyGll>()?;
    m.add_class::<PyVtg>()?;
    m.add_class::<PyHdt>()?;
    m.add_class::<PyHdg>()?;
    m.add_class::<PyTtm>()?;
    m.add_class::<PyTll>()?;
    m.add_class::<PyTargetStatus>()?;
    m.add_class::<PyAngleReference>()?;
    m.add_class::<PyDistanceUnits>()?;
    m.add_class::<PyAcquisitionType>()?;
    m.add_class::<PyRmc>()?;
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
    m.add_function(wrap_pyfunction!(py_decode_gll, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_vtg, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_hdt, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_hdg, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_ttm, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_tll, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_rmc, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_psxn, &m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_prdid, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
