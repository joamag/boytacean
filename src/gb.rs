use crate::{
    cpu::Cpu,
    data::{SGB_BOOT, DMG_BOOT},
    mmu::Mmu,
    ppu::{Ppu, FRAME_BUFFER_SIZE},
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
    pub fn new() -> GameBoy {
        let ppu = Ppu::new();
        let mmu = Mmu::new(ppu);
        let cpu = Cpu::new(mmu);
        GameBoy { cpu: cpu }
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
        println!("{:?}", data);
        self.load_boot(&data);
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot_file("./res/boot/dmg_boot.bin");
    }

    pub fn load_boot_dmg(&mut self) {
        self.load_boot(&DMG_BOOT);
    }

    pub fn load_boot_sgb(&mut self) {
        self.load_boot(&SGB_BOOT);
    }

    pub fn frame_buffer_eager(&mut self) -> Vec<u8> {
        self.frame_buffer().to_vec()
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
