use crate::cpu::Cpu;

pub const VRAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;
pub const PALETTE_SIZE: usize = 4;
pub const RGBA_SIZE: usize = 4;
pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 154;

/// Represents the Game Boy PPU (Pixel Processing Unit) and controls
/// all of the logic behind the graphics processing and presentation.
/// Should store both the VRAM and HRAM together with the internal
/// graphic related registers.
/// Outputs the screen as a monochromatic 8 bit frame buffer.
///
/// # Basic usage
/// ```rust
/// let ppu = Ppu::new();
/// ppu.tick();
/// ```
pub struct Ppu {
    /// The 8 bit based monochromatic frame buffer with the
    /// processed set of pixels ready to be displayed on screen.
    pub frame_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    /// Video dedicated memory (VRAM) where both the tiles and
    /// the sprites are going to be stored.
    pub vram: [u8; VRAM_SIZE],
    pub hram: [u8; HRAM_SIZE],
    palette: [[u8; RGBA_SIZE]; PALETTE_SIZE],
    /// The scroll Y register that controls the Y offset
    /// of the background.
    scy: u8,
    /// The scroll X register that controls the X offset
    /// of the background.
    scx: u8,
    /// The current scan line in processing, should
    /// range between 0 (0x00) and 153 (0x99), representing
    /// the 154 lines plus 10 extra v-blank lines.
    line: u8,
    switch_bg: bool,
    bg_map: bool,
    bg_tile: bool,
    switch_lcd: bool,
    /// The current execution mode of the PPU, should change
    /// between states over the drawing of a frame.
    mode: PpuMode,
    /// Internal clock counter used to control the time in ticks
    /// spent in each of the PPU modes.
    mode_clock: u16,
}

pub enum PpuMode {
    OamRead,
    VramRead,
    Hblank,
    VBlank,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            frame_buffer: [0u8; SCREEN_WIDTH * SCREEN_HEIGHT],
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
            mode: PpuMode::OamRead,
            mode_clock: 0,
        }
    }

    pub fn clock(&mut self, ticks: u8) {
        self.mode_clock += ticks as u16;
        match self.mode {
            PpuMode::OamRead => {
                if self.mode_clock >= 204 {
                    self.mode_clock = 0;
                    self.mode = PpuMode::VramRead;
                }
            }
            PpuMode::VramRead => {
                if self.mode_clock >= 172 {
                    self.render_line();

                    self.mode_clock = 0;
                    self.mode = PpuMode::Hblank;
                }
            }
            PpuMode::Hblank => todo!(),
            PpuMode::VBlank => todo!(),
        }
    }

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

    fn render_line(&self) {
        //@todo implement the rendering of a line
    }
}
