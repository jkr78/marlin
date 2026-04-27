//! Python wrappers for `marlin-ais` shared types.
//!
//! Task 10 covers the data primitives (enums + Dimensions/Eta value types)
//! that Task 11's typed message variants depend on. Same patterns as the
//! NMEA layer (`SCREAMING_SNAKE_CASE` enum variants via
//! `#[pyo3(name = "...")]`, `Reserved`/`Other` payload variants collapse to
//! the fieldless default via a `_ => ...` wildcard arm in the `From<RustX>`
//! impl).

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use marlin_ais::{
    AisFragmentParser, AisMessageBody, AisReassembler, AisVersion as RustAisVersion,
    BitReader as RustBitReader, Dimensions as RustDimensions, EpfdType as RustEpfdType,
    Eta as RustEta, ExtendedPositionReportB as RustExtendedPositionReportB,
    ManeuverIndicator as RustManeuverIndicator, NavStatus as RustNavStatus,
    PositionReportA as RustPositionReportA, PositionReportB as RustPositionReportB,
    StaticAndVoyageA as RustStaticAndVoyageA, StaticDataB24A as RustStaticDataB24A,
    StaticDataB24B as RustStaticDataB24B, DEFAULT_MAX_PARTIALS,
};
use marlin_nmea_envelope::{OneShot, Streaming};

use crate::envelope::DEFAULT_MAX_SIZE;
use crate::errors::ais_err;

// ---------- NavStatus ----------

/// Navigation status (mirrors `NavStatus`). Wire values 0..8, 14, 15;
/// `Reserved(u8)` for 9..=13 collapses to `NOT_DEFINED`.
#[pyclass(name = "NavStatus", eq, eq_int, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    #[pyo3(name = "NOT_DEFINED")]
    NotDefined = 15,
}

impl From<RustNavStatus> for PyNavStatus {
    // `Reserved(u8)` carries a raw byte this fieldless enum cannot represent;
    // collapse to NotDefined via the wildcard. `match_same_arms` fires on the
    // NotDefined => NotDefined + _ => NotDefined pair — intentional.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustNavStatus) -> Self {
        match v {
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
            RustNavStatus::NotDefined => Self::NotDefined,
            _ => Self::NotDefined,
        }
    }
}

// ---------- ManeuverIndicator ----------

/// Special maneuver indicator (mirrors `ManeuverIndicator`).
///
/// All four upstream variants are fieldless and map 1:1. The Rust
/// source is `#[non_exhaustive]` across crate boundaries, which forces
/// the `From` impl to carry a wildcard arm — that arm collapses any
/// future upstream variant to `NotAvailable` (same "unknown → safe
/// default" convention as `NavStatus::NotDefined`, `EpfdType::Undefined`,
/// `AisVersion::Future`). The compile-failure-as-audit-signal strategy
/// used elsewhere in the repo doesn't apply here because the wildcard
/// is mandatory, not optional.
#[pyclass(name = "ManeuverIndicator", eq, eq_int, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyManeuverIndicator {
    #[pyo3(name = "NOT_AVAILABLE")]
    NotAvailable = 0,
    #[pyo3(name = "NO_SPECIAL")]
    NoSpecial = 1,
    #[pyo3(name = "SPECIAL")]
    Special = 2,
    #[pyo3(name = "RESERVED")]
    Reserved = 3,
}

impl From<RustManeuverIndicator> for PyManeuverIndicator {
    // `#[non_exhaustive]` forces the wildcard; defensive collapse onto
    // NotAvailable keeps the binding compiling if upstream grows a new
    // variant. Silences `match_same_arms` on the
    // `NotAvailable => NotAvailable` + `_ => NotAvailable` pair.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustManeuverIndicator) -> Self {
        match v {
            RustManeuverIndicator::NotAvailable => Self::NotAvailable,
            RustManeuverIndicator::NoSpecial => Self::NoSpecial,
            RustManeuverIndicator::Special => Self::Special,
            RustManeuverIndicator::Reserved => Self::Reserved,
            _ => Self::NotAvailable,
        }
    }
}

// ---------- EpfdType ----------

/// Electronic Position-Fixing Device type (mirrors `EpfdType`). Wire
/// values 0..8 and 15; `Reserved(u8)` for 9..=14 collapses to
/// `UNDEFINED`.
#[pyclass(name = "EpfdType", eq, eq_int, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyEpfdType {
    #[pyo3(name = "UNDEFINED")]
    Undefined = 0,
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

