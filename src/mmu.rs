use crate::ppu::Ppu;

pub const RAM_SIZE: usize = 8192;

pub struct Mmu {
    ppu: Ppu,
    ram: [u8; RAM_SIZE],
}

impl Mmu {
    pub fn new(ppu: Ppu) -> Mmu {
        Mmu {
            ppu: ppu,
            ram: [0u8; RAM_SIZE],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.ram[addr as usize] = value;
    }

    pub fn write_buffer(&mut self, addr: u16, buffer: &[u8]) {
        self.ram[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }
}
