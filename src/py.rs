use pyo3::prelude::*;

#[pyclass]
struct Boytacean {
    #[pyo3(get, set)]
    value: i32,
}

#[pymethods]
impl Boytacean {
    #[new]
    fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn add(&mut self, other: i32) {
        self.value += other;
    }
}

#[pymodule]
fn boytacean(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<Boytacean>()?;
    Ok(())
}
