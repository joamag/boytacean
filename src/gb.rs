//! Game Boy emulation entrypoint and associated functions and structures.
//!
//! Most of the meaningful publicly available functions and structures to build
//! a working emulator should be present here.
//!
//! # Examples
//!
//! Creates a simple [`GameBoy`] instance and boots the boot ROM. Does that by
//! clocking the CPU until PC reaches 0x0100 (post boot address).
//!
//! ```rust
//! use boytacean::gb::{GameBoy, GameBoyMode};
//! let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
//! game_boy.load(true).unwrap();
//! game_boy.load_rom_empty().unwrap();
//! let cycles = game_boy.step_to(0x0100);
//! println!("Ran {} cycles", cycles);
//! ```

use boytacean_common::error::Error;
use std::{
    collections::VecDeque,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use crate::{
    apu::Apu,
    cheats::{
        genie::{GameGenie, GameGenieCode},
        shark::{GameShark, GameSharkCode},
    },
    cpu::Cpu,
    data::{BootRom, CGB_BOOT, CGB_BOYTACEAN, DMG_BOOT, DMG_BOOTIX, MGB_BOOTIX, SGB_BOOT},
    devices::{printer::PrinterDevice, stdout::StdoutDevice},
    dma::Dma,
    info::Info,
    mmu::Mmu,
    pad::{Pad, PadKey},
    ppu::{
        Ppu, PpuMode, Tile, DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_RGB1555_SIZE,
        FRAME_BUFFER_RGB565_SIZE, FRAME_BUFFER_SIZE, FRAME_BUFFER_XRGB8888_SIZE,
    },
    rom::{Cartridge, RamSize},
    serial::{NullDevice, Serial, SerialDevice},
    timer::Timer,
    util::{read_file, SharedThread},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{color::Pixel, gen::dependencies_map, ppu::Palette};

#[cfg(feature = "wasm")]
use std::{
    convert::TryInto,
    panic::{set_hook, take_hook, PanicInfo},
};

/// Enumeration that describes the multiple running
// modes of the Game Boy emulator.
// DMG = Original Game Boy
// CGB = Game Boy Color
// SGB = Super Game Boy
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameBoyMode {
    Dmg = 1,
    Cgb = 2,
    Sgb = 3,
}

impl GameBoyMode {
    pub fn description(&self) -> &'static str {
        match self {
            GameBoyMode::Dmg => "Game Boy (DMG)",
            GameBoyMode::Cgb => "Game Boy Color (CGB)",
            GameBoyMode::Sgb => "Super Game Boy (SGB)",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => GameBoyMode::Dmg,
            2 => GameBoyMode::Cgb,
            3 => GameBoyMode::Sgb,
            _ => panic!("Invalid mode value: {}", value),
        }
    }

    pub fn from_string(value: &str) -> Self {
        match value {
            "dmg" | "DMG" => GameBoyMode::Dmg,
            "cgb" | "CGB" => GameBoyMode::Cgb,
            "sgb" | "SGB" => GameBoyMode::Sgb,
            _ => panic!("Invalid mode value: {}", value),
        }
    }

    pub fn to_string(&self, uppercase: Option<bool>) -> String {
        let uppercase = uppercase.unwrap_or(false);
        match self {
            GameBoyMode::Dmg => (if uppercase { "DMG" } else { "dmg" }).to_string(),
            GameBoyMode::Cgb => (if uppercase { "CGB" } else { "cgb" }).to_string(),
            GameBoyMode::Sgb => (if uppercase { "SGB" } else { "sgb" }).to_string(),
        }
    }

    pub fn is_dmg(&self) -> bool {
        *self == GameBoyMode::Dmg
    }

    pub fn is_cgb(&self) -> bool {
        *self == GameBoyMode::Cgb
    }

    pub fn is_sgb(&self) -> bool {
        *self == GameBoyMode::Sgb
    }
}

impl Display for GameBoyMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for GameBoyMode {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<&str> for GameBoyMode {
    fn from(value: &str) -> Self {
        Self::from_string(value)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameBoySpeed {
    Normal = 0,
    Double = 1,
}

impl GameBoySpeed {
    pub fn description(&self) -> &'static str {
        match self {
            GameBoySpeed::Normal => "Normal Speed",
            GameBoySpeed::Double => "Double Speed",
        }
    }

    pub fn switch(&self) -> Self {
        match self {
            GameBoySpeed::Normal => GameBoySpeed::Double,
            GameBoySpeed::Double => GameBoySpeed::Normal,
        }
    }

    pub fn multiplier(&self) -> u8 {
        match self {
            GameBoySpeed::Normal => 1,
            GameBoySpeed::Double => 2,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => GameBoySpeed::Normal,
            1 => GameBoySpeed::Double,
            _ => panic!("Invalid speed value: {}", value),
        }
    }
}

impl Display for GameBoySpeed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for GameBoySpeed {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GameBoyConfig {
    /// The current running mode of the emulator, this
    /// may affect many aspects of the emulation, like
    /// CPU frequency, PPU frequency, Boot rome size, etc.
    mode: GameBoyMode,

    /// If the PPU is enabled, it will be clocked.
    ppu_enabled: bool,

    /// If the APU is enabled, it will be clocked.
    apu_enabled: bool,

    /// if the DMA is enabled, it will be clocked.
    dma_enabled: bool,

    /// If the timer is enabled, it will be clocked.
    timer_enabled: bool,

    /// If the serial is enabled, it will be clocked.
    serial_enabled: bool,

    /// The current frequency at which the Game Boy
    /// emulator is being handled. This is a "hint" that
    /// may help components to adjust their internal
    /// logic to match the current frequency. For example
    /// the APU will adjust its internal clock to match
    /// this hint.
    clock_freq: u32,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoyConfig {
    pub fn is_dmg(&self) -> bool {
        self.mode == GameBoyMode::Dmg
    }

    pub fn is_cgb(&self) -> bool {
        self.mode == GameBoyMode::Cgb
    }

    pub fn is_sgb(&self) -> bool {
        self.mode == GameBoyMode::Sgb
    }

    pub fn mode(&self) -> GameBoyMode {
        self.mode
    }

    pub fn set_mode(&mut self, value: GameBoyMode) {
        self.mode = value;
    }

    pub fn ppu_enabled(&self) -> bool {
        self.ppu_enabled
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        self.ppu_enabled = value;
    }

    pub fn apu_enabled(&self) -> bool {
        self.apu_enabled
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        self.apu_enabled = value;
    }

    pub fn dma_enabled(&self) -> bool {
        self.dma_enabled
    }

    pub fn set_dma_enabled(&mut self, value: bool) {
        self.dma_enabled = value;
    }

    pub fn timer_enabled(&self) -> bool {
        self.timer_enabled
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        self.timer_enabled = value;
    }

    pub fn serial_enabled(&self) -> bool {
        self.serial_enabled
    }

    pub fn set_serial_enabled(&mut self, value: bool) {
        self.serial_enabled = value;
    }

    pub fn clock_freq(&self) -> u32 {
        self.clock_freq
    }

    pub fn set_clock_freq(&mut self, value: u32) {
        self.clock_freq = value;
    }
}

impl Default for GameBoyConfig {
    fn default() -> Self {
        Self {
            mode: GameBoyMode::Dmg,
            ppu_enabled: true,
            apu_enabled: true,
            dma_enabled: true,
            timer_enabled: true,
            serial_enabled: true,
            clock_freq: GameBoy::CPU_FREQ,
        }
    }
}

/// Aggregation structure allowing the bundling of
/// all the components of a GameBoy into a single
/// element for easy access.
pub struct Components {
    pub ppu: Ppu,
    pub apu: Apu,
    pub dma: Dma,
    pub pad: Pad,
    pub timer: Timer,
    pub serial: Serial,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Registers {
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub scy: u8,
    pub scx: u8,
    pub wy: u8,
    pub wx: u8,
    pub ly: u8,
    pub lyc: u8,
}

pub trait AudioProvider {
    fn audio_output(&self) -> u8;
    fn audio_buffer(&self) -> &VecDeque<u8>;
    fn clear_audio_buffer(&mut self);
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct ClockFrame {
    pub cycles: u64,
    pub frames: u16,
    frame_buffer: Option<Vec<u8>>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl ClockFrame {
    pub fn frame_buffer_eager(&mut self) -> Option<Vec<u8>> {
        self.frame_buffer.take()
    }
}

/// Top level structure that abstracts the usage of the
/// Game Boy system under the Boytacean emulator.
///
/// Should serve as the main entry-point API.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameBoy {
    /// The current running mode of the emulator, this
    /// may affect many aspects of the emulation, like
    /// CPU frequency, PPU frequency, Boot rome size, etc.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    mode: GameBoyMode,

    /// If the PPU is enabled, it will be clocked.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    ppu_enabled: bool,

    /// If the APU is enabled, it will be clocked.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    apu_enabled: bool,

    /// If the DMA is enabled, it will be clocked.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    dma_enabled: bool,

    /// If the timer is enabled, it will be clocked.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    timer_enabled: bool,

    /// If the serial is enabled, it will be clocked.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    serial_enabled: bool,

    /// The current frequency at which the Game Boy
    /// emulator is being handled. This is a "hint" that
    /// may help components to adjust their internal
    /// logic to match the current frequency. For example
    /// the APU will adjust its internal clock to match
    /// this hint.
    ///
    /// This is a clone of the configuration value
    /// kept for performance reasons.
    clock_freq: u32,

    /// The boot ROM that will (or was) used to boot the
    /// current Game Boy system.
    ///
    /// This should be explicitly set by the developed when
    /// set the boot ROM in the system's memory.
    ///
    /// The loading process used to load the boot ROM is not
    /// taken in consideration for this value.
    boot_rom: BootRom,

    /// Reference to the Game Boy CPU component to be
    /// used as the main element of the system, when
    /// clocked, the amount of ticks from it will be
    /// used as reference or the rest of the components.
    cpu: Cpu,

    /// The reference counted and mutable reference to
    /// Game Boy configuration structure that can be
    /// used by the GB components to access global
    /// configuration values on the current emulator.
    ///
    /// If performance is required (may value access)
    /// the values should be cloned and stored locally.
    gbc: SharedThread<GameBoyConfig>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoy {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(mode: Option<GameBoyMode>) -> Self {
        let mode = mode.unwrap_or(GameBoyMode::Dmg);
        let gbc = Arc::new(Mutex::new(GameBoyConfig {
            mode,
            ppu_enabled: true,
            apu_enabled: true,
            dma_enabled: true,
            timer_enabled: true,
            serial_enabled: true,
            clock_freq: GameBoy::CPU_FREQ,
        }));

        let components = Components {
            ppu: Ppu::new(mode, gbc.clone()),
            apu: Apu::default(),
            dma: Dma::default(),
            pad: Pad::default(),
            timer: Timer::default(),
            serial: Serial::default(),
        };
        let mmu = Mmu::new(components, mode, gbc.clone());
        let cpu = Cpu::new(mmu, gbc.clone());

        Self {
            mode,
            boot_rom: BootRom::None,
            ppu_enabled: true,
            apu_enabled: true,
            dma_enabled: true,
            timer_enabled: true,
            serial_enabled: true,
            clock_freq: GameBoy::CPU_FREQ,
            cpu,
            gbc,
        }
    }

    pub fn verify_rom(data: &[u8]) -> bool {
        Cartridge::from_data(data).is_ok()
    }

    pub fn reset(&mut self) {
        self.ppu().reset();
        self.apu().reset();
        self.timer().reset();
        self.serial().reset();
        self.mmu().reset();
        self.cpu.reset();
        self.reset_cheats();
    }

    pub fn reload(&mut self) {
        let rom = self.rom().clone();
        self.reset();
        self.load(true).unwrap();
        self.load_cartridge(rom).unwrap();
    }

    /// Advance the clock of the system by one tick, this will
    /// usually imply executing one CPU instruction and advancing
    /// all the other components of the system by the required
    /// amount of cycles.
    ///
    /// This method takes into account the current speed of the
    /// system (single or double) and will execute the required
    /// amount of cycles in the other components of the system
    /// accordingly.
    ///
    /// The amount of cycles executed by the CPU is returned.
    pub fn clock(&mut self) -> u16 {
        let cycles = self.cpu_clock() as u16;
        let cycles_n = cycles / self.multiplier() as u16;
        self.clock_devices(cycles, cycles_n);
        cycles
    }

    /// Risky function that will clock the CPU multiple times
    /// allowing an undefined number of cycles to be executed
    /// in the other Game Boy components.
    ///
    /// This can cause unwanted behaviour in components like
    /// the PPU where only one mode switch operation is expected
    /// per each clock call.
    ///
    /// At the end of this execution major synchronization issues
    /// may arise, so use with caution.
    pub fn clock_many(&mut self, count: usize) -> u16 {
        let mut cycles = 0u16;
        for _ in 0..count {
            cycles += self.cpu_clock() as u16;
        }
        let cycles_n = cycles / self.multiplier() as u16;
        self.clock_devices(cycles, cycles_n);
        cycles
    }

    /// Function equivalent to `clock()` but that allows pre-emptive
    /// breaking of the clock cycle loop if the PC (Program Counter)
    /// reaches the provided address, making sure that in such a situation
    /// the devices are not clocked.
    pub fn clock_step(&mut self, addr: u16) -> u16 {
        let cycles = self.cpu_clock() as u16;
        if self.cpu_i().pc() == addr {
            return cycles;
        }
        let cycles_n = cycles / self.multiplier() as u16;
        self.clock_devices(cycles, cycles_n);
        cycles
    }

    /// Equivalent to `clock()` but allows the execution of multiple
    /// clock operations in a single call.
    pub fn clocks(&mut self, count: usize) -> u64 {
        let mut cycles = 0_u64;
        for _ in 0..count {
            cycles += self.clock() as u64;
        }
        cycles
    }

    /// Clocks the emulator until the limit of cycles that has been
    /// provided and returns the amount of cycles that have been
    /// clocked.
    pub fn clocks_cycles(&mut self, limit: usize) -> u64 {
        let mut cycles = 0_u64;
        while cycles < limit as u64 {
            cycles += self.clock() as u64;
        }
        cycles
    }

    /// Clocks the emulator until the limit of cycles that has been
    /// provided and returns the amount of cycles that have been
    /// clocked together with the frame buffer of the PPU.
    ///
    /// Allows a caller to clock the emulator and at the same time
    /// retrieve the frame buffer of the PPU at the proper timing
    /// (on V-Blank).
    ///
    /// This method allows for complex foreign call optimizations
    /// by preventing the need to call the emulator clock multiple
    /// times to obtain the right frame buffer retrieval timing.
    pub fn clocks_frame_buffer(&mut self, limit: usize) -> ClockFrame {
        let mut cycles = 0_u64;
        let mut frames = 0_u16;
        let mut frame_buffer: Option<Vec<u8>> = None;
        let mut last_frame = self.ppu_frame();
        while cycles < limit as u64 {
            cycles += self.clock() as u64;
            if self.ppu_frame() != last_frame {
                frame_buffer = Some(self.frame_buffer().to_vec());
                last_frame = self.ppu_frame();
                frames += 1;
            }
        }
        ClockFrame {
            cycles,
            frames,
            frame_buffer,
        }
    }

    pub fn next_frame(&mut self) -> u32 {
        let mut cycles = 0u32;
        let current_frame = self.ppu_frame();
        while self.ppu_frame() == current_frame {
            cycles += self.clock() as u32;
        }
        cycles
    }

    pub fn step_to(&mut self, addr: u16) -> u32 {
        let mut cycles = 0u32;
        while self.cpu_i().pc() != addr {
            cycles += self.clock_step(addr) as u32;
        }
        cycles
    }

    #[inline(always)]
    fn clock_devices(&mut self, cycles: u16, cycles_n: u16) {
        if self.ppu_enabled {
            self.ppu_clock(cycles_n);
        }
        if self.apu_enabled {
            self.apu_clock(cycles_n);
        }
        if self.dma_enabled {
            self.dma_clock(cycles);
        }
        if self.timer_enabled {
            self.timer_clock(cycles);
        }
        if self.serial_enabled {
            self.serial_clock(cycles);
        }
    }

    pub fn key_press(&mut self, key: PadKey) {
        self.pad().key_press(key);
    }

    pub fn key_lift(&mut self, key: PadKey) {
        self.pad().key_lift(key);
    }

    pub fn cpu_clock(&mut self) -> u8 {
        self.cpu.clock()
    }

    pub fn ppu_clock(&mut self, cycles: u16) {
        self.ppu().clock(cycles)
    }

    pub fn apu_clock(&mut self, cycles: u16) {
        self.apu().clock(cycles)
    }

    pub fn dma_clock(&mut self, cycles: u16) {
        self.mmu().clock_dma(cycles);
    }

    pub fn timer_clock(&mut self, cycles: u16) {
        self.timer().clock(cycles)
    }

    pub fn serial_clock(&mut self, cycles: u16) {
        self.serial().clock(cycles)
    }

    pub fn ppu_ly(&mut self) -> u8 {
        self.ppu().ly()
    }

    pub fn ppu_mode(&mut self) -> PpuMode {
        self.ppu().mode()
    }

    pub fn ppu_frame(&mut self) -> u16 {
        self.ppu().frame_index()
    }

    /// Direct boot method that immediately jumps the machine
    /// to the post boot state, this will effectively skip the
    /// boot sequence and jump to the cartridge execution.
    pub fn boot(&mut self) {
        self.load_boot_state();
    }

    /// Unsafe load strategy that will panic the current system
    /// in case there are boot ROM loading issues.
    pub fn load_unsafe(&mut self, boot: bool) {
        self.load(boot).unwrap();
    }

    /// Loads the machine directly to after the boot execution state,
    /// setting the state of the system accordingly and updating the
    /// Program Counter (PC) to the post boot address (0x0100).
    ///
    /// Should allow the machine to jump to the cartridge (ROM) execution
    /// directly, skipping the boot sequence.
    ///
    /// Currently supports only DMG machines.
    pub fn load_boot_state(&mut self) {
        self.cpu.boot();
    }

    pub fn vram_eager(&mut self) -> Vec<u8> {
        self.ppu().vram().to_vec()
    }

    pub fn hram_eager(&mut self) -> Vec<u8> {
        self.ppu().vram().to_vec()
    }

    pub fn frame_buffer_eager(&mut self) -> Vec<u8> {
        self.frame_buffer().to_vec()
    }

    pub fn frame_buffer_raw_eager(&mut self) -> Vec<u8> {
        self.frame_buffer_raw().to_vec()
    }

    pub fn audio_buffer_eager(&mut self, clear: bool) -> Vec<u8> {
        let buffer = Vec::from(self.audio_buffer().clone());
        if clear {
            self.clear_audio_buffer();
        }
        buffer
    }

    pub fn audio_output(&self) -> u8 {
        self.apu_i().output()
    }

    pub fn audio_all_output(&self) -> Vec<u8> {
        vec![
            self.audio_output(),
            self.audio_ch1_output(),
            self.audio_ch2_output(),
            self.audio_ch3_output(),
            self.audio_ch4_output(),
        ]
    }

    pub fn audio_ch1_output(&self) -> u8 {
        self.apu_i().ch1_output()
    }

    pub fn audio_ch2_output(&self) -> u8 {
        self.apu_i().ch2_output()
    }

    pub fn audio_ch3_output(&self) -> u8 {
        self.apu_i().ch3_output()
    }

    pub fn audio_ch4_output(&self) -> u8 {
        self.apu_i().ch4_output()
    }

    pub fn audio_ch1_enabled(&self) -> bool {
        self.apu_i().ch2_out_enabled()
    }

    pub fn set_audio_ch1_enabled(&mut self, enabled: bool) {
        self.apu().set_ch1_out_enabled(enabled)
    }

    pub fn audio_ch2_enabled(&self) -> bool {
        self.apu_i().ch2_out_enabled()
    }

    pub fn set_audio_ch2_enabled(&mut self, enabled: bool) {
        self.apu().set_ch2_out_enabled(enabled)
    }

    pub fn audio_ch3_enabled(&self) -> bool {
        self.apu_i().ch3_out_enabled()
    }

    pub fn set_audio_ch3_enabled(&mut self, enabled: bool) {
        self.apu().set_ch3_out_enabled(enabled)
    }

    pub fn audio_ch4_enabled(&self) -> bool {
        self.apu_i().ch4_out_enabled()
    }

    pub fn set_audio_ch4_enabled(&mut self, enabled: bool) {
        self.apu().set_ch4_out_enabled(enabled)
    }

    pub fn audio_sampling_rate(&self) -> u16 {
        self.apu_i().sampling_rate()
    }

    pub fn audio_channels(&self) -> u8 {
        self.apu_i().channels()
    }

    pub fn cartridge_eager(&mut self) -> Cartridge {
        self.mmu().rom().clone()
    }

    pub fn ram_data_eager(&mut self) -> Vec<u8> {
        self.mmu().rom().ram_data_eager()
    }

    pub fn set_ram_data(&mut self, ram_data: Vec<u8>) {
        self.mmu().rom().set_ram_data(&ram_data)
    }

    pub fn registers(&mut self) -> Registers {
        let ppu_registers = self.ppu().registers();
        Registers {
            pc: self.cpu.pc,
            sp: self.cpu.sp,
            a: self.cpu.a,
            b: self.cpu.b,
            c: self.cpu.c,
            d: self.cpu.d,
            e: self.cpu.e,
            h: self.cpu.h,
            l: self.cpu.l,
            scy: ppu_registers.scy,
            scx: ppu_registers.scx,
            wy: ppu_registers.wy,
            wx: ppu_registers.wx,
            ly: ppu_registers.ly,
            lyc: ppu_registers.lyc,
        }
    }

    /// Obtains the tile structure for the tile at the
    /// given index, no conversion in the pixel buffer
    /// is done so that the color reference is the GB one.
    pub fn get_tile(&mut self, index: usize) -> Tile {
        self.ppu().tiles()[index]
    }

    /// Obtains the pixel buffer for the tile at the
    /// provided index, converting the color buffer
    /// using the currently loaded (background) palette.
    pub fn get_tile_buffer(&mut self, index: usize) -> Vec<u8> {
        let tile = self.get_tile(index);
        tile.palette_buffer(self.ppu().palette_bg())
    }

    pub fn is_dmg(&self) -> bool {
        self.mode == GameBoyMode::Dmg
    }

    pub fn is_cgb(&self) -> bool {
        self.mode == GameBoyMode::Cgb
    }

    pub fn is_sgb(&self) -> bool {
        self.mode == GameBoyMode::Sgb
    }

    pub fn speed(&self) -> GameBoySpeed {
        self.mmu_i().speed()
    }

    pub fn multiplier(&self) -> u8 {
        self.mmu_i().speed().multiplier()
    }

    pub fn mode(&self) -> GameBoyMode {
        self.mode
    }

    pub fn set_mode(&mut self, value: GameBoyMode) {
        self.mode = value;
        (*self.gbc).lock().unwrap().set_mode(value);
        self.mmu().set_mode(value);
        self.ppu().set_gb_mode(value);
    }

    pub fn ppu_enabled(&self) -> bool {
        self.ppu_enabled
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        self.ppu_enabled = value;
        (*self.gbc).lock().unwrap().set_ppu_enabled(value);
    }

    pub fn apu_enabled(&self) -> bool {
        self.apu_enabled
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        self.apu_enabled = value;
        (*self.gbc).lock().unwrap().set_apu_enabled(value);
    }

    pub fn dma_enabled(&self) -> bool {
        self.dma_enabled
    }

    pub fn set_dma_enabled(&mut self, value: bool) {
        self.dma_enabled = value;
        (*self.gbc).lock().unwrap().set_dma_enabled(value);
    }

    pub fn timer_enabled(&self) -> bool {
        self.timer_enabled
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        self.timer_enabled = value;
        (*self.gbc).lock().unwrap().set_timer_enabled(value);
    }

    pub fn serial_enabled(&self) -> bool {
        self.serial_enabled
    }

    pub fn set_serial_enabled(&mut self, value: bool) {
        self.serial_enabled = value;
        (*self.gbc).lock().unwrap().set_serial_enabled(value);
    }

    pub fn set_all_enabled(&mut self, value: bool) {
        self.set_ppu_enabled(value);
        self.set_apu_enabled(value);
        self.set_dma_enabled(value);
        self.set_timer_enabled(value);
        self.set_serial_enabled(value);
    }

    pub fn clock_freq(&self) -> u32 {
        self.clock_freq
    }

    pub fn set_clock_freq(&mut self, value: u32) {
        self.clock_freq = value;
        (*self.gbc).lock().unwrap().set_clock_freq(value);
        self.apu().set_clock_freq(value);
    }

    pub fn clock_freq_s(&self) -> String {
        format!("{:.02} Mhz", self.clock_freq() as f32 / 1000.0 / 1000.0)
    }

    pub fn boot_rom(&self) -> BootRom {
        self.boot_rom
    }

    pub fn set_boot_rom(&mut self, value: BootRom) {
        self.boot_rom = value;
    }

    pub fn boot_rom_s(&self) -> String {
        String::from(self.boot_rom().description())
    }

    pub fn attach_null_serial(&mut self) {
        self.attach_serial(Box::<NullDevice>::default());
    }

    pub fn attach_stdout_serial(&mut self) {
        self.attach_serial(Box::<StdoutDevice>::default());
    }

    pub fn attach_printer_serial(&mut self) {
        self.attach_serial(Box::<PrinterDevice>::default());
    }

    pub fn display_width(&self) -> usize {
        DISPLAY_WIDTH
    }

    pub fn display_height(&self) -> usize {
        DISPLAY_HEIGHT
    }

    pub fn ram_size(&self) -> RamSize {
        match self.mode {
            GameBoyMode::Dmg => RamSize::Size8K,
            GameBoyMode::Cgb => RamSize::Size32K,
            GameBoyMode::Sgb => RamSize::Size8K,
        }
    }

    pub fn vram_size(&self) -> RamSize {
        match self.mode {
            GameBoyMode::Dmg => RamSize::Size8K,
            GameBoyMode::Cgb => RamSize::Size16K,
            GameBoyMode::Sgb => RamSize::Size8K,
        }
    }

    pub fn description(&self, column_length: usize) -> String {
        let version_l = format!("{:width$}", "Version", width = column_length);
        let mode_l = format!("{:width$}", "Mode", width = column_length);
        let boot_rom_l = format!("{:width$}", "Boot ROM", width = column_length);
        let clock_l = format!("{:width$}", "Clock", width = column_length);
        let ram_size_l = format!("{:width$}", "RAM Size", width = column_length);
        let vram_size_l = format!("{:width$}", "VRAM Size", width = column_length);
        let serial_l = format!("{:width$}", "Serial", width = column_length);
        format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}",
            version_l,
            Info::version(),
            mode_l,
            self.mode(),
            boot_rom_l,
            self.boot_rom(),
            clock_l,
            self.clock_freq_s(),
            ram_size_l,
            self.ram_size(),
            vram_size_l,
            self.vram_size(),
            serial_l,
            self.serial_i().device().description(),
        )
    }

    pub fn description_debug(&self) -> String {
        format!(
            "{}\nCPU:\n{}\nDMA:\n{}",
            self.description(12),
            self.cpu_i().description_default(),
            self.dma_i().description()
        )
    }
}

/// Gameboy implementations that are meant with performance
/// in mind and that do not support WASM interface of copy.
impl GameBoy {
    /// The logic frequency of the Game Boy
    /// CPU in hz.
    pub const CPU_FREQ: u32 = 4194304;

    /// The visual frequency (refresh rate)
    /// of the Game Boy, close to 60 hz.
    pub const VISUAL_FREQ: f32 = 59.7275;

    /// The cycles taken to run a complete frame
    /// loop in the Game Boy's PPU (in CPU cycles).
    pub const LCD_CYCLES: u32 = 70224;

    pub fn cpu(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn cpu_i(&self) -> &Cpu {
        &self.cpu
    }

    pub fn mmu(&mut self) -> &mut Mmu {
        self.cpu.mmu()
    }

    pub fn mmu_i(&self) -> &Mmu {
        self.cpu.mmu_i()
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        self.cpu.ppu()
    }

    pub fn ppu_i(&self) -> &Ppu {
        self.cpu.ppu_i()
    }

    pub fn apu(&mut self) -> &mut Apu {
        self.cpu.apu()
    }

    pub fn apu_i(&self) -> &Apu {
        self.cpu.apu_i()
    }

    pub fn dma(&mut self) -> &mut Dma {
        self.cpu.dma()
    }

    pub fn dma_i(&self) -> &Dma {
        self.cpu.dma_i()
    }

    pub fn pad(&mut self) -> &mut Pad {
        self.cpu.pad()
    }

    pub fn pad_i(&self) -> &Pad {
        self.cpu.pad_i()
    }

    pub fn timer(&mut self) -> &mut Timer {
        self.cpu.timer()
    }

    pub fn timer_i(&self) -> &Timer {
        self.cpu.timer_i()
    }

    pub fn serial(&mut self) -> &mut Serial {
        self.cpu.serial()
    }

    pub fn serial_i(&self) -> &Serial {
        self.cpu.serial_i()
    }

    pub fn rom(&mut self) -> &mut Cartridge {
        self.mmu().rom()
    }

    pub fn rom_i(&self) -> &Cartridge {
        self.mmu_i().rom_i()
    }

    pub fn frame_buffer(&mut self) -> &[u8; FRAME_BUFFER_SIZE] {
        self.ppu().frame_buffer()
    }

    pub fn frame_buffer_xrgb8888(&mut self) -> [u8; FRAME_BUFFER_XRGB8888_SIZE] {
        self.ppu().frame_buffer_xrgb8888()
    }

    pub fn frame_buffer_xrgb8888_u32(&mut self) -> [u32; FRAME_BUFFER_SIZE] {
        self.ppu().frame_buffer_xrgb8888_u32()
    }

    pub fn frame_buffer_rgb1555(&mut self) -> [u8; FRAME_BUFFER_RGB1555_SIZE] {
        self.ppu().frame_buffer_rgb1555()
    }

    pub fn frame_buffer_rgb1555_u16(&mut self) -> [u16; FRAME_BUFFER_SIZE] {
        self.ppu().frame_buffer_rgb1555_u16()
    }

    pub fn frame_buffer_rgb565(&mut self) -> [u8; FRAME_BUFFER_RGB565_SIZE] {
        self.ppu().frame_buffer_rgb565()
    }

    pub fn frame_buffer_rgb565_u16(&mut self) -> [u16; FRAME_BUFFER_SIZE] {
        self.ppu().frame_buffer_rgb565_u16()
    }

    pub fn frame_buffer_raw(&mut self) -> [u8; FRAME_BUFFER_SIZE] {
        self.ppu().frame_buffer_raw()
    }

    pub fn audio_buffer(&mut self) -> &VecDeque<u8> {
        self.apu().audio_buffer()
    }

    pub fn cartridge(&mut self) -> &mut Cartridge {
        self.mmu().rom()
    }

    pub fn cartridge_i(&self) -> &Cartridge {
        self.mmu_i().rom_i()
    }

    pub fn load(&mut self, boot: bool) -> Result<(), Error> {
        let boot_rom = self.boot_rom().reusable(self.mode());
        match self.mode() {
            GameBoyMode::Dmg => self.load_dmg(boot, boot_rom)?,
            GameBoyMode::Cgb => self.load_cgb(boot, boot_rom)?,
            GameBoyMode::Sgb => unimplemented!("SGB is not supported"),
        }
        Ok(())
    }

    pub fn load_dmg(&mut self, boot: bool, boot_rom: Option<BootRom>) -> Result<(), Error> {
        self.mmu().allocate_dmg();
        if boot {
            self.load_boot_dmg(boot_rom)?;
        }
        Ok(())
    }

    pub fn load_cgb(&mut self, boot: bool, boot_rom: Option<BootRom>) -> Result<(), Error> {
        self.mmu().allocate_cgb();
        if boot {
            self.load_boot_cgb(boot_rom)?;
        }
        Ok(())
    }

    pub fn load_boot(&mut self, data: &[u8]) {
        self.cpu.mmu().write_boot(0x0000, data);
    }

    pub fn load_boot_path(&mut self, path: &str) -> Result<(), Error> {
        let data = read_file(path)?;
        self.load_boot(&data);
        self.boot_rom = BootRom::Other;
        Ok(())
    }

    pub fn load_boot_static(&mut self, boot_rom: BootRom) {
        match boot_rom {
            BootRom::Dmg => self.load_boot(&DMG_BOOT),
            BootRom::Sgb => self.load_boot(&SGB_BOOT),
            BootRom::DmgBootix => self.load_boot(&DMG_BOOTIX),
            BootRom::MgbBootix => self.load_boot(&MGB_BOOTIX),
            BootRom::Cgb => self.load_boot(&CGB_BOOT),
            BootRom::CgbBoytacean => self.load_boot(&CGB_BOYTACEAN),
            BootRom::Other | BootRom::None => (),
        }
        self.boot_rom = boot_rom;
    }

    pub fn load_boot_file(&mut self, boot_rom: BootRom) -> Result<(), Error> {
        match boot_rom {
            BootRom::Dmg => self.load_boot_path("./res/boot/dmg_boot.bin")?,
            BootRom::Sgb => self.load_boot_path("./res/boot/sgb_boot.bin")?,
            BootRom::DmgBootix => self.load_boot_path("./res/boot/dmg_bootix.bin")?,
            BootRom::MgbBootix => self.load_boot_path("./res/boot/mgb_bootix.bin")?,
            BootRom::Cgb => self.load_boot_path("./res/boot/cgb_boot.bin")?,
            BootRom::CgbBoytacean => self.load_boot_path("./res/boot/cgb_boytacean.bin")?,
            BootRom::Other | BootRom::None => (),
        }
        self.boot_rom = boot_rom;
        Ok(())
    }

    pub fn load_boot_default(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        self.load_boot_dmg(boot_rom)
    }

    pub fn load_boot_smart(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        match self.mode() {
            GameBoyMode::Dmg => self.load_boot_dmg(boot_rom)?,
            GameBoyMode::Cgb => self.load_boot_cgb(boot_rom)?,
            GameBoyMode::Sgb => unimplemented!("SGB is not supported"),
        }
        Ok(())
    }

    pub fn load_boot_dmg(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        let boot_rom = boot_rom.unwrap_or(BootRom::DmgBootix);
        if !boot_rom.is_dmg_compat() {
            return Err(Error::IncompatibleBootRom);
        }
        self.load_boot_static(boot_rom);
        Ok(())
    }

    pub fn load_boot_cgb(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        let boot_rom = boot_rom.unwrap_or(BootRom::CgbBoytacean);
        if !boot_rom.is_cgb_compat() {
            return Err(Error::IncompatibleBootRom);
        }
        self.load_boot_static(boot_rom);
        Ok(())
    }

    pub fn load_boot_default_f(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        self.load_boot_dmg_f(boot_rom)?;
        Ok(())
    }

    pub fn load_boot_smart_f(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        match self.mode() {
            GameBoyMode::Dmg => self.load_boot_dmg_f(boot_rom)?,
            GameBoyMode::Cgb => self.load_boot_cgb_f(boot_rom)?,
            GameBoyMode::Sgb => unimplemented!("SGB is not supported"),
        }
        Ok(())
    }

    pub fn load_boot_dmg_f(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        let boot_rom = boot_rom.unwrap_or(BootRom::DmgBootix);
        if !boot_rom.is_dmg_compat() {
            return Err(Error::IncompatibleBootRom);
        }
        self.load_boot_file(boot_rom)?;
        Ok(())
    }

    pub fn load_boot_cgb_f(&mut self, boot_rom: Option<BootRom>) -> Result<(), Error> {
        let boot_rom = boot_rom.unwrap_or(BootRom::Cgb);
        if !boot_rom.is_cgb_compat() {
            return Err(Error::IncompatibleBootRom);
        }
        self.load_boot_file(boot_rom)?;
        Ok(())
    }

    pub fn load_cartridge(&mut self, rom: Cartridge) -> Result<&mut Cartridge, Error> {
        self.mmu().set_rom(rom);
        Ok(self.mmu().rom())
    }

    pub fn load_rom(
        &mut self,
        data: &[u8],
        ram_data: Option<&[u8]>,
    ) -> Result<&mut Cartridge, Error> {
        let mut rom = Cartridge::from_data(data)?;
        if let Some(ram_data) = ram_data {
            rom.set_ram_data(ram_data)
        }
        self.load_cartridge(rom)
    }

    pub fn load_rom_file(
        &mut self,
        path: &str,
        ram_path: Option<&str>,
    ) -> Result<&mut Cartridge, Error> {
        let data = read_file(path)?;
        match ram_path {
            Some(ram_path) => {
                let ram_data = read_file(ram_path)?;
                self.load_rom(&data, Some(&ram_data))
            }
            None => self.load_rom(&data, None),
        }
    }

    pub fn load_rom_empty(&mut self) -> Result<&mut Cartridge, Error> {
        let data = [0u8; 32 * 1024];
        self.load_rom(&data, None)
    }

    pub fn attach_serial(&mut self, device: Box<dyn SerialDevice>) {
        self.serial().set_device(device);
    }

    pub fn read_memory(&mut self, addr: u16) -> u8 {
        self.mmu().read(addr)
    }

    pub fn write_memory(&mut self, addr: u16, value: u8) {
        self.mmu().write(addr, value);
    }

    pub fn set_speed_callback(&mut self, callback: fn(speed: GameBoySpeed)) {
        self.mmu().set_speed_callback(callback);
    }

    pub fn reset_cheats(&mut self) {
        self.reset_game_genie();
        self.reset_game_shark();
    }

    pub fn add_cheat_code(&mut self, code: &str) -> Result<bool, Error> {
        if GameGenie::is_code(code) {
            return self.add_game_genie_code(code).map(|_| true);
        }

        if GameShark::is_code(code) {
            return self.add_game_shark_code(code).map(|_| true);
        }

        Err(Error::CustomError(String::from("Not a valid cheat code")))
    }

    pub fn add_game_genie_code(&mut self, code: &str) -> Result<&GameGenieCode, Error> {
        let rom = self.mmu().rom();
        if rom.game_genie().is_none() {
            let game_genie = GameGenie::default();
            rom.attach_genie(game_genie);
        }
        let game_genie = rom.game_genie_mut().as_mut().unwrap();
        game_genie.add_code(code)
    }

    pub fn add_game_shark_code(&mut self, code: &str) -> Result<&GameSharkCode, Error> {
        let rom = self.rom();
        if rom.game_shark().is_none() {
            let game_shark = GameShark::default();
            rom.attach_shark(game_shark);
        }
        let game_shark = rom.game_shark_mut().as_mut().unwrap();
        game_shark.add_code(code)
    }

    pub fn reset_game_genie(&mut self) {
        let rom = self.rom();
        if rom.game_genie().is_some() {
            rom.game_genie_mut().as_mut().unwrap().reset();
        }
    }

    pub fn reset_game_shark(&mut self) {
        let rom = self.mmu().rom();
        if rom.game_shark().is_some() {
            rom.game_shark_mut().as_mut().unwrap().reset();
        }
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoy {
    pub fn set_panic_hook_wa() {
        let prev = take_hook();
        set_hook(Box::new(move |info| {
            hook_impl(info);
            prev(info);
        }));
    }

    pub fn load_rom_wa(&mut self, data: &[u8]) -> Result<Cartridge, String> {
        let rom = self.load_rom(data, None)?;
        rom.set_rumble_cb(|active| {
            rumble_callback(active);
        });
        Ok(rom.clone())
    }

    pub fn load_callbacks_wa(&mut self) {
        self.set_speed_callback(|speed| {
            speed_callback(speed);
        });
    }

    pub fn load_null_wa(&mut self) {
        let null = Box::<NullDevice>::default();
        self.attach_serial(null);
    }

    pub fn load_logger_wa(&mut self) {
        let mut logger = Box::<StdoutDevice>::default();
        logger.set_callback(|data| {
            logger_callback(data.to_vec());
        });
        self.attach_serial(logger);
    }

    pub fn load_printer_wa(&mut self) {
        let mut printer = Box::<PrinterDevice>::default();
        printer.set_callback(|image_buffer| {
            printer_callback(image_buffer.to_vec());
        });
        self.attach_serial(printer);
    }

    /// Updates the emulation mode using the cartridge info
    /// for the provided data to obtain the CGB flag value
    /// and set the mode accordingly.
    ///
    /// This can be an expensive operation as it will require
    /// cartridge data parsing to obtain the CGB flag.
    /// It will also have to clone the data buffer.
    pub fn infer_mode_wa(&mut self, data: &[u8]) -> Result<(), String> {
        let mode = Cartridge::from_data(data)?.gb_mode();
        self.set_mode(mode);
        Ok(())
    }

    pub fn set_palette_colors_wa(&mut self, value: Vec<JsValue>) {
        let palette: Palette = value
            .into_iter()
            .map(|v| Self::js_to_pixel(&v))
            .collect::<Vec<Pixel>>()
            .try_into()
            .unwrap();
        self.ppu().set_palette_colors(&palette);
    }

    pub fn wasm_engine_wa(&self) -> Option<String> {
        let dependencies = dependencies_map();
        if !dependencies.contains_key("wasm-bindgen") {
            return None;
        }
        Some(String::from(format!(
            "wasm-bindgen/{}",
            *dependencies.get("wasm-bindgen").unwrap()
        )))
    }

    fn js_to_pixel(value: &JsValue) -> Pixel {
        value
            .as_string()
            .unwrap()
            .chars()
            .collect::<Vec<char>>()
            .chunks(2)
            .map(|s| s.iter().collect::<String>())
            .map(|s| u8::from_str_radix(&s, 16).unwrap())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn panic(message: &str);

    #[wasm_bindgen(js_namespace = window, js_name = speedCallback)]
    fn speed_callback(speed: GameBoySpeed);

    #[wasm_bindgen(js_namespace = window, js_name = loggerCallback)]
    fn logger_callback(data: Vec<u8>);

    #[wasm_bindgen(js_namespace = window, js_name = printerCallback)]
    fn printer_callback(image_buffer: Vec<u8>);

    #[wasm_bindgen(js_namespace = window, js_name = rumbleCallback)]
    fn rumble_callback(active: bool);
}

#[cfg(feature = "wasm")]
pub fn hook_impl(info: &PanicInfo) {
    let message = info.to_string();
    panic(message.as_str());
}

impl AudioProvider for GameBoy {
    fn audio_output(&self) -> u8 {
        self.apu_i().output()
    }

    fn audio_buffer(&self) -> &VecDeque<u8> {
        self.apu_i().audio_buffer()
    }

    fn clear_audio_buffer(&mut self) {
        self.apu().clear_audio_buffer()
    }
}

impl Default for GameBoy {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Display for GameBoy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description(9))
    }
}
