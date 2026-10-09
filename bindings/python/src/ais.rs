//! Python wrappers for `marlin-ais`.
//!
//! The data primitives (int-backed enums, the `Dimensions` / `Eta` value
//! types and the three sum types `RateOfTurn`, `Timestamp` and
//! `Type24BExtent`) come first, then the typed message classes, the
//! `AisMessage` wrapper, the `BitReader` primitive and the parser. Every
//! field the Rust decoder carries as a `FieldState<T>` is a
//! `marlin.field.FieldState` attribute here, converted through
//! `crate::field::{to_py, field_arg}`; a sum type in field position is a
//! frozen class with one variant class per Rust variant (ADR-0009).

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule, PyTuple};
use pyo3::IntoPyObjectExt;

use marlin_ais::{
    AidToNavigationReport as RustAidToNavigationReport, AisFragmentParser, AisMessageBody,
    AisReassembler, AisVersion as RustAisVersion, AltitudeSensor as RustAltitudeSensor,
    AtonType as RustAtonType, BitReader as RustBitReader, Dimensions as RustDimensions,
    EpfdType as RustEpfdType, Eta as RustEta,
    ExtendedPositionReportB as RustExtendedPositionReportB, FieldState,
    ManeuverIndicator as RustManeuverIndicator, NavStatus as RustNavStatus,
    PositionReportA as RustPositionReportA, PositionReportB as RustPositionReportB,
    PositioningStatus as RustPositioningStatus, RateOfTurn as RustRateOfTurn,
    SarAircraftPositionReport as RustSarAircraftPositionReport,
    StaticAndVoyageA as RustStaticAndVoyageA, StaticDataB24A as RustStaticDataB24A,
    StaticDataB24B as RustStaticDataB24B, Timestamp as RustTimestamp,
    TurnDirection as RustTurnDirection, Type24BExtent as RustType24BExtent, DEFAULT_MAX_PARTIALS,
};
use marlin_nmea_envelope::Parser;

use crate::envelope::DEFAULT_MAX_SIZE;
use crate::errors::ais_err;
use crate::field::{
    enum_state, field_arg, name_variants, repr_state, to_py, unsupported_variant, PyFieldState,
};

// ---------- NavStatus ----------

/// Navigation status of a Class A position report (binding class for
/// `NavStatus`). The int values are the 4-bit wire codes 0..=8 and 14.
///
/// Code 15 (not defined) decodes to `FieldState.NotAvailable()` and the
/// reserved codes 9..=13 to `FieldState.Invalid(code)` on the message,
/// so neither is a member here.
#[pyclass(name = "NavStatus", frozen, eq, eq_int, hash, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyNavStatus {
    #[pyo3(name = "UNDERWAY_USING_ENGINE")]
    UnderwayUsingEngine = 0,
    #[pyo3(name = "AT_ANCHOR")]
    AtAnchor = 1,
    #[pyo3(name = "NOT_UNDER_COMMAND")]
    NotUnderCommand = 2,
    #[pyo3(name = "RESTRICTED_MANEUVERABILITY")]
    RestrictedManeuverability = 3,
    #[pyo3(name = "CONSTRAINED_BY_DRAFT")]
    ConstrainedByDraft = 4,
    #[pyo3(name = "MOORED")]
    Moored = 5,
    #[pyo3(name = "AGROUND")]
    Aground = 6,
    #[pyo3(name = "ENGAGED_IN_FISHING")]
    EngagedInFishing = 7,
    #[pyo3(name = "UNDERWAY_SAILING")]
    UnderwaySailing = 8,
    #[pyo3(name = "AIS_SART_ACTIVE")]
    AisSartActive = 14,
}

impl TryFrom<RustNavStatus> for PyNavStatus {
    type Error = PyErr;

    fn try_from(v: RustNavStatus) -> PyResult<Self> {
        Ok(match v {
            RustNavStatus::UnderwayUsingEngine => Self::UnderwayUsingEngine,
            RustNavStatus::AtAnchor => Self::AtAnchor,
            RustNavStatus::NotUnderCommand => Self::NotUnderCommand,
            RustNavStatus::RestrictedManeuverability => Self::RestrictedManeuverability,
            RustNavStatus::ConstrainedByDraft => Self::ConstrainedByDraft,
            RustNavStatus::Moored => Self::Moored,
            RustNavStatus::Aground => Self::Aground,
            RustNavStatus::EngagedInFishing => Self::EngagedInFishing,
            RustNavStatus::UnderwaySailing => Self::UnderwaySailing,
            RustNavStatus::AisSartActive => Self::AisSartActive,
            other => return Err(unsupported_variant("NavStatus", &other)),
        })
    }
}

// ---------- ManeuverIndicator ----------

/// Special manoeuvre indicator of a Class A position report (binding
/// class for `ManeuverIndicator`). The int values are the 2-bit wire
/// codes 1 and 2.
///
/// Code 0 decodes to `FieldState.NotAvailable()` and the reserved code 3
/// to `FieldState.Invalid(3)` on the message, so neither is a member
/// here.
#[pyclass(
    name = "ManeuverIndicator",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.ais"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyManeuverIndicator {
    #[pyo3(name = "NO_SPECIAL")]
    NoSpecial = 1,
    #[pyo3(name = "SPECIAL")]
    Special = 2,
}

impl TryFrom<RustManeuverIndicator> for PyManeuverIndicator {
    type Error = PyErr;

    fn try_from(v: RustManeuverIndicator) -> PyResult<Self> {
        Ok(match v {
            RustManeuverIndicator::NoSpecial => Self::NoSpecial,
            RustManeuverIndicator::Special => Self::Special,
            other => return Err(unsupported_variant("ManeuverIndicator", &other)),
        })
    }
}

// ---------- TurnDirection ----------

/// Direction of turn when a Type 1/2/3 report carries the "turning
/// right/left at more than 5° per 30 s, no turn indicator" status
/// (binding class for `TurnDirection`). Carried by
/// `RateOfTurn.NoIndicator(direction)`.
///
/// The int values (`RIGHT = 0`, `LEFT = 1`) are enum discriminants, not
/// wire codes: on the wire the statuses are the raw rate-of-turn bytes
/// `+127` and `−127`.
#[pyclass(
    name = "TurnDirection",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.ais"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyTurnDirection {
    #[pyo3(name = "RIGHT")]
    Right = 0,
    #[pyo3(name = "LEFT")]
    Left = 1,
}

impl From<RustTurnDirection> for PyTurnDirection {
    fn from(v: RustTurnDirection) -> Self {
        match v {
            RustTurnDirection::Right => Self::Right,
            RustTurnDirection::Left => Self::Left,
        }
    }
}

// ---------- PositioningStatus ----------

/// Status of the positioning system when a report's timestamp field
/// carries a status instead of a second (binding class for
/// `PositioningStatus`). Carried by `Timestamp.PositioningStatus(status)`.
/// The int values are the 6-bit timestamp wire codes 61, 62 and 63.
#[pyclass(
    name = "PositioningStatus",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.ais"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyPositioningStatus {
    #[pyo3(name = "MANUAL_INPUT")]
    ManualInput = 61,
    #[pyo3(name = "DEAD_RECKONING")]
    DeadReckoning = 62,
    #[pyo3(name = "INOPERATIVE")]
    Inoperative = 63,
}

impl From<RustPositioningStatus> for PyPositioningStatus {
    fn from(v: RustPositioningStatus) -> Self {
        match v {
            RustPositioningStatus::ManualInput => Self::ManualInput,
            RustPositioningStatus::DeadReckoning => Self::DeadReckoning,
            RustPositioningStatus::Inoperative => Self::Inoperative,
        }
    }
}

// ---------- AltitudeSensor ----------

/// Source of a SAR aircraft's reported altitude (binding class for
/// `AltitudeSensor`, ITU-R M.1371-5 Table 59 bit 134). The int values
/// are the wire codes: `GNSS = 0`, `BAROMETRIC = 1`.
#[pyclass(
    name = "AltitudeSensor",
    frozen,
    eq,
    eq_int,
    hash,
    module = "marlin.ais"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAltitudeSensor {
    #[pyo3(name = "GNSS")]
    Gnss = 0,
    #[pyo3(name = "BAROMETRIC")]
    Barometric = 1,
}

impl From<RustAltitudeSensor> for PyAltitudeSensor {
    fn from(v: RustAltitudeSensor) -> Self {
        match v {
            RustAltitudeSensor::Gnss => Self::Gnss,
            RustAltitudeSensor::Barometric => Self::Barometric,
        }
    }
}

// ---------- AtonType ----------

