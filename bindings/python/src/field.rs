//! Python wrappers for `marlin-field`: the decoded-field state.
//!
//! One runtime class, `marlin.field.FieldState`, carries every payload
//! type as an untyped Python object. Its five nested variant classes are
//! the five field states of the Rust `FieldState<T>`; the `kind` strings
//! come from `marlin_field::Kind::name()`, so the two languages share one
//! table.
//!
//! The decoder bindings convert each `FieldState<T>` attribute with
//! [`to_py`] on the way out and [`from_py`] on the way in.

use pyo3::prelude::*;
use pyo3::types::{PyModule, PyTuple, PyType};
use pyo3::IntoPyObjectExt;

use marlin_field::{FieldState, Invalid, Kind, RawCode};

/// The decoded outcome of one wire field: the field state.
///
/// One of five variant classes: `FieldState.Value(value)`,
/// `FieldState.AtLeast(bound)`, `FieldState.NotAvailable()`,
/// `FieldState.SenderError(code)` or `FieldState.Invalid(code)`, where
/// `Invalid.code` is `None` for a field with no wire integer. Read a field
/// with `isinstance`, `match` on the variant class, or the shared
/// `kind`, `value` and `value_or_bound` properties. Every state is truthy,
/// `FieldState.NotAvailable()` included: test the state, never the object.
#[pyclass(name = "FieldState", frozen, module = "marlin.field")]
#[derive(Debug)]
pub enum PyFieldState {
    /// The wire gave the field a meaning and the decoder accepted it.
    #[pyo3(name = "Value")]
    Value {
        /// The decoded value.
        value: Py<PyAny>,
    },
    /// Over-range: the true value is at or beyond this bound, in
    /// engineering units.
    #[pyo3(name = "AtLeast")]
    AtLeast {
        /// The over-range bound, which is not a value.
        bound: Py<PyAny>,
    },
    /// The sender supplied no value. Not an error.
    #[pyo3(name = "NotAvailable")]
    NotAvailable {},
    /// The sender knows it has no usable value; carries the raw code.
    #[pyo3(name = "SenderError")]
    SenderError {
        /// The raw code.
        code: i64,
    },
    /// The decoder could not give the wire field a meaning: the raw code
    /// of an undefined number, or `None` when no wire integer could be
    /// read.
    #[pyo3(name = "Invalid")]
    Invalid {
        /// The raw code, or `None` for an unparsable field.
        code: Option<i64>,
    },
}

impl PyFieldState {
    /// The state with the value erased, for the shared `kind` table.
    fn rust_kind(&self) -> Kind {
        match self {
            Self::Value { .. } => Kind::Value,
            Self::AtLeast { .. } => Kind::AtLeast,
            Self::NotAvailable {} => Kind::NotAvailable,
            Self::SenderError { code } => Kind::SenderError(RawCode(*code)),
            Self::Invalid { code: Some(code) } => Kind::Invalid(Invalid::Undefined(RawCode(*code))),
            Self::Invalid { code: None } => Kind::Invalid(Invalid::Unparsable),
        }
    }

    /// The Python name of this state's variant class.
    fn variant_name(&self) -> &'static str {
        match self {
            Self::Value { .. } => "Value",
            Self::AtLeast { .. } => "AtLeast",
            Self::NotAvailable {} => "NotAvailable",
            Self::SenderError { .. } => "SenderError",
            Self::Invalid { .. } => "Invalid",
        }
    }

    /// The payload object, if the state carries one: the value or the
    /// bound.
    fn payload(&self) -> Option<&Py<PyAny>> {
        match self {
            Self::Value { value } => Some(value),
            Self::AtLeast { bound } => Some(bound),
            Self::NotAvailable {} | Self::SenderError { .. } | Self::Invalid { .. } => None,
        }
    }
}

