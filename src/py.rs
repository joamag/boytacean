use pyo3::{prelude::*, types::PyBytes};

use crate::{
    gb::{GameBoy as GameBoyBase, GameBoyMode},
    info::Info,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
};

#[pyclass]
struct GameBoy {
    system: GameBoyBase,
}

#[pymethods]
impl GameBoy {
    #[new]
    fn new(mode: u8) -> Self {
        Self {
            system: GameBoyBase::new(Some(GameBoyMode::from_u8(mode))),
        }
    }

    pub fn reset(&mut self) {
        self.system.reset();
    }

    pub fn load(&mut self) {
        self.system.load(true);
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.system.load_rom(data, None);
    }

    pub fn load_rom_file(&mut self, path: &str) {
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

    pub fn ppu_enabled(&self) -> bool {
        self.system.ppu_enabled()
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        self.system.set_ppu_enabled(value);
    }

    pub fn apu_enabled(&self) -> bool {
        self.system.apu_enabled()
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        self.system.set_apu_enabled(value);
    }

    pub fn dma_enabled(&self) -> bool {
        self.system.dma_enabled()
    }

    pub fn set_dma_enabled(&mut self, value: bool) {
        self.system.set_dma_enabled(value);
    }

    pub fn timer_enabled(&self) -> bool {
        self.system.timer_enabled()
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        self.system.set_timer_enabled(value);
    }

    pub fn serial_enabled(&self) -> bool {
        self.system.serial_enabled()
    }

    pub fn set_serial_enabled(&mut self, value: bool) {
        self.system.set_serial_enabled(value);
    }

    pub fn version(&self) -> String {
        Info::version()
    }

    pub fn clock_freq_s(&self) -> String {
        self.system.clock_freq_s()
    }
}

#[pymodule]
fn boytacean(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<GameBoy>()?;
    module.add("DISPLAY_WIDTH", DISPLAY_WIDTH)?;
    module.add("DISPLAY_HEIGHT", DISPLAY_HEIGHT)?;
    module.add("CPU_FREQ", GameBoyBase::CPU_FREQ)?;
    Ok(())
}