/// Type of aid to navigation (binding class for `AtonType`, ITU-R
/// M.1371-5 Table 74). The int values are the 5-bit wire codes 0..=31;
/// codes 5–19 are fixed AtoN, 20–31 floating AtoN, 0–4 neither.
#[pyclass(name = "AtonType", frozen, eq, eq_int, hash, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAtonType {
    #[pyo3(name = "NOT_SPECIFIED")]
    NotSpecified = 0,
    #[pyo3(name = "REFERENCE_POINT")]
    ReferencePoint = 1,
    #[pyo3(name = "RACON")]
    Racon = 2,
    #[pyo3(name = "FIXED_STRUCTURE_OFFSHORE")]
    FixedStructureOffshore = 3,
    #[pyo3(name = "EMERGENCY_WRECK_MARKING_BUOY")]
    EmergencyWreckMarkingBuoy = 4,
    #[pyo3(name = "LIGHT_WITHOUT_SECTORS")]
    LightWithoutSectors = 5,
    #[pyo3(name = "LIGHT_WITH_SECTORS")]
    LightWithSectors = 6,
    #[pyo3(name = "LEADING_LIGHT_FRONT")]
    LeadingLightFront = 7,
    #[pyo3(name = "LEADING_LIGHT_REAR")]
    LeadingLightRear = 8,
    #[pyo3(name = "BEACON_CARDINAL_NORTH")]
    BeaconCardinalNorth = 9,
    #[pyo3(name = "BEACON_CARDINAL_EAST")]
    BeaconCardinalEast = 10,
    #[pyo3(name = "BEACON_CARDINAL_SOUTH")]
    BeaconCardinalSouth = 11,
    #[pyo3(name = "BEACON_CARDINAL_WEST")]
    BeaconCardinalWest = 12,
    #[pyo3(name = "BEACON_PORT_HAND")]
    BeaconPortHand = 13,
    #[pyo3(name = "BEACON_STARBOARD_HAND")]
    BeaconStarboardHand = 14,
    #[pyo3(name = "BEACON_PREFERRED_CHANNEL_PORT_HAND")]
    BeaconPreferredChannelPortHand = 15,
    #[pyo3(name = "BEACON_PREFERRED_CHANNEL_STARBOARD_HAND")]
    BeaconPreferredChannelStarboardHand = 16,
    #[pyo3(name = "BEACON_ISOLATED_DANGER")]
    BeaconIsolatedDanger = 17,
    #[pyo3(name = "BEACON_SAFE_WATER")]
    BeaconSafeWater = 18,
    #[pyo3(name = "BEACON_SPECIAL_MARK")]
    BeaconSpecialMark = 19,
    #[pyo3(name = "CARDINAL_MARK_NORTH")]
    CardinalMarkNorth = 20,
    #[pyo3(name = "CARDINAL_MARK_EAST")]
    CardinalMarkEast = 21,
    #[pyo3(name = "CARDINAL_MARK_SOUTH")]
    CardinalMarkSouth = 22,
    #[pyo3(name = "CARDINAL_MARK_WEST")]
    CardinalMarkWest = 23,
    #[pyo3(name = "PORT_HAND_MARK")]
    PortHandMark = 24,
    #[pyo3(name = "STARBOARD_HAND_MARK")]
    StarboardHandMark = 25,
    #[pyo3(name = "PREFERRED_CHANNEL_PORT_HAND")]
    PreferredChannelPortHand = 26,
    #[pyo3(name = "PREFERRED_CHANNEL_STARBOARD_HAND")]
    PreferredChannelStarboardHand = 27,
    #[pyo3(name = "ISOLATED_DANGER")]
    IsolatedDanger = 28,
    #[pyo3(name = "SAFE_WATER")]
    SafeWater = 29,
    #[pyo3(name = "SPECIAL_MARK")]
    SpecialMark = 30,
    #[pyo3(name = "LIGHT_VESSEL")]
    LightVessel = 31,
}

impl From<RustAtonType> for PyAtonType {
    fn from(v: RustAtonType) -> Self {
        match v {
            RustAtonType::NotSpecified => Self::NotSpecified,
            RustAtonType::ReferencePoint => Self::ReferencePoint,
            RustAtonType::Racon => Self::Racon,
            RustAtonType::FixedStructureOffshore => Self::FixedStructureOffshore,
            RustAtonType::EmergencyWreckMarkingBuoy => Self::EmergencyWreckMarkingBuoy,
            RustAtonType::LightWithoutSectors => Self::LightWithoutSectors,
            RustAtonType::LightWithSectors => Self::LightWithSectors,
            RustAtonType::LeadingLightFront => Self::LeadingLightFront,
            RustAtonType::LeadingLightRear => Self::LeadingLightRear,
            RustAtonType::BeaconCardinalNorth => Self::BeaconCardinalNorth,
            RustAtonType::BeaconCardinalEast => Self::BeaconCardinalEast,
            RustAtonType::BeaconCardinalSouth => Self::BeaconCardinalSouth,
            RustAtonType::BeaconCardinalWest => Self::BeaconCardinalWest,
            RustAtonType::BeaconPortHand => Self::BeaconPortHand,
            RustAtonType::BeaconStarboardHand => Self::BeaconStarboardHand,
            RustAtonType::BeaconPreferredChannelPortHand => Self::BeaconPreferredChannelPortHand,
            RustAtonType::BeaconPreferredChannelStarboardHand => {
                Self::BeaconPreferredChannelStarboardHand
            }
            RustAtonType::BeaconIsolatedDanger => Self::BeaconIsolatedDanger,
            RustAtonType::BeaconSafeWater => Self::BeaconSafeWater,
            RustAtonType::BeaconSpecialMark => Self::BeaconSpecialMark,
            RustAtonType::CardinalMarkNorth => Self::CardinalMarkNorth,
            RustAtonType::CardinalMarkEast => Self::CardinalMarkEast,
            RustAtonType::CardinalMarkSouth => Self::CardinalMarkSouth,
            RustAtonType::CardinalMarkWest => Self::CardinalMarkWest,
            RustAtonType::PortHandMark => Self::PortHandMark,
            RustAtonType::StarboardHandMark => Self::StarboardHandMark,
            RustAtonType::PreferredChannelPortHand => Self::PreferredChannelPortHand,
            RustAtonType::PreferredChannelStarboardHand => Self::PreferredChannelStarboardHand,
            RustAtonType::IsolatedDanger => Self::IsolatedDanger,
            RustAtonType::SafeWater => Self::SafeWater,
            RustAtonType::SpecialMark => Self::SpecialMark,
            RustAtonType::LightVessel => Self::LightVessel,
        }
    }
}

// ---------- EpfdType ----------

/// Electronic Position-Fixing Device type (binding class for `EpfdType`).
/// The int values are the 4-bit wire codes 1..=8 and 15.
///
/// Code 0 (undefined) decodes to `FieldState.NotAvailable()` and the
/// reserved codes 9..=14 to `FieldState.Invalid(code)` on the message,
/// so neither is a member here.
#[pyclass(name = "EpfdType", frozen, eq, eq_int, hash, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyEpfdType {
    #[pyo3(name = "GPS")]
    Gps = 1,
    #[pyo3(name = "GLONASS")]
    Glonass = 2,
    #[pyo3(name = "COMBINED_GPS_GLONASS")]
    CombinedGpsGlonass = 3,
    #[pyo3(name = "LORAN_C")]
    LoranC = 4,
    #[pyo3(name = "CHAYKA")]
    Chayka = 5,
    #[pyo3(name = "INTEGRATED_NAVIGATION")]
    IntegratedNavigation = 6,
    #[pyo3(name = "SURVEYED")]
    Surveyed = 7,
    #[pyo3(name = "GALILEO")]
    Galileo = 8,
    #[pyo3(name = "INTERNAL_GNSS")]
    InternalGnss = 15,
}

impl TryFrom<RustEpfdType> for PyEpfdType {
    type Error = PyErr;

    fn try_from(v: RustEpfdType) -> PyResult<Self> {
        Ok(match v {
            RustEpfdType::Gps => Self::Gps,
            RustEpfdType::Glonass => Self::Glonass,
            RustEpfdType::CombinedGpsGlonass => Self::CombinedGpsGlonass,
            RustEpfdType::LoranC => Self::LoranC,
            RustEpfdType::Chayka => Self::Chayka,
            RustEpfdType::IntegratedNavigation => Self::IntegratedNavigation,
            RustEpfdType::Surveyed => Self::Surveyed,
            RustEpfdType::Galileo => Self::Galileo,
            RustEpfdType::InternalGnss => Self::InternalGnss,
            other => return Err(unsupported_variant("EpfdType", &other)),
        })
    }
}

// ---------- AisVersion ----------

/// AIS protocol version indicator (binding class for `AisVersion`). The
/// int values are the 2-bit wire codes. A plain field: every code is
/// defined.
#[pyclass(name = "AisVersion", frozen, eq, eq_int, hash, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAisVersion {
    #[pyo3(name = "ITU1371V1")]
    Itu1371v1 = 0,
    #[pyo3(name = "ITU1371V3")]
    Itu1371v3 = 1,
    #[pyo3(name = "ITU1371V5")]
    Itu1371v5 = 2,
    #[pyo3(name = "FUTURE")]
    Future = 3,
}

impl From<RustAisVersion> for PyAisVersion {
    // The Rust enum is `#[non_exhaustive]`, so the wildcard is required;
    // a future edition code is what `Future` means. Silences
    // `match_same_arms` for the `Future => Future` + `_ => Future` pair.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustAisVersion) -> Self {
        match v {
            RustAisVersion::Itu1371v1 => Self::Itu1371v1,
            RustAisVersion::Itu1371v3 => Self::Itu1371v3,
            RustAisVersion::Itu1371v5 => Self::Itu1371v5,
            RustAisVersion::Future => Self::Future,
            _ => Self::Future,
        }
    }
}

// ---------- Dimensions ----------

/// Extent of a station from its position-reference point, in metres
/// (binding class for `Dimensions`). Each of the four attributes is a
/// `marlin.field.FieldState[int]`: the wire code `0` is
/// `FieldState.NotAvailable()` and the field maximum (511 m to bow or
/// stern, 63 m to port or starboard) is the over-range bound
/// `FieldState.AtLeast(511)` / `FieldState.AtLeast(63)`.
///
/// The constructor accepts `FieldState[int] | int | None` per attribute,
/// coerces a bare value to `FieldState.Value` and `None` to
/// `FieldState.NotAvailable()`, and defaults every attribute to
/// `FieldState.NotAvailable()`.
// All four fields are `to_*` distances — the shared prefix is the wire
// shape (distance from reference point to bow/stern/port/starboard) and
// must not be stripped.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "Dimensions", frozen, eq, hash, module = "marlin.ais")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyDimensions {
    to_bow_m: FieldState<u16>,
    to_stern_m: FieldState<u16>,
    to_port_m: FieldState<u8>,
    to_starboard_m: FieldState<u8>,
}

