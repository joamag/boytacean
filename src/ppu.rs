pub const VRAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;
pub const PALETTE_SIZE: usize = 4;
pub const RGBA_SIZE: usize = 4;

pub struct Ppu {
    pub vram: [u8; VRAM_SIZE],
    pub hram: [u8; HRAM_SIZE],
    pub palette: [[u8; RGBA_SIZE]; PALETTE_SIZE],
    pub scy: u8,
    pub scx: u8,
    pub line: u8,
    pub switch_bg: bool,
    pub bg_map: bool,
    pub bg_tile: bool,
    pub switch_lcd: bool,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            vram: [0u8; VRAM_SIZE],
            hram: [0u8; HRAM_SIZE],
            palette: [[0u8; RGBA_SIZE]; PALETTE_SIZE],
            scy: 0x0,
            scx: 0x0,
            line: 0x0,
            switch_bg: false,
            bg_map: false,
            bg_tile: false,
            switch_lcd: false,
        }
    }

    pub fn clock() {}

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x0040 => {
                let value = if self.switch_bg { 0x01 } else { 0x00 }
                    | if self.bg_map { 0x08 } else { 0x00 }
                    | if self.bg_tile { 0x10 } else { 0x00 }
                    | if self.switch_lcd { 0x80 } else { 0x00 };
                value
            }
            0x0042 => self.scy,
            0x0043 => self.scx,
            0x0044 => self.line,
            addr => panic!("Reading from unknown PPU location 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0x00ff {
            0x0040 => {
                self.switch_bg = value & 0x01 == 0x01;
                self.bg_map = value & 0x08 == 0x08;
                self.bg_tile = value & 0x10 == 0x10;
                self.switch_lcd = value & 0x80 == 0x80;
            }
            0x0042 => self.scy = value,
            0x0043 => self.scx = value,
            0x0047 => {
                for index in 0..PALETTE_SIZE {
                    match (value >> (index * 2)) & 3 {
                        0 => self.palette[index] = [255, 255, 255, 255],
                        1 => self.palette[index] = [192, 192, 192, 255],
                        2 => self.palette[index] = [96, 96, 96, 255],
                        3 => self.palette[index] = [0, 0, 0, 255],
                        color_index => panic!("Invalid palette color index {:04x}", color_index),
                    }
                }
            }
            addr => panic!("Writing in unknown PPU location 0x{:04x}", addr),
        }
    }
}
