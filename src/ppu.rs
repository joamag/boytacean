pub const VRAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;

pub struct Ppu {
    pub vram: [u8; VRAM_SIZE],
    pub hram: [u8; HRAM_SIZE],
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            vram: [0u8; VRAM_SIZE],
            hram: [0u8; HRAM_SIZE],
        }
    }
}