#[pymethods]
impl PyDimensions {
    #[new]
    #[pyo3(signature = (to_bow_m = None, to_stern_m = None, to_port_m = None, to_starboard_m = None))]
    fn new(
        to_bow_m: Option<&Bound<'_, PyAny>>,
        to_stern_m: Option<&Bound<'_, PyAny>>,
        to_port_m: Option<&Bound<'_, PyAny>>,
        to_starboard_m: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            to_bow_m: field_arg(to_bow_m)?,
            to_stern_m: field_arg(to_stern_m)?,
            to_port_m: field_arg(to_port_m)?,
            to_starboard_m: field_arg(to_starboard_m)?,
        })
    }

    #[getter]
    fn to_bow_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.to_bow_m)
    }
    #[getter]
    fn to_stern_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.to_stern_m)
    }
    #[getter]
    fn to_port_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.to_port_m)
    }
    #[getter]
    fn to_starboard_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.to_starboard_m)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Dimensions(to_bow_m={}, to_stern_m={}, to_port_m={}, to_starboard_m={})",
            repr_state(py, self.to_bow_m)?,
            repr_state(py, self.to_stern_m)?,
            repr_state(py, self.to_port_m)?,
            repr_state(py, self.to_starboard_m)?,
        ))
    }
}

impl PyDimensions {
    /// Every member not available: the Python default for a message built
    /// without dimensions.
    fn all_not_available() -> Self {
        Self {
            to_bow_m: FieldState::NotAvailable,
            to_stern_m: FieldState::NotAvailable,
            to_port_m: FieldState::NotAvailable,
            to_starboard_m: FieldState::NotAvailable,
        }
    }
}

impl From<RustDimensions> for PyDimensions {
    fn from(d: RustDimensions) -> Self {
        Self {
            to_bow_m: d.to_bow_m,
            to_stern_m: d.to_stern_m,
            to_port_m: d.to_port_m,
            to_starboard_m: d.to_starboard_m,
        }
    }
}

// ---------- Eta ----------

/// Estimated time of arrival of a Type 5 report (binding class for
/// `Eta`). Each of the four attributes is a `marlin.field.FieldState[int]`
/// with its own not-available code (month `0`, day `0`, hour `24`,
/// minute `60`); a code the standard leaves undefined (month `13..=15`,
/// hour `25..=31`, minute `61..=63`) is `FieldState.Invalid(code)`.
///
/// The constructor accepts `FieldState[int] | int | None` per attribute,
/// coerces a bare value to `FieldState.Value` and `None` to
/// `FieldState.NotAvailable()`, and defaults every attribute to
/// `FieldState.NotAvailable()`.
#[pyclass(name = "Eta", frozen, eq, hash, module = "marlin.ais")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyEta {
    month: FieldState<u8>,
    day: FieldState<u8>,
    hour: FieldState<u8>,
    minute: FieldState<u8>,
}

#[pymethods]
impl PyEta {
    #[new]
    #[pyo3(signature = (month = None, day = None, hour = None, minute = None))]
    fn new(
        month: Option<&Bound<'_, PyAny>>,
        day: Option<&Bound<'_, PyAny>>,
        hour: Option<&Bound<'_, PyAny>>,
        minute: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            month: field_arg(month)?,
            day: field_arg(day)?,
            hour: field_arg(hour)?,
            minute: field_arg(minute)?,
        })
    }

    #[getter]
    fn month(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.month)
    }
    #[getter]
    fn day(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.day)
    }
    #[getter]
    fn hour(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.hour)
    }
    #[getter]
    fn minute(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.minute)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "Eta(month={}, day={}, hour={}, minute={})",
            repr_state(py, self.month)?,
            repr_state(py, self.day)?,
            repr_state(py, self.hour)?,
            repr_state(py, self.minute)?,
        ))
    }
}

impl PyEta {
    /// Every member not available: the Python default for a message built
    /// without an ETA.
    fn all_not_available() -> Self {
        Self {
            month: FieldState::NotAvailable,
            day: FieldState::NotAvailable,
            hour: FieldState::NotAvailable,
            minute: FieldState::NotAvailable,
        }
    }
}

impl From<RustEta> for PyEta {
    fn from(e: RustEta) -> Self {
        Self {
            month: e.month,
            day: e.day,
            hour: e.hour,
            minute: e.minute,
        }
    }
}

// ---------- RateOfTurn ----------

/// Rate of turn of a Class A position report (binding class for
/// `RateOfTurn`), carried as `PositionReportA.rate_of_turn:
/// FieldState[RateOfTurn]`.
///
/// One of two variant classes: `RateOfTurn.DegPerMin(deg_per_min)`, a
/// measured rate in degrees per minute, starboard positive; or
/// `RateOfTurn.NoIndicator(direction)`, the raw ±127 status "turning
/// right/left at more than 5° per 30 s, no turn indicator", a value of
/// this field and not a field state. Read one with `isinstance` or
/// `match` on the variant class.
#[pyclass(name = "RateOfTurn", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub enum PyRateOfTurn {
    /// A measured rate in degrees per minute.
    #[pyo3(name = "DegPerMin")]
    DegPerMin {
        /// The rate, starboard positive.
        deg_per_min: f32,
    },
    /// Turning at more than 5° per 30 s with no turn indicator; only
    /// the direction is known.
    #[pyo3(name = "NoIndicator")]
    NoIndicator {
        /// Which way the vessel is turning.
        direction: PyTurnDirection,
    },
}

#[pymethods]
impl PyRateOfTurn {
    // Python `==` on the payload is exact float equality; this mirrors it.
    #[allow(clippy::float_cmp)]
    fn __eq__(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::DegPerMin { deg_per_min: a }, Self::DegPerMin { deg_per_min: b }) => a == b,
            (Self::NoIndicator { direction: a }, Self::NoIndicator { direction: b }) => a == b,
            _ => false,
        }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        let payload = match self {
            Self::DegPerMin { deg_per_min } => deg_per_min.into_py_any(py)?,
            Self::NoIndicator { direction } => direction.into_py_any(py)?,
        };
        PyTuple::new(py, [self.variant_name().into_py_any(py)?, payload])?.hash()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let payload = match self {
            Self::DegPerMin { deg_per_min } => {
                deg_per_min.into_bound_py_any(py)?.repr()?.to_string()
            }
            Self::NoIndicator { direction } => direction.into_bound_py_any(py)?.repr()?.to_string(),
        };
        Ok(format!("RateOfTurn.{}({payload})", self.variant_name()))
    }
}

impl PyRateOfTurn {
    const VARIANTS: [&'static str; 2] = ["DegPerMin", "NoIndicator"];

    fn variant_name(&self) -> &'static str {
        match self {
            Self::DegPerMin { .. } => "DegPerMin",
            Self::NoIndicator { .. } => "NoIndicator",
        }
    }
}

impl From<RustRateOfTurn> for PyRateOfTurn {
    fn from(v: RustRateOfTurn) -> Self {
        match v {
            RustRateOfTurn::DegPerMin(deg_per_min) => Self::DegPerMin { deg_per_min },
            RustRateOfTurn::NoIndicator(direction) => Self::NoIndicator {
                direction: direction.into(),
            },
        }
    }
}

// ---------- Timestamp ----------

/// The timestamp field of a position report (binding class for
/// `Timestamp`), carried as `timestamp: FieldState[Timestamp]` on Types
/// 1/2/3, 9, 18, 19 and 21.
///
/// One of two variant classes: `Timestamp.Second(second)`, the UTC
/// second of the position fix (0..=59); or
/// `Timestamp.PositioningStatus(status)`, the wire codes 61..=63 that
/// report the positioning system's status instead of a second, a value
/// of this field and not a field state. The code 60 is
/// `FieldState.NotAvailable()` on the message. Read one with
/// `isinstance` or `match` on the variant class.
#[pyclass(name = "Timestamp", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub enum PyTimestamp {
    /// The UTC second within the minute of the position fix.
    #[pyo3(name = "Second")]
    Second {
        /// The second, 0..=59.
        second: u8,
    },
    /// The positioning system reports a status instead of a second.
    #[pyo3(name = "PositioningStatus")]
    PositioningStatus {
        /// The status.
        status: PyPositioningStatus,
    },
}

#[pymethods]
impl PyTimestamp {
    fn __eq__(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Second { second: a }, Self::Second { second: b }) => a == b,
            (Self::PositioningStatus { status: a }, Self::PositioningStatus { status: b }) => {
                a == b
            }
            _ => false,
        }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        let payload = match self {
            Self::Second { second } => second.into_py_any(py)?,
            Self::PositioningStatus { status } => status.into_py_any(py)?,
        };
        PyTuple::new(py, [self.variant_name().into_py_any(py)?, payload])?.hash()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let payload = match self {
            Self::Second { second } => second.to_string(),
            Self::PositioningStatus { status } => status.into_bound_py_any(py)?.repr()?.to_string(),
        };
        Ok(format!("Timestamp.{}({payload})", self.variant_name()))
    }
}

impl PyTimestamp {
    const VARIANTS: [&'static str; 2] = ["Second", "PositioningStatus"];

    fn variant_name(&self) -> &'static str {
        match self {
            Self::Second { .. } => "Second",
            Self::PositioningStatus { .. } => "PositioningStatus",
        }
    }
}

impl From<RustTimestamp> for PyTimestamp {
    fn from(v: RustTimestamp) -> Self {
        match v {
            RustTimestamp::Second(second) => Self::Second { second },
            RustTimestamp::PositioningStatus(status) => Self::PositioningStatus {
                status: status.into(),
            },
        }
    }
}

// ---------- Type24BExtent ----------