impl From<RustEpfdType> for PyEpfdType {
    // `Reserved(u8)` collapses to `Undefined` via the wildcard; silence
    // `match_same_arms` for the `Undefined => Undefined` + `_ => Undefined`
    // pair — same rationale as PyNavStatus.
    #[allow(clippy::match_same_arms)]
    fn from(v: RustEpfdType) -> Self {
        match v {
            RustEpfdType::Undefined => Self::Undefined,
            RustEpfdType::Gps => Self::Gps,
            RustEpfdType::Glonass => Self::Glonass,
            RustEpfdType::CombinedGpsGlonass => Self::CombinedGpsGlonass,
            RustEpfdType::LoranC => Self::LoranC,
            RustEpfdType::Chayka => Self::Chayka,
            RustEpfdType::IntegratedNavigation => Self::IntegratedNavigation,
            RustEpfdType::Surveyed => Self::Surveyed,
            RustEpfdType::Galileo => Self::Galileo,
            RustEpfdType::InternalGnss => Self::InternalGnss,
            _ => Self::Undefined,
        }
    }
}

// ---------- AisVersion ----------

/// AIS protocol version indicator (mirrors `AisVersion`). Rust type is
/// `#[non_exhaustive]`; defensive wildcard collapses future variants
/// onto `FUTURE`.
#[pyclass(name = "AisVersion", eq, eq_int, module = "marlin.ais")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    // Defensive wildcard — upstream is `#[non_exhaustive]`; any future
    // variant collapses to Future. Silences `match_same_arms` for the
    // `Future => Future` + `_ => Future` pair.
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

/// Frozen vessel dimensions value type (mirrors `Dimensions`). All four
/// fields are `Option<u{8,16}>` with `None` signalling "not available"
/// (the wire sentinel `0`). `_m` suffix preserves the unit, matching
/// the Rust struct.
// All four fields are `to_*` distances — the shared prefix is the wire
// shape (distance from reference point to bow/stern/port/starboard) and
// must not be stripped.
#[allow(clippy::struct_field_names)]
#[pyclass(name = "Dimensions", frozen, eq, hash, module = "marlin.ais")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyDimensions {
    #[pyo3(get)]
    to_bow_m: Option<u16>,
    #[pyo3(get)]
    to_stern_m: Option<u16>,
    #[pyo3(get)]
    to_port_m: Option<u8>,
    #[pyo3(get)]
    to_starboard_m: Option<u8>,
}