#[pymethods]
impl PyFieldState {
    /// The snake-case state tag: `"value"`, `"at_least"`,
    /// `"not_available"`, `"sender_error"` or `"invalid"`.
    #[getter]
    fn kind(&self) -> &'static str {
        self.rust_kind().name()
    }

    /// The value, if the field holds one; `None` in every other state. An
    /// over-range bound is not a value.
    #[getter]
    fn value(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        match self {
            Self::Value { value } => Some(value.clone_ref(py)),
            _ => None,
        }
    }

    /// The value or the over-range bound, if the field holds either;
    /// `None` in every other state.
    #[getter]
    fn value_or_bound(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.payload().map(|obj| obj.clone_ref(py))
    }

    fn __eq__(&self, py: Python<'_>, other: &Self) -> PyResult<bool> {
        match (self, other) {
            (Self::Value { value: a }, Self::Value { value: b })
            | (Self::AtLeast { bound: a }, Self::AtLeast { bound: b }) => a.bind(py).eq(b.bind(py)),
            (Self::NotAvailable {}, Self::NotAvailable {}) => Ok(true),
            (Self::SenderError { code: a }, Self::SenderError { code: b }) => Ok(a == b),
            (Self::Invalid { code: a }, Self::Invalid { code: b }) => Ok(a == b),
            _ => Ok(false),
        }
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        let payload = match self {
            Self::Value { value } | Self::AtLeast { bound: value } => value.clone_ref(py),
            Self::NotAvailable {} => py.None(),
            Self::SenderError { code } => code.into_py_any(py)?,
            Self::Invalid { code } => code.into_py_any(py)?,
        };
        PyTuple::new(py, [self.rust_kind().name().into_py_any(py)?, payload])?.hash()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let payload = match self {
            Self::Value { value } | Self::AtLeast { bound: value } => {
                value.bind(py).repr()?.to_string()
            }
            Self::NotAvailable {} => String::new(),
            Self::SenderError { code } | Self::Invalid { code: Some(code) } => code.to_string(),
            Self::Invalid { code: None } => String::from("None"),
        };
        Ok(format!("FieldState.{}({payload})", self.variant_name()))
    }
}

/// Converts a decoded `FieldState<T>` into its Python class, for the
/// getters of the decoder bindings.
pub(crate) fn to_py<'py, T: IntoPyObject<'py>>(
    py: Python<'py>,
    state: FieldState<T>,
) -> PyResult<PyFieldState> {
    Ok(match state {
        FieldState::Value(value) => PyFieldState::Value {
            value: value.into_py_any(py)?,
        },
        FieldState::AtLeast(bound) => PyFieldState::AtLeast {
            bound: bound.into_py_any(py)?,
        },
        FieldState::NotAvailable => PyFieldState::NotAvailable {},
        FieldState::SenderError(RawCode(code)) => PyFieldState::SenderError { code },
        FieldState::Invalid(Invalid::Undefined(RawCode(code))) => {
            PyFieldState::Invalid { code: Some(code) }
        }
        FieldState::Invalid(Invalid::Unparsable) => PyFieldState::Invalid { code: None },
    })
}

/// Coerces a constructor or setter argument into a `FieldState<T>`: a
/// `FieldState` passes through, `None` becomes `NotAvailable`, and any
/// other object becomes `Value`. The payload is extracted to `T` and the
/// extraction error, if any, is raised as is: `TypeError` for an object
/// of the wrong type, `OverflowError` for an integer outside the field's
/// width.
pub(crate) fn from_py<'py, T: FromPyObjectOwned<'py>>(
    obj: &Bound<'py, PyAny>,
) -> PyResult<FieldState<T>> {
    if obj.is_none() {
        return Ok(FieldState::NotAvailable);
    }
    let Ok(state) = obj.cast::<PyFieldState>() else {
        return Ok(FieldState::Value(extract_payload(obj)?));
    };
    Ok(match state.get() {
        PyFieldState::Value { value } => FieldState::Value(extract_payload(value.bind(obj.py()))?),
        PyFieldState::AtLeast { bound } => {
            FieldState::AtLeast(extract_payload(bound.bind(obj.py()))?)
        }
        PyFieldState::NotAvailable {} => FieldState::NotAvailable,
        PyFieldState::SenderError { code } => FieldState::SenderError(RawCode(*code)),
        PyFieldState::Invalid { code: Some(code) } => {
            FieldState::Invalid(Invalid::Undefined(RawCode(*code)))
        }
        PyFieldState::Invalid { code: None } => FieldState::Invalid(Invalid::Unparsable),
    })
}