/// What the 30-bit extent field of a Type 24 Part B holds (binding class
/// for `Type24BExtent`), carried as `StaticDataB24B.extent`.
///
/// One of two variant classes, decided by the MMSI prefix (ADR-0002):
/// `Type24BExtent.Dimensions(dimensions)` for every ordinary MMSI, or
/// `Type24BExtent.MothershipMmsi(mmsi)` for an auxiliary craft
/// (`98MIDxxxx`), whose 30 bits hold the mother ship's MMSI instead. A
/// plain field: the wire cannot leave it without a value. Read one with
/// `isinstance` or `match` on the variant class.
#[pyclass(name = "Type24BExtent", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub enum PyType24BExtent {
    /// Dimensions A/B/C/D, for every MMSI that is not an auxiliary craft.
    #[pyo3(name = "Dimensions")]
    Dimensions {
        /// The extent.
        dimensions: PyDimensions,
    },
    /// MMSI of the mother ship, for an auxiliary-craft MMSI.
    #[pyo3(name = "MothershipMmsi")]
    MothershipMmsi {
        /// The mother ship's MMSI.
        mmsi: u32,
    },
}

#[pymethods]
impl PyType24BExtent {
    fn __eq__(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Dimensions { dimensions: a }, Self::Dimensions { dimensions: b }) => a == b,
            (Self::MothershipMmsi { mmsi: a }, Self::MothershipMmsi { mmsi: b }) => a == b,
            _ => false,
        }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        let payload = match self {
            Self::Dimensions { dimensions } => dimensions.clone().into_py_any(py)?,
            Self::MothershipMmsi { mmsi } => mmsi.into_py_any(py)?,
        };
        PyTuple::new(py, [self.variant_name().into_py_any(py)?, payload])?.hash()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let payload = match self {
            Self::Dimensions { dimensions } => dimensions.__repr__(py)?,
            Self::MothershipMmsi { mmsi } => mmsi.to_string(),
        };
        Ok(format!("Type24BExtent.{}({payload})", self.variant_name()))
    }
}

impl PyType24BExtent {
    const VARIANTS: [&'static str; 2] = ["Dimensions", "MothershipMmsi"];

    fn variant_name(&self) -> &'static str {
        match self {
            Self::Dimensions { .. } => "Dimensions",
            Self::MothershipMmsi { .. } => "MothershipMmsi",
        }
    }
}

impl From<RustType24BExtent> for PyType24BExtent {
    fn from(v: RustType24BExtent) -> Self {
        match v {
            RustType24BExtent::Dimensions(dimensions) => Self::Dimensions {
                dimensions: dimensions.into(),
            },
            RustType24BExtent::MothershipMmsi(mmsi) => Self::MothershipMmsi { mmsi },
        }
    }
}

// ---------- PositionReportA (Types 1/2/3) ----------

/// Class A position report payload (Types 1, 2 and 3). The message-type
/// distinction is preserved as `AisMessage.type_tag`, not here.
///
/// Every attribute the wire can leave without a value is a
/// `marlin.field.FieldState`; the one-bit flags, `mmsi` and
/// `radio_status` are plain. `rate_of_turn` and `timestamp` carry the
/// sum types `RateOfTurn` and `Timestamp`. The constructor accepts
/// `FieldState[T] | T | None` per field-state attribute, coerces a bare
/// value to `FieldState.Value` and `None` to `FieldState.NotAvailable()`,
/// and defaults every such keyword to `FieldState.NotAvailable()`.
#[pyclass(name = "PositionReportA", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyPositionReportA {
    #[pyo3(get)]
    mmsi: u32,
    navigation_status: FieldState<PyNavStatus>,
    rate_of_turn: FieldState<PyRateOfTurn>,
    speed_over_ground: FieldState<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    longitude_deg: FieldState<f64>,
    latitude_deg: FieldState<f64>,
    course_over_ground: FieldState<f32>,
    true_heading: FieldState<u16>,
    timestamp: FieldState<PyTimestamp>,
    special_maneuver: FieldState<PyManeuverIndicator>,
    #[pyo3(get)]
    raim: bool,
    #[pyo3(get)]
    radio_status: u32,
}

#[pymethods]
impl PyPositionReportA {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        mmsi = 0,
        navigation_status = None,
        rate_of_turn = None,
        speed_over_ground = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        course_over_ground = None,
        true_heading = None,
        timestamp = None,
        special_maneuver = None,
        raim = false,
        radio_status = 0,
    ))]
    fn new(
        mmsi: u32,
        navigation_status: Option<&Bound<'_, PyAny>>,
        rate_of_turn: Option<&Bound<'_, PyAny>>,
        speed_over_ground: Option<&Bound<'_, PyAny>>,
        position_accuracy: bool,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        course_over_ground: Option<&Bound<'_, PyAny>>,
        true_heading: Option<&Bound<'_, PyAny>>,
        timestamp: Option<&Bound<'_, PyAny>>,
        special_maneuver: Option<&Bound<'_, PyAny>>,
        raim: bool,
        radio_status: u32,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            navigation_status: field_arg(navigation_status)?,
            rate_of_turn: field_arg(rate_of_turn)?,
            speed_over_ground: field_arg(speed_over_ground)?,
            position_accuracy,
            longitude_deg: field_arg(longitude_deg)?,
            latitude_deg: field_arg(latitude_deg)?,
            course_over_ground: field_arg(course_over_ground)?,
            true_heading: field_arg(true_heading)?,
            timestamp: field_arg(timestamp)?,
            special_maneuver: field_arg(special_maneuver)?,
            raim,
            radio_status,
        })
    }

    #[getter]
    fn navigation_status(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.navigation_status)
    }
    #[getter]
    fn rate_of_turn(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.rate_of_turn.clone())
    }
    #[getter]
    fn speed_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_over_ground)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn course_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_over_ground)
    }
    #[getter]
    fn true_heading(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.true_heading)
    }
    #[getter]
    fn timestamp(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.timestamp.clone())
    }
    #[getter]
    fn special_maneuver(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.special_maneuver)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "PositionReportA(mmsi={}, lat={}, lon={}, sog={})",
            self.mmsi,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
            repr_state(py, self.speed_over_ground)?,
        ))
    }
}

impl TryFrom<RustPositionReportA> for PyPositionReportA {
    type Error = PyErr;

    fn try_from(d: RustPositionReportA) -> PyResult<Self> {
        Ok(Self {
            mmsi: d.mmsi,
            navigation_status: enum_state(d.navigation_status)?,
            rate_of_turn: d.rate_of_turn.map(PyRateOfTurn::from),
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            true_heading: d.true_heading,
            timestamp: d.timestamp.map(PyTimestamp::from),
            special_maneuver: enum_state(d.special_maneuver)?,
            raim: d.raim,
            radio_status: d.radio_status,
        })
    }
}

// ---------- StaticAndVoyageA (Type 5) ----------

/// Class A static and voyage data payload (Type 5).
///
/// Every attribute the wire can leave without a value is a
/// `marlin.field.FieldState`, `dte` included: a 420- or 422-bit payload
/// ends before the DTE bit, so `dte` is `FieldState.NotAvailable()`
/// there. `dimensions` and `eta` carry a field state per member. The
/// constructor accepts `FieldState[T] | T | None` per field-state
/// attribute and defaults every such keyword to
/// `FieldState.NotAvailable()`; `dimensions` and `eta` default to all
/// members not available.
#[pyclass(name = "StaticAndVoyageA", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticAndVoyageA {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    ais_version: PyAisVersion,
    imo_number: FieldState<u32>,
    call_sign: FieldState<String>,
    vessel_name: FieldState<String>,
    ship_type: FieldState<u8>,
    #[pyo3(get)]
    dimensions: PyDimensions,
    epfd: FieldState<PyEpfdType>,
    #[pyo3(get)]
    eta: PyEta,
    draught_m: FieldState<f32>,
    destination: FieldState<String>,
    dte: FieldState<bool>,
}

#[pymethods]
impl PyStaticAndVoyageA {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        mmsi = 0,
        ais_version = PyAisVersion::Future,
        imo_number = None,
        call_sign = None,
        vessel_name = None,
        ship_type = None,
        dimensions = None,
        epfd = None,
        eta = None,
        draught_m = None,
        destination = None,
        dte = None,
    ))]
    fn new(
        mmsi: u32,
        ais_version: PyAisVersion,
        imo_number: Option<&Bound<'_, PyAny>>,
        call_sign: Option<&Bound<'_, PyAny>>,
        vessel_name: Option<&Bound<'_, PyAny>>,
        ship_type: Option<&Bound<'_, PyAny>>,
        dimensions: Option<PyDimensions>,
        epfd: Option<&Bound<'_, PyAny>>,
        eta: Option<PyEta>,
        draught_m: Option<&Bound<'_, PyAny>>,
        destination: Option<&Bound<'_, PyAny>>,
        dte: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            ais_version,
            imo_number: field_arg(imo_number)?,
            call_sign: field_arg(call_sign)?,
            vessel_name: field_arg(vessel_name)?,
            ship_type: field_arg(ship_type)?,
            dimensions: dimensions.unwrap_or_else(PyDimensions::all_not_available),
            epfd: field_arg(epfd)?,
            eta: eta.unwrap_or_else(PyEta::all_not_available),
            draught_m: field_arg(draught_m)?,
            destination: field_arg(destination)?,
            dte: field_arg(dte)?,
        })
    }

    #[getter]
    fn imo_number(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.imo_number)
    }
    #[getter]
    fn call_sign(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.call_sign.clone())
    }
    #[getter]
    fn vessel_name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.vessel_name.clone())
    }
    #[getter]
    fn ship_type(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.ship_type)
    }
    #[getter]
    fn epfd(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.epfd)
    }
    #[getter]
    fn draught_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.draught_m)
    }
    #[getter]
    fn destination(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.destination.clone())
    }
    #[getter]
    fn dte(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.dte)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "StaticAndVoyageA(mmsi={}, vessel_name={}, destination={})",
            self.mmsi,
            repr_state(py, self.vessel_name.clone())?,
            repr_state(py, self.destination.clone())?,
        ))
    }
}