#[pymethods]
impl PyDimensions {
    #[new]
    #[pyo3(signature = (to_bow_m = None, to_stern_m = None, to_port_m = None, to_starboard_m = None))]
    fn new(
        to_bow_m: Option<u16>,
        to_stern_m: Option<u16>,
        to_port_m: Option<u8>,
        to_starboard_m: Option<u8>,
    ) -> Self {
        Self {
            to_bow_m,
            to_stern_m,
            to_port_m,
            to_starboard_m,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Dimensions(to_bow_m={:?}, to_stern_m={:?}, to_port_m={:?}, to_starboard_m={:?})",
            self.to_bow_m, self.to_stern_m, self.to_port_m, self.to_starboard_m,
        )
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

/// Frozen ETA value type (mirrors `Eta`). All four fields are
/// `Option<u8>` with `None` on the per-sub-field sentinel.
#[pyclass(name = "Eta", frozen, eq, hash, module = "marlin.ais")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyEta {
    #[pyo3(get)]
    month: Option<u8>,
    #[pyo3(get)]
    day: Option<u8>,
    #[pyo3(get)]
    hour: Option<u8>,
    #[pyo3(get)]
    minute: Option<u8>,
}

#[pymethods]
impl PyEta {
    #[new]
    #[pyo3(signature = (month = None, day = None, hour = None, minute = None))]
    fn new(month: Option<u8>, day: Option<u8>, hour: Option<u8>, minute: Option<u8>) -> Self {
        Self {
            month,
            day,
            hour,
            minute,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Eta(month={:?}, day={:?}, hour={:?}, minute={:?})",
            self.month, self.day, self.hour, self.minute,
        )
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

// ---------- PositionReportA (Types 1/2/3) ----------

/// Class A position report payload. Used by Types 1, 2, and 3; the
/// AIS message-type distinction (1 vs 2 vs 3) is preserved at the
/// `AisMessage` wrapper level, not here.
#[pyclass(name = "PositionReportA", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyPositionReportA {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    navigation_status: PyNavStatus,
    #[pyo3(get)]
    rate_of_turn: Option<f32>,
    #[pyo3(get)]
    speed_over_ground: Option<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    #[pyo3(get)]
    longitude_deg: Option<f64>,
    #[pyo3(get)]
    latitude_deg: Option<f64>,
    #[pyo3(get)]
    course_over_ground: Option<f32>,
    #[pyo3(get)]
    true_heading: Option<u16>,
    #[pyo3(get)]
    timestamp: u8,
    #[pyo3(get)]
    special_maneuver: PyManeuverIndicator,
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
        navigation_status = PyNavStatus::NotDefined,
        rate_of_turn = None,
        speed_over_ground = None,
        position_accuracy = false,
        longitude_deg = None,
        latitude_deg = None,
        course_over_ground = None,
        true_heading = None,
        timestamp = 60,
        special_maneuver = PyManeuverIndicator::NotAvailable,
        raim = false,
        radio_status = 0,
    ))]
    fn new(
        mmsi: u32,
        navigation_status: PyNavStatus,
        rate_of_turn: Option<f32>,
        speed_over_ground: Option<f32>,
        position_accuracy: bool,
        longitude_deg: Option<f64>,
        latitude_deg: Option<f64>,
        course_over_ground: Option<f32>,
        true_heading: Option<u16>,
        timestamp: u8,
        special_maneuver: PyManeuverIndicator,
        raim: bool,
        radio_status: u32,
    ) -> Self {
        Self {
            mmsi,
            navigation_status,
            rate_of_turn,
            speed_over_ground,
            position_accuracy,
            longitude_deg,
            latitude_deg,
            course_over_ground,
            true_heading,
            timestamp,
            special_maneuver,
            raim,
            radio_status,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionReportA(mmsi={}, lat={:?}, lon={:?}, sog={:?})",
            self.mmsi, self.latitude_deg, self.longitude_deg, self.speed_over_ground,
        )
    }
}

impl From<RustPositionReportA> for PyPositionReportA {
    fn from(d: RustPositionReportA) -> Self {
        Self {
            mmsi: d.mmsi,
            navigation_status: d.navigation_status.into(),
            rate_of_turn: d.rate_of_turn,
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            true_heading: d.true_heading,
            timestamp: d.timestamp,
            special_maneuver: d.special_maneuver.into(),
            raim: d.raim,
            radio_status: d.radio_status,
        }
    }
}

// ---------- StaticAndVoyageA (Type 5) ----------

/// Class A static and voyage data payload (Type 5).
#[pyclass(name = "StaticAndVoyageA", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticAndVoyageA {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    ais_version: PyAisVersion,
    #[pyo3(get)]
    imo_number: Option<u32>,
    #[pyo3(get)]
    call_sign: Option<String>,
    #[pyo3(get)]
    vessel_name: Option<String>,
    #[pyo3(get)]
    ship_type: u8,
    #[pyo3(get)]
    dimensions: PyDimensions,
    #[pyo3(get)]
    epfd: PyEpfdType,
    #[pyo3(get)]
    eta: PyEta,
    #[pyo3(get)]
    draught_m: Option<f32>,
    #[pyo3(get)]
    destination: Option<String>,
    #[pyo3(get)]
    dte: bool,
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
        ship_type = 0,
        dimensions = None,
        epfd = PyEpfdType::Undefined,
        eta = None,
        draught_m = None,
        destination = None,
        dte = false,
    ))]
    fn new(
        mmsi: u32,
        ais_version: PyAisVersion,
        imo_number: Option<u32>,
        call_sign: Option<String>,
        vessel_name: Option<String>,
        ship_type: u8,
        dimensions: Option<PyDimensions>,
        epfd: PyEpfdType,
        eta: Option<PyEta>,
        draught_m: Option<f32>,
        destination: Option<String>,
        dte: bool,
    ) -> Self {
        let dimensions = dimensions.unwrap_or_else(|| {
            PyDimensions::from(RustDimensions {
                to_bow_m: None,
                to_stern_m: None,
                to_port_m: None,
                to_starboard_m: None,
            })
        });
        let eta = eta.unwrap_or_else(|| PyEta::from(RustEta::default()));
        Self {
            mmsi,
            ais_version,
            imo_number,
            call_sign,
            vessel_name,
            ship_type,
            dimensions,
            epfd,
            eta,
            draught_m,
            destination,
            dte,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "StaticAndVoyageA(mmsi={}, vessel_name={:?}, destination={:?})",
            self.mmsi, self.vessel_name, self.destination,
        )
    }
}

