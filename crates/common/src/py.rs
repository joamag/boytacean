use pyo3::prelude::*;

use crate::error::Error;

impl<'py> IntoPyObject<'py> for Error {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.to_string().into_pyobject(py).map(|s| s.into_any())?)
    }
}