impl TryFrom<RustStaticAndVoyageA> for PyStaticAndVoyageA {
    type Error = PyErr;

    fn try_from(d: RustStaticAndVoyageA) -> PyResult<Self> {
        Ok(Self {
            mmsi: d.mmsi,
            ais_version: d.ais_version.into(),
            imo_number: d.imo_number,
            call_sign: d.call_sign,
            vessel_name: d.vessel_name,
            ship_type: d.ship_type,
            dimensions: d.dimensions.into(),
            epfd: enum_state(d.epfd)?,
            eta: d.eta.into(),
            draught_m: d.draught_m,
            destination: d.destination,
            dte: d.dte,
        })
    }
}

// ---------- SarAircraftPositionReport (Type 9) ----------

/// Standard SAR aircraft position report payload (Type 9).
///
/// `altitude_m` and `speed_over_ground` are `FieldState[int]` in whole
/// metres / whole knots (not 0.1 kn as on the vessel reports): the codes
/// 4095 / 1023 are `FieldState.NotAvailable()` and the over-range codes
/// 4094 / 1022 are `FieldState.AtLeast(4094)` / `FieldState.AtLeast(1022)`.
/// No heading, rate of turn or navigational status exists on Type 9.
/// `dte` is a plain `bool`: the 168-bit floor covers the bit. The
/// constructor accepts `FieldState[T] | T | None` per field-state
/// attribute and defaults every such keyword to
/// `FieldState.NotAvailable()`.
// 4 bools (`position_accuracy`, `dte`, `assigned_flag`, `raim`) are
// ITU-R M.1371 wire-format flags — the wire reality.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "SarAircraftPositionReport", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PySarAircraftPositionReport {
    #[pyo3(get)]
    mmsi: u32,
    altitude_m: FieldState<u16>,
    speed_over_ground: FieldState<u16>,
    #[pyo3(get)]
    position_accuracy: bool,
    longitude_deg: FieldState<f64>,
    latitude_deg: FieldState<f64>,
    course_over_ground: FieldState<f32>,
    timestamp: FieldState<PyTimestamp>,
    #[pyo3(get)]
    altitude_sensor: PyAltitudeSensor,
    #[pyo3(get)]
    dte: bool,
    #[pyo3(get)]
    assigned_flag: bool,
    #[pyo3(get)]
    raim: bool,
    #[pyo3(get)]
    radio_status: u32,
}

#[pymethods]
impl PySarAircraftPositionReport {
    #[new]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    #[pyo3(signature = (
        mmsi = 0,
        altitude_m = None,
        speed_over_ground = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        course_over_ground = None,
        timestamp = None,
        altitude_sensor = PyAltitudeSensor::Gnss,
        dte = false,
        assigned_flag = false,
        raim = false,
        radio_status = 0,
    ))]
    fn new(
        mmsi: u32,
        altitude_m: Option<&Bound<'_, PyAny>>,
        speed_over_ground: Option<&Bound<'_, PyAny>>,
        position_accuracy: bool,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        course_over_ground: Option<&Bound<'_, PyAny>>,
        timestamp: Option<&Bound<'_, PyAny>>,
        altitude_sensor: PyAltitudeSensor,
        dte: bool,
        assigned_flag: bool,
        raim: bool,
        radio_status: u32,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            altitude_m: field_arg(altitude_m)?,
            speed_over_ground: field_arg(speed_over_ground)?,
            position_accuracy,
            longitude_deg: field_arg(longitude_deg)?,
            latitude_deg: field_arg(latitude_deg)?,
            course_over_ground: field_arg(course_over_ground)?,
            timestamp: field_arg(timestamp)?,
            altitude_sensor,
            dte,
            assigned_flag,
            raim,
            radio_status,
        })
    }

    #[getter]
    fn altitude_m(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.altitude_m)
    }
    #[getter]
    fn speed_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_over_ground)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn course_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_over_ground)
    }
    #[getter]
    fn timestamp(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.timestamp.clone())
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "SarAircraftPositionReport(mmsi={}, altitude_m={}, lat={}, lon={})",
            self.mmsi,
            repr_state(py, self.altitude_m)?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl From<RustSarAircraftPositionReport> for PySarAircraftPositionReport {
    fn from(d: RustSarAircraftPositionReport) -> Self {
        Self {
            mmsi: d.mmsi,
            altitude_m: d.altitude_m,
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            timestamp: d.timestamp.map(PyTimestamp::from),
            altitude_sensor: d.altitude_sensor.into(),
            dte: d.dte,
            assigned_flag: d.assigned_flag,
            raim: d.raim,
            radio_status: d.radio_status,
        }
    }
}

// ---------- PositionReportB (Type 18) ----------

/// Class B CS position report payload (Type 18).
///
/// The position, speed, course, heading and timestamp attributes are
/// `marlin.field.FieldState`s as on `PositionReportA`; the Class B
/// capability flags are plain. The constructor accepts
/// `FieldState[T] | T | None` per field-state attribute and defaults
/// every such keyword to `FieldState.NotAvailable()`.
// The five `class_b_*_flag` bits are ITU-R M.1371 wire-format flags;
// bundling them is the wire reality, so silence `struct_excessive_bools`.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "PositionReportB", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyPositionReportB {
    #[pyo3(get)]
    mmsi: u32,
    speed_over_ground: FieldState<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    longitude_deg: FieldState<f64>,
    latitude_deg: FieldState<f64>,
    course_over_ground: FieldState<f32>,
    true_heading: FieldState<u16>,
    timestamp: FieldState<PyTimestamp>,
    #[pyo3(get)]
    class_b_cs_flag: bool,
    #[pyo3(get)]
    class_b_display_flag: bool,
    #[pyo3(get)]
    class_b_dsc_flag: bool,
    #[pyo3(get)]
    class_b_band_flag: bool,
    #[pyo3(get)]
    class_b_message22_flag: bool,
    #[pyo3(get)]
    assigned_flag: bool,
    #[pyo3(get)]
    raim: bool,
    #[pyo3(get)]
    radio_status: u32,
}

#[pymethods]
impl PyPositionReportB {
    #[new]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    #[pyo3(signature = (
        mmsi = 0,
        speed_over_ground = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        course_over_ground = None,
        true_heading = None,
        timestamp = None,
        class_b_cs_flag = false,
        class_b_display_flag = false,
        class_b_dsc_flag = false,
        class_b_band_flag = false,
        class_b_message22_flag = false,
        assigned_flag = false,
        raim = false,
        radio_status = 0,
    ))]
    fn new(
        mmsi: u32,
        speed_over_ground: Option<&Bound<'_, PyAny>>,
        position_accuracy: bool,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        course_over_ground: Option<&Bound<'_, PyAny>>,
        true_heading: Option<&Bound<'_, PyAny>>,
        timestamp: Option<&Bound<'_, PyAny>>,
        class_b_cs_flag: bool,
        class_b_display_flag: bool,
        class_b_dsc_flag: bool,
        class_b_band_flag: bool,
        class_b_message22_flag: bool,
        assigned_flag: bool,
        raim: bool,
        radio_status: u32,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            speed_over_ground: field_arg(speed_over_ground)?,
            position_accuracy,
            longitude_deg: field_arg(longitude_deg)?,
            latitude_deg: field_arg(latitude_deg)?,
            course_over_ground: field_arg(course_over_ground)?,
            true_heading: field_arg(true_heading)?,
            timestamp: field_arg(timestamp)?,
            class_b_cs_flag,
            class_b_display_flag,
            class_b_dsc_flag,
            class_b_band_flag,
            class_b_message22_flag,
            assigned_flag,
            raim,
            radio_status,
        })
    }

    #[getter]
    fn speed_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_over_ground)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn course_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_over_ground)
    }
    #[getter]
    fn true_heading(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.true_heading)
    }
    #[getter]
    fn timestamp(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.timestamp.clone())
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "PositionReportB(mmsi={}, lat={}, lon={}, sog={})",
            self.mmsi,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
            repr_state(py, self.speed_over_ground)?,
        ))
    }
}

impl From<RustPositionReportB> for PyPositionReportB {
    fn from(d: RustPositionReportB) -> Self {
        Self {
            mmsi: d.mmsi,
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            true_heading: d.true_heading,
            timestamp: d.timestamp.map(PyTimestamp::from),
            class_b_cs_flag: d.class_b_cs_flag,
            class_b_display_flag: d.class_b_display_flag,
            class_b_dsc_flag: d.class_b_dsc_flag,
            class_b_band_flag: d.class_b_band_flag,
            class_b_message22_flag: d.class_b_message22_flag,
            assigned_flag: d.assigned_flag,
            raim: d.raim,
            radio_status: d.radio_status,
        }
    }
}

// ---------- ExtendedPositionReportB (Type 19) ----------