impl From<RustStaticAndVoyageA> for PyStaticAndVoyageA {
    fn from(d: RustStaticAndVoyageA) -> Self {
        Self {
            mmsi: d.mmsi,
            ais_version: d.ais_version.into(),
            imo_number: d.imo_number,
            call_sign: d.call_sign,
            vessel_name: d.vessel_name,
            ship_type: d.ship_type,
            dimensions: d.dimensions.into(),
            epfd: d.epfd.into(),
            eta: d.eta.into(),
            draught_m: d.draught_m,
            destination: d.destination,
            dte: d.dte,
        }
    }
}

// ---------- PositionReportB (Type 18) ----------

/// Class B CS position report payload (Type 18).
// The five `class_b_*_flag` bits are ITU-R M.1371 wire-format flags;
// bundling them is the wire reality, so silence `struct_excessive_bools`.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "PositionReportB", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyPositionReportB {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    speed_over_ground: Option<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    #[pyo3(get)]
    longitude_deg: Option<f64>,
    #[pyo3(get)]
    latitude_deg: Option<f64>,
    #[pyo3(get)]
    course_over_ground: Option<f32>,
    #[pyo3(get)]
    true_heading: Option<u16>,
    #[pyo3(get)]
    timestamp: u8,
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
        timestamp = 60,
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
        speed_over_ground: Option<f32>,
        position_accuracy: bool,
        longitude_deg: Option<f64>,
        latitude_deg: Option<f64>,
        course_over_ground: Option<f32>,
        true_heading: Option<u16>,
        timestamp: u8,
        class_b_cs_flag: bool,
        class_b_display_flag: bool,
        class_b_dsc_flag: bool,
        class_b_band_flag: bool,
        class_b_message22_flag: bool,
        assigned_flag: bool,
        raim: bool,
        radio_status: u32,
    ) -> Self {
        Self {
            mmsi,
            speed_over_ground,
            position_accuracy,
            longitude_deg,
            latitude_deg,
            course_over_ground,
            true_heading,
            timestamp,
            class_b_cs_flag,
            class_b_display_flag,
            class_b_dsc_flag,
            class_b_band_flag,
            class_b_message22_flag,
            assigned_flag,
            raim,
            radio_status,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionReportB(mmsi={}, lat={:?}, lon={:?}, sog={:?})",
            self.mmsi, self.latitude_deg, self.longitude_deg, self.speed_over_ground,
        )
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
            timestamp: d.timestamp,
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
// 4 bools (`position_accuracy`, `raim`, `dte`, `assigned_flag`) are
// ITU-R M.1371 wire-format flags — the wire reality.
#[allow(clippy::struct_excessive_bools)]
#[pyclass(name = "ExtendedPositionReportB", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyExtendedPositionReportB {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    speed_over_ground: Option<f32>,
    #[pyo3(get)]
    position_accuracy: bool,
    #[pyo3(get)]
    longitude_deg: Option<f64>,
    #[pyo3(get)]
    latitude_deg: Option<f64>,
    #[pyo3(get)]
    course_over_ground: Option<f32>,
    #[pyo3(get)]
    true_heading: Option<u16>,
    #[pyo3(get)]
    timestamp: u8,
    #[pyo3(get)]
    vessel_name: Option<String>,
    #[pyo3(get)]
    ship_type: u8,
    #[pyo3(get)]
    dimensions: PyDimensions,
    #[pyo3(get)]
    epfd: PyEpfdType,
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
        timestamp = 60,
        vessel_name = None,
        ship_type = 0,
        dimensions = None,
        epfd = PyEpfdType::Undefined,
        raim = false,
        dte = false,
        assigned_flag = false,
    ))]
    fn new(
        mmsi: u32,
        speed_over_ground: Option<f32>,
        position_accuracy: bool,
        longitude_deg: Option<f64>,
        latitude_deg: Option<f64>,
        course_over_ground: Option<f32>,
        true_heading: Option<u16>,
        timestamp: u8,
        vessel_name: Option<String>,
        ship_type: u8,
        dimensions: Option<PyDimensions>,
        epfd: PyEpfdType,
        raim: bool,
        dte: bool,
        assigned_flag: bool,
    ) -> Self {
        let dimensions = dimensions.unwrap_or_else(|| {
            PyDimensions::from(RustDimensions {
                to_bow_m: None,
                to_stern_m: None,
                to_port_m: None,
                to_starboard_m: None,
            })
        });
        Self {
            mmsi,
            speed_over_ground,
            position_accuracy,
            longitude_deg,
            latitude_deg,
            course_over_ground,
            true_heading,
            timestamp,
            vessel_name,
            ship_type,
            dimensions,
            epfd,
            raim,
            dte,
            assigned_flag,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtendedPositionReportB(mmsi={}, vessel_name={:?}, lat={:?}, lon={:?})",
            self.mmsi, self.vessel_name, self.latitude_deg, self.longitude_deg,
        )
    }
}

