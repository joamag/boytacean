use crate::{
    cpu::Cpu,
    mmu::Mmu,
    ppu::{Ppu, FRAME_BUFFER_SIZE},
    util::read_file,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Static data corresponding to the DMG boot ROM
/// allows freely using the emulator without external dependency.
pub const BOOT_DATA: [u8; 256] = [
    49, 254, 255, 175, 33, 255, 159, 50, 203, 124, 32, 251, 33, 38, 255, 14, 17, 62, 128, 50, 226,
    12, 62, 243, 226, 50, 62, 119, 119, 62, 252, 224, 71, 17, 4, 1, 33, 16, 128, 26, 205, 149, 0,
    205, 150, 0, 19, 123, 254, 52, 32, 243, 17, 216, 0, 6, 8, 26, 19, 34, 35, 5, 32, 249, 62, 25,
    234, 16, 153, 33, 47, 153, 14, 12, 61, 40, 8, 50, 13, 32, 249, 46, 15, 24, 243, 103, 62, 100,
    87, 224, 66, 62, 145, 224, 64, 4, 30, 2, 14, 12, 240, 68, 254, 144, 32, 250, 13, 32, 247, 29,
    32, 242, 14, 19, 36, 124, 30, 131, 254, 98, 40, 6, 30, 193, 254, 100, 32, 6, 123, 226, 12, 62,
    135, 226, 240, 66, 144, 224, 66, 21, 32, 210, 5, 32, 79, 22, 32, 24, 203, 79, 6, 4, 197, 203,
    17, 23, 193, 203, 17, 23, 5, 32, 245, 34, 35, 34, 35, 201, 206, 237, 102, 102, 204, 13, 0, 11,
    3, 115, 0, 131, 0, 12, 0, 13, 0, 8, 17, 31, 136, 137, 0, 14, 220, 204, 110, 230, 221, 221, 217,
    153, 187, 187, 103, 99, 110, 14, 236, 204, 221, 220, 153, 159, 187, 185, 51, 62, 60, 66, 185,
    165, 185, 165, 66, 60, 33, 4, 1, 17, 168, 0, 26, 19, 190, 32, 254, 35, 125, 254, 52, 32, 245,
    6, 25, 120, 134, 35, 5, 32, 251, 134, 32, 254, 62, 1, 224, 80,
];

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
        self.load_boot(&data);
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot_file("./res/dmg_rom.bin");
    }

    pub fn load_boot_static(&mut self) {
        self.load_boot(&BOOT_DATA);
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