/// Class B extended position report payload (Type 19).
///
/// The Type 18 position attributes plus the Type 5 static tail
/// (`vessel_name`, `ship_type`, `dimensions`, `epfd`), each a
/// `marlin.field.FieldState` where the wire can leave it without a
/// value. `dte` is a plain `bool`: the 312-bit floor covers the bit.
/// The constructor accepts `FieldState[T] | T | None` per field-state
/// attribute and defaults every such keyword to
/// `FieldState.NotAvailable()`; `dimensions` defaults to all members
/// not available.
// 4 bools (`position_accuracy`, `raim`, `dte`, `assigned_flag`) are
// ITU-R M.1371 wire-format flags — the wire reality.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "ExtendedPositionReportB", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyExtendedPositionReportB {
    #[pyo3(get)]
    mmsi: u32,
    speed_over_ground: FieldState<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    longitude_deg: FieldState<f64>,
    latitude_deg: FieldState<f64>,
    course_over_ground: FieldState<f32>,
    true_heading: FieldState<u16>,
    timestamp: FieldState<PyTimestamp>,
    vessel_name: FieldState<String>,
    ship_type: FieldState<u8>,
    #[pyo3(get)]
    dimensions: PyDimensions,
    epfd: FieldState<PyEpfdType>,
    #[pyo3(get)]
    raim: bool,
    #[pyo3(get)]
    dte: bool,
    #[pyo3(get)]
    assigned_flag: bool,
}

#[pymethods]
impl PyExtendedPositionReportB {
    #[new]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    #[pyo3(signature = (
        mmsi = 0,
        speed_over_ground = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        course_over_ground = None,
        true_heading = None,
        timestamp = None,
        vessel_name = None,
        ship_type = None,
        dimensions = None,
        epfd = None,
        raim = false,
        dte = false,
        assigned_flag = false,
    ))]
    fn new(
        mmsi: u32,
        speed_over_ground: Option<&Bound<'_, PyAny>>,
        position_accuracy: bool,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        course_over_ground: Option<&Bound<'_, PyAny>>,
        true_heading: Option<&Bound<'_, PyAny>>,
        timestamp: Option<&Bound<'_, PyAny>>,
        vessel_name: Option<&Bound<'_, PyAny>>,
        ship_type: Option<&Bound<'_, PyAny>>,
        dimensions: Option<PyDimensions>,
        epfd: Option<&Bound<'_, PyAny>>,
        raim: bool,
        dte: bool,
        assigned_flag: bool,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            speed_over_ground: field_arg(speed_over_ground)?,
            position_accuracy,
            longitude_deg: field_arg(longitude_deg)?,
            latitude_deg: field_arg(latitude_deg)?,
            course_over_ground: field_arg(course_over_ground)?,
            true_heading: field_arg(true_heading)?,
            timestamp: field_arg(timestamp)?,
            vessel_name: field_arg(vessel_name)?,
            ship_type: field_arg(ship_type)?,
            dimensions: dimensions.unwrap_or_else(PyDimensions::all_not_available),
            epfd: field_arg(epfd)?,
            raim,
            dte,
            assigned_flag,
        })
    }

    #[getter]
    fn speed_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.speed_over_ground)
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn course_over_ground(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.course_over_ground)
    }
    #[getter]
    fn true_heading(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.true_heading)
    }
    #[getter]
    fn timestamp(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.timestamp.clone())
    }
    #[getter]
    fn vessel_name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.vessel_name.clone())
    }
    #[getter]
    fn ship_type(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.ship_type)
    }
    #[getter]
    fn epfd(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.epfd)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "ExtendedPositionReportB(mmsi={}, vessel_name={}, lat={}, lon={})",
            self.mmsi,
            repr_state(py, self.vessel_name.clone())?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl TryFrom<RustExtendedPositionReportB> for PyExtendedPositionReportB {
    type Error = PyErr;

    fn try_from(d: RustExtendedPositionReportB) -> PyResult<Self> {
        Ok(Self {
            mmsi: d.mmsi,
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            true_heading: d.true_heading,
            timestamp: d.timestamp.map(PyTimestamp::from),
            vessel_name: d.vessel_name,
            ship_type: d.ship_type,
            dimensions: d.dimensions.into(),
            epfd: enum_state(d.epfd)?,
            raim: d.raim,
            dte: d.dte,
            assigned_flag: d.assigned_flag,
        })
    }
}

// ---------- AidToNavigationReport (Type 21) ----------

/// Aid-to-navigation report payload (Type 21).
///
/// `name` is the 20-character name joined with the optional extension
/// (up to 14 more characters) and trimmed of trailing `@` / spaces; an
/// `@` inside the name is kept; all padding is `FieldState.NotAvailable()`.
/// `dimensions` carries a field state per member and is all not
/// available for virtual AtoN and reference points; `aton_status` is
/// the plain 8-bit field. The constructor accepts
/// `FieldState[T] | T | None` per field-state attribute and defaults
/// every such keyword to `FieldState.NotAvailable()`; `dimensions`
/// defaults to all members not available.
// 5 bools (`position_accuracy`, `off_position`, `raim`, `virtual_aton`,
// `assigned_flag`) are ITU-R M.1371 wire-format flags — the wire reality.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "AidToNavigationReport", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyAidToNavigationReport {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    aton_type: PyAtonType,
    name: FieldState<String>,
    #[pyo3(get)]
    position_accuracy: bool,
    longitude_deg: FieldState<f64>,
    latitude_deg: FieldState<f64>,
    #[pyo3(get)]
    dimensions: PyDimensions,
    epfd: FieldState<PyEpfdType>,
    timestamp: FieldState<PyTimestamp>,
    #[pyo3(get)]
    off_position: bool,
    #[pyo3(get)]
    aton_status: u8,
    #[pyo3(get)]
    raim: bool,
    #[pyo3(get)]
    virtual_aton: bool,
    #[pyo3(get)]
    assigned_flag: bool,
}

#[pymethods]
impl PyAidToNavigationReport {
    #[new]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    #[pyo3(signature = (
        mmsi = 0,
        aton_type = PyAtonType::NotSpecified,
        name = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        dimensions = None,
        epfd = None,
        timestamp = None,
        off_position = false,
        aton_status = 0,
        raim = false,
        virtual_aton = false,
        assigned_flag = false,
    ))]
    fn new(
        mmsi: u32,
        aton_type: PyAtonType,
        name: Option<&Bound<'_, PyAny>>,
        position_accuracy: bool,
        longitude_deg: Option<&Bound<'_, PyAny>>,
        latitude_deg: Option<&Bound<'_, PyAny>>,
        dimensions: Option<PyDimensions>,
        epfd: Option<&Bound<'_, PyAny>>,
        timestamp: Option<&Bound<'_, PyAny>>,
        off_position: bool,
        aton_status: u8,
        raim: bool,
        virtual_aton: bool,
        assigned_flag: bool,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            aton_type,
            name: field_arg(name)?,
            position_accuracy,
            longitude_deg: field_arg(longitude_deg)?,
            latitude_deg: field_arg(latitude_deg)?,
            dimensions: dimensions.unwrap_or_else(PyDimensions::all_not_available),
            epfd: field_arg(epfd)?,
            timestamp: field_arg(timestamp)?,
            off_position,
            aton_status,
            raim,
            virtual_aton,
            assigned_flag,
        })
    }

    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.name.clone())
    }
    #[getter]
    fn longitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.longitude_deg)
    }
    #[getter]
    fn latitude_deg(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.latitude_deg)
    }
    #[getter]
    fn epfd(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.epfd)
    }
    #[getter]
    fn timestamp(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.timestamp.clone())
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "AidToNavigationReport(mmsi={}, aton_type={:?}, name={}, lat={}, lon={})",
            self.mmsi,
            self.aton_type,
            repr_state(py, self.name.clone())?,
            repr_state(py, self.latitude_deg)?,
            repr_state(py, self.longitude_deg)?,
        ))
    }
}

impl TryFrom<RustAidToNavigationReport> for PyAidToNavigationReport {
    type Error = PyErr;

    fn try_from(d: RustAidToNavigationReport) -> PyResult<Self> {
        Ok(Self {
            mmsi: d.mmsi,
            aton_type: d.aton_type.into(),
            name: d.name,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            dimensions: d.dimensions.into(),
            epfd: enum_state(d.epfd)?,
            timestamp: d.timestamp.map(PyTimestamp::from),
            off_position: d.off_position,
            aton_status: d.aton_status,
            raim: d.raim,
            virtual_aton: d.virtual_aton,
            assigned_flag: d.assigned_flag,
        })
    }
}

// ---------- StaticDataB24A (Type 24 Part A) ----------

/// Class B static data Part A payload (Type 24 Part A): the vessel name
/// as a `marlin.field.FieldState[str]`, `FieldState.NotAvailable()` on
/// all padding. The constructor accepts `FieldState[str] | str | None`
/// and defaults to `FieldState.NotAvailable()`.
#[pyclass(name = "StaticDataB24A", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticDataB24A {
    #[pyo3(get)]
    mmsi: u32,
    vessel_name: FieldState<String>,
}

#[pymethods]
impl PyStaticDataB24A {
    #[new]
    #[pyo3(signature = (mmsi = 0, vessel_name = None))]
    fn new(mmsi: u32, vessel_name: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            vessel_name: field_arg(vessel_name)?,
        })
    }

    #[getter]
    fn vessel_name(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.vessel_name.clone())
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "StaticDataB24A(mmsi={}, vessel_name={})",
            self.mmsi,
            repr_state(py, self.vessel_name.clone())?,
        ))
    }
}

impl From<RustStaticDataB24A> for PyStaticDataB24A {
    fn from(d: RustStaticDataB24A) -> Self {
        Self {
            mmsi: d.mmsi,
            vessel_name: d.vessel_name,
        }
    }
}

// ---------- StaticDataB24B (Type 24 Part B) ----------

/// Class B static data Part B payload (Type 24 Part B).
///
/// `extent` is a `Type24BExtent`: `Type24BExtent.Dimensions(dimensions)`
/// for every ordinary MMSI, `Type24BExtent.MothershipMmsi(mmsi)` for an
/// auxiliary craft (`98MIDxxxx`, ADR-0002). Every other attribute the
/// wire can leave without a value is a `marlin.field.FieldState`. The
/// constructor accepts `FieldState[T] | T | None` per field-state
/// attribute and defaults every such keyword to
/// `FieldState.NotAvailable()`; `extent` defaults to dimensions with
/// every member not available.
#[pyclass(name = "StaticDataB24B", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticDataB24B {
    #[pyo3(get)]
    mmsi: u32,
    ship_type: FieldState<u8>,
    vendor_id: FieldState<String>,
    call_sign: FieldState<String>,
    #[pyo3(get)]
    extent: PyType24BExtent,
    epfd: FieldState<PyEpfdType>,
}

