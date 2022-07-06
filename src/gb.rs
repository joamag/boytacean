use crate::{
    cpu::Cpu,
    data::{DMG_BOOT, DMG_BOOTIX, MGB_BOOTIX, SGB_BOOT},
    mmu::Mmu,
    pad::Pad,
    ppu::{Ppu, Tile, FRAME_BUFFER_SIZE},
    util::read_file,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Top level structure that abstracts the usage of the
/// Game Boy system under the Boytacean emulator.
/// Should serve as the main entry-point API.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameBoy {
    cpu: Cpu,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoy {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        let pad = Pad::new();
        let ppu = Ppu::new();
        let mmu = Mmu::new(ppu, pad);
        let cpu = Cpu::new(mmu);
        Self { cpu: cpu }
    }

    pub fn reset(&mut self) {
        self.ppu().reset();
        self.mmu().reset();
        self.cpu.reset();
    }

    pub fn pc(&self) -> u16 {
        self.cpu.pc()
    }

    pub fn clock(&mut self) -> u8 {
        let cycles = self.cpu_clock();
        self.ppu_clock(cycles);
        cycles
    }

    pub fn cpu_clock(&mut self) -> u8 {
        self.cpu.clock()
    }

    pub fn ppu_clock(&mut self, cycles: u8) {
        self.ppu().clock(cycles)
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        self.cpu.mmu().write_rom(0x0000, data);
    }

    pub fn load_rom_file(&mut self, path: &str) {
        let data = read_file(path);
        self.load_rom(&data);
    }

    pub fn load_boot(&mut self, data: &[u8]) {
        self.cpu.mmu().write_boot(0x0000, data);
    }

    pub fn load_boot_file(&mut self, path: &str) {
        let data = read_file(path);
        self.load_boot(&data);
    }

    pub fn load_boot_dmg_f(&mut self) {
        self.load_boot_file("./res/boot/dmg_boot.bin");
    }

    pub fn load_boot_sgb_f(&mut self) {
        self.load_boot_file("./res/boot/sgb_boot.bin");
    }

    pub fn load_boot_dmg_bootix_f(&mut self) {
        self.load_boot_file("./res/boot/dmg_bootix.bin");
    }

    pub fn load_boot_mgb_bootix_f(&mut self) {
        self.load_boot_file("./res/boot/mgb_bootix.bin");
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot_dmg_bootix_f();
    }

    pub fn load_boot_dmg(&mut self) {
        self.load_boot(&DMG_BOOT);
    }

    pub fn load_boot_sgb(&mut self) {
        self.load_boot(&SGB_BOOT);
    }

    pub fn load_boot_dmg_bootix(&mut self) {
        self.load_boot(&DMG_BOOTIX);
    }

    pub fn load_boot_mgb_bootix(&mut self) {
        self.load_boot(&MGB_BOOTIX);
    }

    pub fn load_boot_static(&mut self) {
        self.load_boot_dmg_bootix();
    }

    pub fn frame_buffer_eager(&mut self) -> Vec<u8> {
        self.frame_buffer().to_vec()
    }

    pub fn get_tile(&mut self, index: usize) -> Tile {
        self.ppu().tiles()[index]
    }

    pub fn get_tile_buffer(&mut self, index: usize) -> Vec<u8> {
        let tile = self.get_tile(index);
        tile.palette_buffer(self.ppu().palette())
    }
}

impl GameBoy {
    pub fn cpu(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn mmu(&mut self) -> &mut Mmu {
        self.cpu.mmu()
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        self.cpu.ppu()
    }

    pub fn frame_buffer(&mut self) -> &Box<[u8; FRAME_BUFFER_SIZE]> {
        &(self.ppu().frame_buffer)
    }
}
