use pyo3::{prelude::*, types::PyBytes};

use crate::{
    gb::GameBoy as GameBoyBase,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
};

#[pyclass]
struct GameBoy {
    system: GameBoyBase,
}

#[pymethods]
impl GameBoy {
    #[new]
    fn new() -> Self {
        Self {
            system: GameBoyBase::new(None),
        }
    }

    pub fn reset(&mut self) {
        self.system.reset();
    }

    pub fn load(&mut self) {
        self.system.load(true);
    }

    pub fn load_rom(&mut self, path: &str) {
        self.system.load_rom_file(path, None);
    }

    pub fn clock(&mut self) -> u16 {
        self.system.clock()
    }

    pub fn clock_m(&mut self, count: usize) -> u16 {
        self.system.clock_m(count)
    }

    pub fn clocks(&mut self, count: usize) -> u64 {
        self.system.clocks(count)
    }

    pub fn next_frame(&mut self) -> u32 {
        self.system.next_frame()
    }

    pub fn frame_buffer(&mut self, py: Python) -> PyObject {
        let pybytes = PyBytes::new(py, self.system.frame_buffer());
        pybytes.into()
    }

    pub fn apu_enabled(&self) -> bool {
        self.system.apu_enabled()
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        self.system.set_apu_enabled(value);
    }
}

#[pymodule]
fn boytacean(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<GameBoy>()?;
    module.add("DISPLAY_WIDTH", DISPLAY_WIDTH)?;
    module.add("DISPLAY_HEIGHT", DISPLAY_HEIGHT)?;
    Ok(())
}
