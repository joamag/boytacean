pub const VRAM_SIZE: usize = 8192;

pub struct Ppu {
    vram: [u8; VRAM_SIZE],
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            vram: [0u8; VRAM_SIZE],
        }
    }
}