#[pymethods]
impl PyStaticDataB24B {
    #[new]
    #[pyo3(signature = (
        mmsi = 0,
        ship_type = None,
        vendor_id = None,
        call_sign = None,
        extent = None,
        epfd = None,
    ))]
    fn new(
        mmsi: u32,
        ship_type: Option<&Bound<'_, PyAny>>,
        vendor_id: Option<&Bound<'_, PyAny>>,
        call_sign: Option<&Bound<'_, PyAny>>,
        extent: Option<PyType24BExtent>,
        epfd: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            mmsi,
            ship_type: field_arg(ship_type)?,
            vendor_id: field_arg(vendor_id)?,
            call_sign: field_arg(call_sign)?,
            extent: extent.unwrap_or_else(|| PyType24BExtent::Dimensions {
                dimensions: PyDimensions::all_not_available(),
            }),
            epfd: field_arg(epfd)?,
        })
    }

    #[getter]
    fn ship_type(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.ship_type)
    }
    #[getter]
    fn vendor_id(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.vendor_id.clone())
    }
    #[getter]
    fn call_sign(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.call_sign.clone())
    }
    #[getter]
    fn epfd(&self, py: Python<'_>) -> PyResult<PyFieldState> {
        to_py(py, self.epfd)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "StaticDataB24B(mmsi={}, call_sign={}, vendor_id={})",
            self.mmsi,
            repr_state(py, self.call_sign.clone())?,
            repr_state(py, self.vendor_id.clone())?,
        ))
    }
}

impl TryFrom<RustStaticDataB24B> for PyStaticDataB24B {
    type Error = PyErr;

    fn try_from(d: RustStaticDataB24B) -> PyResult<Self> {
        Ok(Self {
            mmsi: d.mmsi,
            ship_type: d.ship_type,
            vendor_id: d.vendor_id,
            call_sign: d.call_sign,
            extent: d.extent.into(),
            epfd: enum_state(d.epfd)?,
        })
    }
}

// ---------- Other (catch-all for un-decoded msg_type) ----------

/// Catch-all for AIS message types this crate does not yet decode.
/// Preserves the raw bit buffer and total bit count so callers can plug
/// in their own decoder.
#[pyclass(name = "Other", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyOther {
    #[pyo3(get)]
    msg_type: u8,
    // Not `#[pyo3(get)]` — needs `PyBytes` conversion via manual getter.
    raw_payload: Vec<u8>,
    #[pyo3(get)]
    total_bits: usize,
}

#[pymethods]
impl PyOther {
    #[new]
    #[pyo3(signature = (msg_type = 0, raw_payload = None, total_bits = 0))]
    fn new(msg_type: u8, raw_payload: Option<Vec<u8>>, total_bits: usize) -> Self {
        Self {
            msg_type,
            raw_payload: raw_payload.unwrap_or_default(),
            total_bits,
        }
    }

    #[getter]
    fn raw_payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.raw_payload)
    }

    fn __repr__(&self) -> String {
        format!(
            "Other(msg_type={}, total_bits={}, raw_payload_len={})",
            self.msg_type,
            self.total_bits,
            self.raw_payload.len(),
        )
    }
}

// ---------- AisMessageBody dispatch helper ----------

/// Convert a `marlin_ais::AisMessageBody` into a typed Python object.
///
/// Types 1/2/3 all produce `PyPositionReportA` — the variant
/// distinction is preserved at the `AisMessage` wrapper level, not
/// here. Unknown future variants of the `#[non_exhaustive]` upstream
/// enum surface as `PyValueError` so the binding can be updated
/// deliberately rather than silently misrouting payloads.
pub(crate) fn message_body_to_py(py: Python<'_>, body: AisMessageBody) -> PyResult<Py<PyAny>> {
    Ok(match body {
        AisMessageBody::Type1(d) | AisMessageBody::Type2(d) | AisMessageBody::Type3(d) => {
            Py::new(py, PyPositionReportA::try_from(d)?)?.into_any()
        }
        AisMessageBody::Type5(d) => Py::new(py, PyStaticAndVoyageA::try_from(d)?)?.into_any(),
        AisMessageBody::Type9(d) => {
            Py::new(py, PySarAircraftPositionReport::from(d))?.into_any()
        }
        AisMessageBody::Type18(d) => Py::new(py, PyPositionReportB::from(d))?.into_any(),
        AisMessageBody::Type19(d) => {
            Py::new(py, PyExtendedPositionReportB::try_from(d)?)?.into_any()
        }
        AisMessageBody::Type21(d) => {
            Py::new(py, PyAidToNavigationReport::try_from(d)?)?.into_any()
        }
        AisMessageBody::Type24A(d) => Py::new(py, PyStaticDataB24A::from(d))?.into_any(),
        AisMessageBody::Type24B(d) => Py::new(py, PyStaticDataB24B::try_from(d)?)?.into_any(),
        AisMessageBody::Other {
            msg_type,
            raw_payload,
            total_bits,
        } => Py::new(
            py,
            PyOther {
                msg_type,
                raw_payload,
                total_bits,
            },
        )?
        .into_any(),
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "marlin Python bindings encountered an unsupported AisMessageBody variant — bindings need updating",
            ))
        }
    })
}

// ---------- AisMessage (wrapper) ----------

/// Typed AIS message as surfaced by the parser: `is_own_ship` flag,
/// a string discriminator preserving the underlying AIS message type
/// (since Types 1/2/3 share one body struct), and the decoded body
/// pyclass.
///
/// Construct in Python for testing or via `from_rust` from the parser.
#[pyclass(name = "AisMessage", frozen, module = "marlin.ais")]
#[derive(Debug)]
pub struct PyAisMessage {
    is_own_ship: bool,
    type_tag: String,
    body: Py<PyAny>,
}

#[pymethods]
impl PyAisMessage {
    #[new]
    fn new(is_own_ship: bool, type_tag: String, body: Py<PyAny>) -> Self {
        Self {
            is_own_ship,
            type_tag,
            body,
        }
    }

    #[getter]
    fn is_own_ship(&self) -> bool {
        self.is_own_ship
    }

    #[getter]
    fn type_tag(&self) -> &str {
        &self.type_tag
    }

    #[getter]
    fn body(&self, py: Python<'_>) -> Py<PyAny> {
        // `Py<PyAny>` isn't `Clone`; `clone_ref` bumps the GIL-tracked
        // refcount so the returned handle points at the same Python
        // object (identity check `msg.body is body` holds).
        self.body.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "AisMessage(type_tag={:?}, is_own_ship={})",
            self.type_tag, self.is_own_ship,
        )
    }
}

impl PyAisMessage {
    /// Convert an owned Rust `AisMessage` into the Python wrapper.
    /// Called by `PyAisParser::next_message`.
    pub(crate) fn from_rust(py: Python<'_>, msg: marlin_ais::AisMessage) -> PyResult<Self> {
        let type_tag = Self::tag_for(&msg.body).to_string();
        let body = message_body_to_py(py, msg.body)?;
        Ok(Self {
            is_own_ship: msg.is_own_ship,
            type_tag,
            body,
        })
    }

    // `AisMessageBody` is `#[non_exhaustive]`, so the trailing wildcard
    // arm is required even though every current variant is covered.
    #[allow(clippy::wildcard_enum_match_arm)] // non_exhaustive upstream enum requires the catch-all
    fn tag_for(body: &AisMessageBody) -> &'static str {
        match body {
            AisMessageBody::Type1(_) => "type1",
            AisMessageBody::Type2(_) => "type2",
            AisMessageBody::Type3(_) => "type3",
            AisMessageBody::Type5(_) => "type5",
            AisMessageBody::Type9(_) => "type9",
            AisMessageBody::Type18(_) => "type18",
            AisMessageBody::Type19(_) => "type19",
            AisMessageBody::Type21(_) => "type21",
            AisMessageBody::Type24A(_) => "type24a",
            AisMessageBody::Type24B(_) => "type24b",
            AisMessageBody::Other { .. } => "other",
            _ => "unknown",
        }
    }
}

// ---------- BitReader (power-user primitive) ----------

/// Bit-level reader over an AIS payload. Binding class for
/// `marlin_ais::BitReader`.
///
/// The binding class owns its byte buffer and tracks the cursor itself;
/// every method constructs a fresh Rust `BitReader` and fast-forwards to
/// the current cursor position. That fast-forward is O(cursor) per call,
/// i.e. O(n²) total — accepted for AIS rates (Type 5 = ~20 field reads
/// × ~200-bit midpoint ≈ 4k bit-ops per decode, well below any
/// user-visible threshold). See the module doc for the rationale around
/// not storing a live `BitReader` (would require a self-referential
/// struct).
#[pyclass(name = "BitReader", module = "marlin.ais")]
pub struct PyBitReader {
    data: Vec<u8>,
    total_bits: usize,
    cursor: usize,
}

#[pymethods]
impl PyBitReader {
    #[new]
    fn new(data: &[u8], total_bits: usize) -> Self {
        Self {
            data: data.to_vec(),
            total_bits,
            cursor: 0,
        }
    }

    /// Read `n` unsigned bits. Past-end reads saturate to zero.
    fn u(&mut self, n: usize) -> u64 {
        let mut r = self.at_cursor();
        let v = r.u(n);
        self.cursor = self.cursor.saturating_add(n);
        v
    }

