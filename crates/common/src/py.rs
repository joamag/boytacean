//! Python interoperability for common Boytacean types.
//!
//! Implements PyO3's [`IntoPyObject`] trait for domain types,
//! enabling seamless conversion when crossing the Rust-Python boundary.

use pyo3::prelude::*;

use crate::error::Error;

/// Converts a Boytacean [`Error`] into a Python object.
///
/// The error is converted to its string representation via [`Display`]
/// and then wrapped as a Python string object (`PyAny`).
impl<'py> IntoPyObject<'py> for Error {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.to_string().into_pyobject(py).map(|s| s.into_any())?)
    }
}