/// The `repr` of a decoded `FieldState<T>` as its Python class shows it,
/// for the message `__repr__`s.
pub(crate) fn repr_state<'py, T: IntoPyObject<'py>>(
    py: Python<'py>,
    state: FieldState<T>,
) -> PyResult<String> {
    to_py(py, state)?.__repr__(py)
}

/// A constructor argument for a field-state attribute: absent or `None`
/// is `NotAvailable`, a `FieldState` passes through, a bare value is
/// `Value`; the payload is extracted to `T` and the extraction error is
/// raised as is.
pub(crate) fn field_arg<'py, T: FromPyObjectOwned<'py>>(
    obj: Option<&Bound<'py, PyAny>>,
) -> PyResult<FieldState<T>> {
    obj.map_or(Ok(FieldState::NotAvailable), from_py)
}

/// A decoded state whose payload is a Rust enum, converted to the
/// binding enum. The Rust enums are `#[non_exhaustive]`, so a variant
/// these bindings do not know is an error, never a fabricated member.
pub(crate) fn enum_state<R, P: TryFrom<R, Error = PyErr>>(
    state: FieldState<R>,
) -> PyResult<FieldState<P>> {
    Ok(match state {
        FieldState::Value(v) => FieldState::Value(P::try_from(v)?),
        FieldState::AtLeast(v) => FieldState::AtLeast(P::try_from(v)?),
        FieldState::NotAvailable => FieldState::NotAvailable,
        FieldState::SenderError(code) => FieldState::SenderError(code),
        FieldState::Invalid(why) => FieldState::Invalid(why),
    })
}

/// The error for a Rust enum variant these bindings have no member for.
pub(crate) fn unsupported_variant(enum_name: &str, variant: &dyn core::fmt::Debug) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(format!(
        "marlin Python bindings encountered an unsupported {enum_name} variant {variant:?} — bindings need updating"
    ))
}

/// Name the variant classes of a sum-type binding class after their
/// variants.
///
/// The `pyo3` 0.27 macros name each variant class after the Rust enum
/// and variant (`PyFieldState_Value`) and ignore the variant's
/// `#[pyo3(name)]` for that purpose, so the classes are renamed here:
/// `type(x).__name__` is the bare variant name (`Value`), which the
/// dataclass converter keys on, and `__qualname__` is dotted
/// (`FieldState.Value`).
pub(crate) fn name_variants(base: &Bound<'_, PyType>, variants: &[&str]) -> PyResult<()> {
    let base_name = base.name()?;
    for name in variants {
        let cls = base.getattr(name)?;
        cls.setattr("__name__", name)?;
        cls.setattr("__qualname__", format!("{base_name}.{name}"))?;
    }
    Ok(())
}

/// Extracts a payload to the field's Rust type. The extraction error type
/// is only `Into<PyErr>`, not `From`, so `?` needs this adapter.
fn extract_payload<'py, T: FromPyObjectOwned<'py>>(obj: &Bound<'py, PyAny>) -> PyResult<T> {
    obj.extract::<T>().map_err(Into::into)
}

/// Register the `field` submodule on the given parent. Called from `lib.rs`.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "field")?;
    m.add_class::<PyFieldState>()?;
    let one_of_each = [
        PyFieldState::Value { value: py.None() },
        PyFieldState::AtLeast { bound: py.None() },
        PyFieldState::NotAvailable {},
        PyFieldState::SenderError { code: 0 },
        PyFieldState::Invalid { code: None },
    ];
    let names = one_of_each.each_ref().map(PyFieldState::variant_name);
    name_variants(&py.get_type::<PyFieldState>(), &names)?;
    parent.add_submodule(&m)?;
    Ok(())
}
