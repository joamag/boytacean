pub const RAM_SIZE: usize = 8192;
pub const VRAM_SIZE: usize = 8192;

pub struct Mmu {
    ram: [u8; RAM_SIZE],
    vram: [u8; VRAM_SIZE],
}

impl Mmu {
    pub fn new() -> Mmu {
        Mmu {
            ram: [0u8; RAM_SIZE],
            vram: [0u8; VRAM_SIZE],
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