impl From<RustExtendedPositionReportB> for PyExtendedPositionReportB {
    fn from(d: RustExtendedPositionReportB) -> Self {
        Self {
            mmsi: d.mmsi,
            speed_over_ground: d.speed_over_ground,
            position_accuracy: d.position_accuracy,
            longitude_deg: d.longitude_deg,
            latitude_deg: d.latitude_deg,
            course_over_ground: d.course_over_ground,
            true_heading: d.true_heading,
            timestamp: d.timestamp,
            vessel_name: d.vessel_name,
            ship_type: d.ship_type,
            dimensions: d.dimensions.into(),
            epfd: d.epfd.into(),
            raim: d.raim,
            dte: d.dte,
            assigned_flag: d.assigned_flag,
        }
    }
}

// ---------- StaticDataB24A (Type 24 Part A) ----------

/// Class B static data Part A payload (Type 24A).
#[pyclass(name = "StaticDataB24A", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticDataB24A {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    vessel_name: Option<String>,
}

#[pymethods]
impl PyStaticDataB24A {
    #[new]
    #[pyo3(signature = (mmsi = 0, vessel_name = None))]
    fn new(mmsi: u32, vessel_name: Option<String>) -> Self {
        Self { mmsi, vessel_name }
    }

    fn __repr__(&self) -> String {
        format!(
            "StaticDataB24A(mmsi={}, vessel_name={:?})",
            self.mmsi, self.vessel_name,
        )
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

/// Class B static data Part B payload (Type 24B).
#[pyclass(name = "StaticDataB24B", frozen, module = "marlin.ais")]
#[derive(Clone, Debug)]
pub struct PyStaticDataB24B {
    #[pyo3(get)]
    mmsi: u32,
    #[pyo3(get)]
    ship_type: u8,
    #[pyo3(get)]
    vendor_id: Option<String>,
    #[pyo3(get)]
    call_sign: Option<String>,
    #[pyo3(get)]
    dimensions: PyDimensions,
}

#[pymethods]
impl PyStaticDataB24B {
    #[new]
    #[pyo3(signature = (
        mmsi = 0,
        ship_type = 0,
        vendor_id = None,
        call_sign = None,
        dimensions = None,
    ))]
    fn new(
        mmsi: u32,
        ship_type: u8,
        vendor_id: Option<String>,
        call_sign: Option<String>,
        dimensions: Option<PyDimensions>,
    ) -> Self {
        let dimensions = dimensions.unwrap_or_else(|| {
            PyDimensions::from(RustDimensions {
                to_bow_m: None,
                to_stern_m: None,
                to_port_m: None,
                to_starboard_m: None,
            })
        });
        Self {
            mmsi,
            ship_type,
            vendor_id,
            call_sign,
            dimensions,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "StaticDataB24B(mmsi={}, call_sign={:?}, vendor_id={:?})",
            self.mmsi, self.call_sign, self.vendor_id,
        )
    }
}

impl From<RustStaticDataB24B> for PyStaticDataB24B {
    fn from(d: RustStaticDataB24B) -> Self {
        Self {
            mmsi: d.mmsi,
            ship_type: d.ship_type,
            vendor_id: d.vendor_id,
            call_sign: d.call_sign,
            dimensions: d.dimensions.into(),
        }
    }
}

// ---------- Other (catch-all for un-decoded msg_type) ----------