    /// Read `n` bits as signed two's complement.
    fn i(&mut self, n: usize) -> i64 {
        let mut r = self.at_cursor();
        let v = r.i(n);
        self.cursor = self.cursor.saturating_add(n);
        v
    }

    /// Read a single bit as `bool`.
    fn b(&mut self) -> bool {
        self.u(1) != 0
    }

    /// Read `chars` 6-bit AIS characters. Trailing `@` padding is
    /// **preserved verbatim** (matching `marlin_ais::BitReader::string`);
    /// callers that want clean vessel names or destinations should trim
    /// `@` and spaces themselves. The typed message decoders (e.g.
    /// `StaticAndVoyageA.vessel_name`) do this trimming upstream.
    fn string(&mut self, chars: usize) -> String {
        let mut r = self.at_cursor();
        let v = r.string(chars);
        // AIS 6-bit chars are 6 bits each; advance the cursor manually
        // because we constructed a fresh BitReader (its cursor doesn't
        // update ours).
        self.cursor = self.cursor.saturating_add(chars.saturating_mul(6));
        v
    }

    /// Bits remaining between the cursor and `total_bits`.
    fn remaining(&self) -> usize {
        self.total_bits.saturating_sub(self.cursor)
    }

    fn __repr__(&self) -> String {
        format!(
            "BitReader(total_bits={}, cursor={}, remaining={})",
            self.total_bits,
            self.cursor,
            self.remaining(),
        )
    }
}

impl PyBitReader {
    /// Construct a fresh Rust `BitReader` and fast-forward it to the
    /// current cursor. O(cursor) per call — see the struct docs for
    /// the rationale.
    fn at_cursor(&self) -> RustBitReader<'_> {
        let mut r = RustBitReader::new(&self.data, self.total_bits);
        // Consume `self.cursor` bits in 64-bit chunks, with a remainder.
        for _ in 0..(self.cursor / 64) {
            let _ = r.u(64);
        }
        let rem = self.cursor % 64;
        if rem > 0 {
            let _ = r.u(rem);
        }
        r
    }
}

// ---------- AisParser (with reassembly + three clock modes) ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClockMode {
    Auto,
    Manual,
}

impl ClockMode {
    fn parse(s: Option<&str>) -> PyResult<Self> {
        match s.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "clock must be 'auto' or 'manual', got {other:?}"
            ))),
        }
    }
}

fn build_reassembler(timeout_ms: Option<u64>, clock_mode: ClockMode) -> AisReassembler {
    let Some(t) = timeout_ms else {
        return AisReassembler::with_max_partials(DEFAULT_MAX_PARTIALS);
    };
    let mut reassembler = AisReassembler::with_timeout_ms(DEFAULT_MAX_PARTIALS, t);
    if clock_mode == ClockMode::Manual {
        // The manual clock starts at 0, as it always has, so a fragment
        // fed before the first `tick()` is stamped 0 and still times out.
        reassembler.tick(0);
    }
    reassembler
}

/// AIS parser with multi-fragment reassembly and selectable clock source.
///
/// Three clock modes:
/// - `timeout_ms=None`: no eviction; the reassembler's clock is never
///   advanced.
/// - `timeout_ms=t, clock="auto"` (default): reads `time.monotonic_ns`
///   and ticks the reassembler at the start of every `next_message()` call.
/// - `timeout_ms=t, clock="manual"`: the caller ticks the reassembler
///   through `tick(now_ms=...)`; the clock starts at 0. Never touches
///   Python's `time` module. Enables deterministic replay of historical
///   data.
#[pyclass(name = "AisParser", module = "marlin.ais")]
pub struct PyAisParser {
    inner: AisFragmentParser<Parser>,
    clock_mode: ClockMode,
    timeout_ms: Option<u64>,
}

#[pymethods]
impl PyAisParser {
    #[staticmethod]
    #[pyo3(signature = (timeout_ms = None, clock = None))]
    fn one_shot(timeout_ms: Option<u64>, clock: Option<&str>) -> PyResult<Self> {
        let clock_mode = ClockMode::parse(clock)?;
        let reassembler = build_reassembler(timeout_ms, clock_mode);
        let inner = AisFragmentParser::with_reassembler(Parser::one_shot(), reassembler);
        Ok(Self {
            inner,
            clock_mode,
            timeout_ms,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (timeout_ms = None, clock = None, max_size = DEFAULT_MAX_SIZE))]
    fn streaming(timeout_ms: Option<u64>, clock: Option<&str>, max_size: usize) -> PyResult<Self> {
        let clock_mode = ClockMode::parse(clock)?;
        let reassembler = build_reassembler(timeout_ms, clock_mode);
        let inner = AisFragmentParser::with_reassembler(
            Parser::streaming_with_capacity(max_size),
            reassembler,
        );
        Ok(Self {
            inner,
            clock_mode,
            timeout_ms,
        })
    }

    fn feed(&mut self, data: &[u8]) {
        self.inner.feed(data);
    }

    /// Manual-clock tick — advance the reassembler's clock to `now_ms`.
    ///
    /// Partials last touched more than `timeout_ms` ago are evicted here; each
    /// eviction surfaces as a `ReassemblyError` from a later
    /// `next_message()` call.
    ///
    /// Only valid when the parser was built with `clock="manual"`.
    /// Raises `ValueError` otherwise.
    fn tick(&mut self, now_ms: u64) -> PyResult<()> {
        if self.clock_mode != ClockMode::Manual {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "tick() is only valid when clock=\"manual\"",
            ));
        }
        self.tick_inner(now_ms);
        Ok(())
    }

    /// Return the next decoded AIS message, `None` if no complete message
    /// is yet buffered, or raise `AisError` / `ReassemblyError` /
    /// `EnvelopeError` on decode or reassembly failure.
    fn next_message(&mut self, py: Python<'_>) -> PyResult<Option<PyAisMessage>> {
        if let Some(now) = self.auto_time_ms(py)? {
            self.tick_inner(now);
        }
        let result = self.inner.next_message();
        match result {
            None => Ok(None),
            Some(Ok(msg)) => Ok(Some(PyAisMessage::from_rust(py, msg)?)),
            Some(Err(e)) => Err(ais_err(py, e)),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyAisIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(
            py,
            PyAisIterator {
                parser,
                strict: false,
            },
        )
    }

    #[pyo3(signature = (strict = false))]
    fn iter(slf: PyRef<'_, Self>, strict: bool) -> PyResult<Py<PyAisIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyAisIterator { parser, strict })
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

impl PyAisParser {
    fn tick_inner(&mut self, now_ms: u64) {
        self.inner.tick(now_ms);
    }

    /// The monotonic clock reading an auto-clock `next_message()` ticks
    /// with; `None` when there is nothing to tick for (no timeout, or
    /// the caller drives the clock through `tick()`).
    fn auto_time_ms(&self, py: Python<'_>) -> PyResult<Option<u64>> {
        if self.timeout_ms.is_none() || self.clock_mode == ClockMode::Manual {
            return Ok(None);
        }
        let time_mod = py.import("time")?;
        let ns: u64 = time_mod.getattr("monotonic_ns")?.call0()?.extract()?;
        Ok(Some(ns / 1_000_000))
    }
}

#[pyclass(module = "marlin.ais")]
pub struct PyAisIterator {
    parser: Py<PyAisParser>,
    strict: bool,
}

#[pymethods]
impl PyAisIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<PyAisMessage> {
        loop {
            let result = {
                let mut borrow = self.parser.borrow_mut(py);
                borrow.next_message(py)
            };
            match result {
                Ok(Some(m)) => return Ok(m),
                Ok(None) => return Err(pyo3::exceptions::PyStopIteration::new_err(())),
                Err(e) => {
                    if self.strict {
                        return Err(e);
                    }
                    // lenient — swallow and loop.
                }
            }
        }
    }
}

// ---------- Registration ----------

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "ais")?;
    // Enums and value types:
    m.add_class::<PyNavStatus>()?;
    m.add_class::<PyManeuverIndicator>()?;
    m.add_class::<PyTurnDirection>()?;
    m.add_class::<PyPositioningStatus>()?;
    m.add_class::<PyAltitudeSensor>()?;
    m.add_class::<PyAtonType>()?;
    m.add_class::<PyEpfdType>()?;
    m.add_class::<PyAisVersion>()?;
    m.add_class::<PyDimensions>()?;
    m.add_class::<PyEta>()?;
    // Sum types in field position, their variant classes named after
    // their variants:
    m.add_class::<PyRateOfTurn>()?;
    name_variants(&py.get_type::<PyRateOfTurn>(), &PyRateOfTurn::VARIANTS)?;
    m.add_class::<PyTimestamp>()?;
    name_variants(&py.get_type::<PyTimestamp>(), &PyTimestamp::VARIANTS)?;
    m.add_class::<PyType24BExtent>()?;
    name_variants(
        &py.get_type::<PyType24BExtent>(),
        &PyType24BExtent::VARIANTS,
    )?;
    // Message variants:
    m.add_class::<PyPositionReportA>()?;
    m.add_class::<PyStaticAndVoyageA>()?;
    m.add_class::<PySarAircraftPositionReport>()?;
    m.add_class::<PyPositionReportB>()?;
    m.add_class::<PyExtendedPositionReportB>()?;
    m.add_class::<PyAidToNavigationReport>()?;
    m.add_class::<PyStaticDataB24A>()?;
    m.add_class::<PyStaticDataB24B>()?;
    m.add_class::<PyOther>()?;
    // Outer message wrapper:
    m.add_class::<PyAisMessage>()?;
    // Power-user primitive:
    m.add_class::<PyBitReader>()?;
    // Parser + iterator:
    m.add_class::<PyAisParser>()?;
    m.add_class::<PyAisIterator>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
