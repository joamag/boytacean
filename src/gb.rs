use crate::{
    apu::Apu,
    cpu::Cpu,
    data::{BootRom, CGB_BOOT, DMG_BOOT, DMG_BOOTIX, MGB_BOOTIX, SGB_BOOT},
    gen::{COMPILATION_DATE, COMPILATION_TIME, COMPILER, COMPILER_VERSION},
    mmu::Mmu,
    pad::{Pad, PadKey},
    ppu::{Ppu, PpuMode, Tile, FRAME_BUFFER_SIZE},
    rom::Cartridge,
    timer::Timer,
    util::read_file,
};

use std::collections::VecDeque;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{
    gen::dependencies_map,
    ppu::{Palette, Pixel},
};

#[cfg(feature = "wasm")]
use std::{
    convert::TryInto,
    panic::{set_hook, take_hook, PanicInfo},
};

/// Top level structure that abstracts the usage of the
/// Game Boy system under the Boytacean emulator.
/// Should serve as the main entry-point API.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameBoy {
    cpu: Cpu,
    ppu_enabled: bool,
    apu_enabled: bool,
    timer_enabled: bool,
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
impl GameBoy {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        let ppu = Ppu::default();
        let apu = Apu::default();
        let pad = Pad::default();
        let timer = Timer::default();
        let mmu = Mmu::new(ppu, apu, pad, timer);
        let cpu = Cpu::new(mmu);
        Self {
            cpu,
            ppu_enabled: true,
            apu_enabled: false,
            timer_enabled: true,
        }
    }

    pub fn reset(&mut self) {
        self.ppu().reset();
        self.mmu().reset();
        self.cpu.reset();
    }

    pub fn clock(&mut self) -> u8 {
        let cycles = self.cpu_clock();
        if self.ppu_enabled {
            self.ppu_clock(cycles);
        }
        if self.apu_enabled {
            self.apu_clock(cycles);
        }
        if self.timer_enabled {
            self.timer_clock(cycles);
        }
        cycles
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

    pub fn ppu_clock(&mut self, cycles: u8) {
        self.ppu().clock(cycles)
    }

    pub fn apu_clock(&mut self, cycles: u8) {
        self.apu().clock(cycles)
    }

    pub fn timer_clock(&mut self, cycles: u8) {
        self.timer().clock(cycles)
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

    pub fn boot(&mut self) {
        self.cpu.boot();
    }

    pub fn load_boot(&mut self, data: &[u8]) {
        self.cpu.mmu().write_boot(0x0000, data);
    }

    pub fn load_boot_static(&mut self, boot_rom: BootRom) {
        match boot_rom {
            BootRom::Dmg => self.load_boot(&DMG_BOOT),
            BootRom::Sgb => self.load_boot(&SGB_BOOT),
            BootRom::DmgBootix => self.load_boot(&DMG_BOOTIX),
            BootRom::MgbBootix => self.load_boot(&MGB_BOOTIX),
            BootRom::Cgb => self.load_boot(&CGB_BOOT),
        }
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot_dmg();
    }

    pub fn load_boot_dmg(&mut self) {
        self.load_boot_static(BootRom::DmgBootix);
    }

    pub fn load_boot_cgb(&mut self) {
        self.load_boot_static(BootRom::Cgb);
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

    pub fn audio_buffer_eager(&mut self, clear: bool) -> Vec<u8> {
        let buffer = Vec::from(self.audio_buffer().clone());
        if clear {
            self.clear_audio_buffer();
        }
        buffer
    }

    pub fn cartridge_eager(&mut self) -> Cartridge {
        self.mmu().rom().clone()
    }

    pub fn ram_data_eager(&mut self) -> Vec<u8> {
        self.mmu().rom().ram_data_eager()
    }

    pub fn set_ram_data(&mut self, ram_data: Vec<u8>) {
        self.mmu().rom().set_ram_data(ram_data)
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
    /// using the currently loaded palette.
    pub fn get_tile_buffer(&mut self, index: usize) -> Vec<u8> {
        let tile = self.get_tile(index);
        tile.palette_buffer(self.ppu().palette())
    }

    /// Obtains the name of the compiler that has been
    /// used in the compilation of the base Boytacean
    /// library. Can be used for diagnostics.
    pub fn get_compiler(&self) -> String {
        String::from(COMPILER)
    }

    pub fn get_compiler_version(&self) -> String {
        String::from(COMPILER_VERSION)
    }

    pub fn get_compilation_date(&self) -> String {
        String::from(COMPILATION_DATE)
    }

    pub fn get_compilation_time(&self) -> String {
        String::from(COMPILATION_TIME)
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

    pub fn mmu(&mut self) -> &mut Mmu {
        self.cpu.mmu()
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        self.cpu.ppu()
    }

    pub fn apu(&mut self) -> &mut Apu {
        self.cpu.apu()
    }

    pub fn apu_i(&self) -> &Apu {
        self.cpu.apu_i()
    }

    pub fn pad(&mut self) -> &mut Pad {
        self.cpu.pad()
    }

    pub fn timer(&mut self) -> &mut Timer {
        self.cpu.timer()
    }

    pub fn frame_buffer(&mut self) -> &[u8; FRAME_BUFFER_SIZE] {
        &(self.ppu().frame_buffer)
    }

    pub fn audio_buffer(&mut self) -> &VecDeque<u8> {
        self.apu().audio_buffer()
    }

    pub fn load_boot_path(&mut self, path: &str) {
        let data = read_file(path);
        self.load_boot(&data);
    }

    pub fn load_boot_file(&mut self, boot_rom: BootRom) {
        match boot_rom {
            BootRom::Dmg => self.load_boot_path("./res/boot/dmg_boot.bin"),
            BootRom::Sgb => self.load_boot_path("./res/boot/sgb_boot.bin"),
            BootRom::DmgBootix => self.load_boot_path("./res/boot/dmg_bootix.bin"),
            BootRom::MgbBootix => self.load_boot_path("./res/boot/mgb_bootix.bin"),
            BootRom::Cgb => self.load_boot_path("./res/boot/cgb_boot.bin"),
        }
    }

    pub fn load_boot_default_f(&mut self) {
        self.load_boot_dmg_f();
    }

    pub fn load_boot_dmg_f(&mut self) {
        self.load_boot_file(BootRom::DmgBootix);
    }

    pub fn load_boot_cgb_f(&mut self) {
        self.load_boot_file(BootRom::Cgb);
    }

    pub fn load_rom(&mut self, data: &[u8]) -> &Cartridge {
        let rom = Cartridge::from_data(data);
        self.mmu().set_rom(rom);
        self.mmu().rom()
    }

    pub fn load_rom_file(&mut self, path: &str) -> &Cartridge {
        let data = read_file(path);
        self.load_rom(&data)
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoy {
    pub fn set_panic_hook_ws() {
        let prev = take_hook();
        set_hook(Box::new(move |info| {
            hook_impl(info);
            prev(info);
        }));
    }

    pub fn load_rom_ws(&mut self, data: &[u8]) -> Cartridge {
        self.load_rom(data).clone()
    }

    pub fn set_palette_colors_ws(&mut self, value: Vec<JsValue>) {
        let palette: Palette = value
            .into_iter()
            .map(|v| Self::js_to_pixel(&v))
            .collect::<Vec<Pixel>>()
            .try_into()
            .unwrap();
        self.ppu().set_palette_colors(&palette);
    }

    pub fn get_wasm_engine_ws(&self) -> Option<String> {
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
        Self::new()
    }
}