/// Catch-all variant for AIS message types this crate does not yet
/// decode. Preserves the raw bit buffer and total bit count so
/// callers can plug in their own decoder.
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
/// distinction is preserved at the `AisMessage` wrapper level (Task
/// 12), not here. Unknown future variants of the `#[non_exhaustive]`
/// upstream enum surface as `PyValueError` so the binding can be
/// updated deliberately rather than silently misrouting payloads.
pub(crate) fn message_body_to_py(py: Python<'_>, body: AisMessageBody) -> PyResult<Py<PyAny>> {
    Ok(match body {
        AisMessageBody::Type1(d)
        | AisMessageBody::Type2(d)
        | AisMessageBody::Type3(d) => Py::new(py, PyPositionReportA::from(d))?.into_any(),
        AisMessageBody::Type5(d) => Py::new(py, PyStaticAndVoyageA::from(d))?.into_any(),
        AisMessageBody::Type18(d) => Py::new(py, PyPositionReportB::from(d))?.into_any(),
        AisMessageBody::Type19(d) => Py::new(py, PyExtendedPositionReportB::from(d))?.into_any(),
        AisMessageBody::Type24A(d) => Py::new(py, PyStaticDataB24A::from(d))?.into_any(),
        AisMessageBody::Type24B(d) => Py::new(py, PyStaticDataB24B::from(d))?.into_any(),
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
/// Construct in Python for testing or via `from_rust` from the parser
/// (Task 14).
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
            AisMessageBody::Type18(_) => "type18",
            AisMessageBody::Type19(_) => "type19",
            AisMessageBody::Type24A(_) => "type24a",
            AisMessageBody::Type24B(_) => "type24b",
            AisMessageBody::Other { .. } => "other",
            _ => "unknown",
        }
    }
}

// ---------- BitReader (power-user primitive) ----------

/// Bit-level reader over an AIS payload. Mirrors `marlin_ais::BitReader`.
///
/// The Python wrapper owns its byte buffer and tracks the cursor itself;
/// every method constructs a fresh Rust `BitReader` and fast-forwards to
/// the current cursor position. That fast-forward is O(cursor) per call,
/// i.e. O(n²) total — documented as accepted for v0.1 AIS rates in the
/// PRD (Type 5 = ~20 field reads × ~200-bit midpoint ≈ 4k bit-ops per
/// decode, well below any user-visible threshold). See the module doc
/// for the rationale around not storing a live `BitReader` (would
/// require a self-referential struct).
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

fn build_reassembler(timeout_ms: Option<u64>) -> AisReassembler {
    match timeout_ms {
        Some(t) => AisReassembler::with_timeout_ms(DEFAULT_MAX_PARTIALS, t),
        None => AisReassembler::with_max_partials(DEFAULT_MAX_PARTIALS),
    }
}

/// Internal mode-enum so the HRTB generic on `AisFragmentParser<P>` doesn't
/// infect the Python class surface.
///
/// Do **not** "simplify" this into an `AisFragmentParser<P>` field with a
/// generic bound. The HRTB nested-call trap (see the repo CLAUDE.md)
/// means `&mut self` methods bounded by
/// `for<'a> SentenceSource<Item<'a> = RawSentence<'a>>` can't delegate to
/// other such methods — rustc emits "P does not live long enough". The
/// mode-enum dodges the trap by matching on a concrete parser in each
/// arm. Same structural rationale as `Nmea0183Inner` in `nmea.rs`.
#[derive(Debug)]
enum AisInner {
    OneShot(AisFragmentParser<OneShot>),
    Streaming(AisFragmentParser<Streaming>),
}

/// AIS parser with multi-fragment reassembly and selectable clock source.
///
/// Three clock modes:
/// - `timeout_ms=None`: no eviction; clock is never read.
/// - `timeout_ms=Some(t), clock="auto"` (default): reads `time.monotonic_ns`
///   every `next_message()` call.
/// - `timeout_ms=Some(t), clock="manual"`: uses `manual_now_ms` (set via
///   `tick(now_ms=...)`). Never touches Python's `time` module. Enables
///   deterministic replay of historical data.
#[pyclass(name = "AisParser", module = "marlin.ais")]
pub struct PyAisParser {
    inner: AisInner,
    clock_mode: ClockMode,
    timeout_ms: Option<u64>,
    manual_now_ms: u64,
}

