use pyo3::{conversion::IntoPy, prelude::*};

use crate::error::Error;

impl IntoPy<PyObject> for Error {
    fn into_py(self, py: Python<'_>) -> PyObject {
        self.to_string().into_py(py)
    }
}
