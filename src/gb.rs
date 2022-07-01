use crate::{cpu::Cpu, mmu::Mmu, ppu::Ppu, util::read_file};

pub struct GameBoy {
    cpu: Cpu,
}

impl GameBoy {
    pub fn new() -> GameBoy {
        let ppu = Ppu::new();
        let mmu = Mmu::new(ppu);
        let cpu = Cpu::new(mmu);
        GameBoy { cpu: cpu }
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

    pub fn cpu(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn mmu(&mut self) -> &mut Mmu {
        self.cpu.mmu()
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        self.cpu.ppu()
    }

    pub fn load_boot(&mut self, path: &str) {
        let data = read_file(path);
        self.cpu.mmu().write_boot(0x0000, &data);
    }

    pub fn load_boot_default(&mut self) {
        self.load_boot("./res/dmg_rom.bin");
    }
}