#[pymethods]
impl PyAisParser {
    #[staticmethod]
    #[pyo3(signature = (timeout_ms = None, clock = None))]
    fn one_shot(timeout_ms: Option<u64>, clock: Option<&str>) -> PyResult<Self> {
        let clock_mode = ClockMode::parse(clock)?;
        let reassembler = build_reassembler(timeout_ms);
        let frag = AisFragmentParser::with_reassembler(OneShot::new(), reassembler);
        Ok(Self {
            inner: AisInner::OneShot(frag),
            clock_mode,
            timeout_ms,
            manual_now_ms: 0,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (timeout_ms = None, clock = None, max_size = DEFAULT_MAX_SIZE))]
    fn streaming(
        timeout_ms: Option<u64>,
        clock: Option<&str>,
        max_size: usize,
    ) -> PyResult<Self> {
        let clock_mode = ClockMode::parse(clock)?;
        let reassembler = build_reassembler(timeout_ms);
        let frag = AisFragmentParser::with_reassembler(
            Streaming::with_capacity(max_size),
            reassembler,
        );
        Ok(Self {
            inner: AisInner::Streaming(frag),
            clock_mode,
            timeout_ms,
            manual_now_ms: 0,
        })
    }

    fn feed(&mut self, data: &[u8]) {
        match &mut self.inner {
            AisInner::OneShot(p) => p.feed(data),
            AisInner::Streaming(p) => p.feed(data),
        }
    }

    /// Manual-clock tick — stash `now_ms` for the next `next_message()`.
    ///
    /// Eviction of expired reassembly partials is **deferred** until the
    /// next `next_message()` call (which forwards `manual_now_ms` to
    /// `next_message_at`). `tick()` on its own does not touch the
    /// reassembler; it only updates the wrapper's stored clock value.
    ///
    /// Only valid when the parser was built with `clock="manual"`.
    /// Raises `ValueError` otherwise.
    fn tick(&mut self, now_ms: u64) -> PyResult<()> {
        if self.clock_mode != ClockMode::Manual {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "tick() is only valid when clock=\"manual\"",
            ));
        }
        self.manual_now_ms = now_ms;
        Ok(())
    }

    /// Return the next decoded AIS message, `None` if no complete message
    /// is yet buffered, or raise `AisError` / `ReassemblyError` /
    /// `EnvelopeError` on decode or reassembly failure.
    fn next_message(&mut self, py: Python<'_>) -> PyResult<Option<PyAisMessage>> {
        let now = self.current_time_ms(py)?;
        let result = match &mut self.inner {
            AisInner::OneShot(p) => p.next_message_at(now),
            AisInner::Streaming(p) => p.next_message_at(now),
        };
        match result {
            None => Ok(None),
            Some(Ok(msg)) => Ok(Some(PyAisMessage::from_rust(py, msg)?)),
            Some(Err(e)) => Err(ais_err(py, e)),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyAisIterator>> {
        let py = slf.py();
        let parser: Py<Self> = slf.into();
        Py::new(py, PyAisIterator { parser, strict: false })
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
    fn current_time_ms(&self, py: Python<'_>) -> PyResult<u64> {
        // When timeout is None, the reassembler ignores `now` (no eviction
        // logic runs); any sentinel value works and we skip the time read.
        if self.timeout_ms.is_none() {
            return Ok(0);
        }
        match self.clock_mode {
            ClockMode::Manual => Ok(self.manual_now_ms),
            ClockMode::Auto => {
                let time_mod = py.import("time")?;
                let ns: u64 = time_mod
                    .getattr("monotonic_ns")?
                    .call0()?
                    .extract()?;
                Ok(ns / 1_000_000)
            }
        }
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
    // Task 10 enums + value types:
    m.add_class::<PyNavStatus>()?;
    m.add_class::<PyManeuverIndicator>()?;
    m.add_class::<PyEpfdType>()?;
    m.add_class::<PyAisVersion>()?;
    m.add_class::<PyDimensions>()?;
    m.add_class::<PyEta>()?;
    // Task 11 message variants:
    m.add_class::<PyPositionReportA>()?;
    m.add_class::<PyStaticAndVoyageA>()?;
    m.add_class::<PyPositionReportB>()?;
    m.add_class::<PyExtendedPositionReportB>()?;
    m.add_class::<PyStaticDataB24A>()?;
    m.add_class::<PyStaticDataB24B>()?;
    m.add_class::<PyOther>()?;
    // Task 12 outer message wrapper:
    m.add_class::<PyAisMessage>()?;
    // Task 13 power-user primitive:
    m.add_class::<PyBitReader>()?;
    // Task 14 parser + iterator:
    m.add_class::<PyAisParser>()?;
    m.add_class::<PyAisIterator>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
