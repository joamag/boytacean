use pyo3::{exceptions::PyException, prelude::*, types::PyBytes};

use crate::{
    gb::{GameBoy as GameBoyBase, GameBoyMode},
    gen::{COMPILATION_DATE, COMPILATION_TIME, COMPILER, COMPILER_VERSION, NAME, VERSION},
    info::Info,
    pad::PadKey,
    ppu::{PaletteInfo, DISPLAY_HEIGHT, DISPLAY_WIDTH},
    state::StateManager,
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

    pub fn boot(&mut self) {
        self.system.boot();
    }

    pub fn load(&mut self, boot: bool) -> PyResult<()> {
        self.system.load(boot).map_err(PyErr::new::<PyException, _>)
    }

    pub fn load_boot(&mut self, data: &[u8]) {
        self.system.load_boot(data);
    }

    pub fn load_boot_path(&mut self, path: &str) -> PyResult<()> {
        self.system
            .load_boot_path(path)
            .map_err(PyErr::new::<PyException, _>)
    }

    pub fn load_rom(&mut self, data: &[u8]) -> PyResult<()> {
        self.system
            .load_rom(data, None)
            .map(|_| ())
            .map_err(PyErr::new::<PyException, _>)
    }

    pub fn load_rom_file(&mut self, path: &str) -> PyResult<()> {
        self.system
            .load_rom_file(path, None)
            .map(|_| ())
            .map_err(PyErr::new::<PyException, _>)
    }

    pub fn read_memory(&mut self, addr: u16) -> u8 {
        self.system.read_memory(addr)
    }

    pub fn write_memory(&mut self, addr: u16, value: u8) {
        self.system.write_memory(addr, value);
    }

    pub fn clock(&mut self) -> u16 {
        self.system.clock()
    }

    pub fn clock_many(&mut self, count: usize) -> u16 {
        self.system.clock_many(count)
    }

    pub fn clock_step(&mut self, addr: u16) -> u16 {
        self.system.clock_step(addr)
    }

    pub fn clocks(&mut self, count: usize) -> u64 {
        self.system.clocks(count)
    }

    pub fn clocks_cycles(&mut self, limit: usize) -> u64 {
        self.system.clocks_cycles(limit)
    }

    pub fn next_frame(&mut self) -> u32 {
        self.system.next_frame()
    }

    pub fn step_to(&mut self, addr: u16) -> u32 {
        self.system.step_to(addr)
    }

    pub fn key_press(&mut self, key: u8) {
        self.system.key_press(PadKey::from_u8(key))
    }

    pub fn key_lift(&mut self, key: u8) {
        self.system.key_lift(PadKey::from_u8(key))
    }

    pub fn frame_buffer(&mut self, py: Python) -> PyObject {
        let pybytes = PyBytes::new(py, self.system.frame_buffer());
        pybytes.into()
    }

    pub fn set_palette_colors(&mut self, colors_hex: &str) {
        let palette = PaletteInfo::from_colors_hex("default", colors_hex);
        self.system.ppu().set_palette_colors(palette.colors());
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

    pub fn rom_title(&self) -> String {
        self.system.rom_i().title()
    }

    pub fn version(&self) -> String {
        Info::version()
    }

    pub fn clock_freq_s(&self) -> String {
        self.system.clock_freq_s()
    }

    pub fn boot_rom_s(&self) -> String {
        self.system.boot_rom_s()
    }

    pub fn timer_div(&self) -> u8 {
        self.system.timer_i().div()
    }

    pub fn set_timer_div(&mut self, value: u8) {
        self.system.timer().set_div(value);
    }

    pub fn save_state(&mut self, py: Python) -> PyResult<PyObject> {
        match StateManager::save(&mut self.system, None, None) {
            Ok(data) => Ok(PyBytes::new(py, &data).into()),
            Err(e) => Err(PyErr::new::<PyException, _>(e)),
        }
    }

    pub fn load_state(&mut self, data: &[u8]) -> PyResult<()> {
        StateManager::load(data, &mut self.system, None, None).map_err(PyErr::new::<PyException, _>)
    }
}

#[pymodule]
fn boytacean(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<GameBoy>()?;
    module.add("__version__", VERSION)?;
    module.add("COMPILATION_DATE", COMPILATION_DATE)?;
    module.add("COMPILATION_TIME", COMPILATION_TIME)?;
    module.add("COMPILER", COMPILER)?;
    module.add("COMPILER_VERSION", COMPILER_VERSION)?;
    module.add("NAME", NAME)?;
    module.add("VERSION", VERSION)?;
    module.add("DISPLAY_WIDTH", DISPLAY_WIDTH)?;
    module.add("DISPLAY_HEIGHT", DISPLAY_HEIGHT)?;
    module.add("CPU_FREQ", GameBoyBase::CPU_FREQ)?;
    module.add("VISUAL_FREQ", GameBoyBase::VISUAL_FREQ)?;
    module.add("LCD_CYCLES", GameBoyBase::LCD_CYCLES)?;
    Ok(())
}
