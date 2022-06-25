use std::{cell::RefCell, rc::Rc};

use crate::{cpu::Cpu, mmu::Mmu, util::read_file};

pub type SharedMut<T> = Rc<RefCell<T>>;

pub struct GameBoy {
    cpu: Cpu,
}

impl GameBoy {
    pub fn new() -> GameBoy {
        let mmu = Mmu::new();
        let cpu = Cpu::new(mmu);
        GameBoy { cpu: cpu }
    }

    pub fn load_boot(&mut self, path: &str) {
        let data = read_file(path);
        self.cpu.mmu().write_buffer(0x0000, &data);
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot("./res/mbr_rom.bin");
    }
}
