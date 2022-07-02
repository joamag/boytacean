pub const VRAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;
pub const PALETTE_SIZE: usize = 4;
pub const RGB_SIZE: usize = 3;

/// The number of tiles that can be store in Game Boy's
/// VRAM memory according to specifications.
pub const TILE_COUNT: usize = 384;

/// The width of the Game Boy screen in pixels.
pub const DISPLAY_WIDTH: usize = 160;

/// The height of the Game Boy screen in pixels.
pub const DISPLAY_HEIGHT: usize = 154;

// The size of the RGB frame buffer in bytes.
pub const FRAME_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * RGB_SIZE;

/// Represents the Game Boy PPU (Pixel Processing Unit) and controls
/// all of the logic behind the graphics processing and presentation.
/// Should store both the VRAM and HRAM together with the internal
/// graphic related registers.
/// Outputs the screen as a RGB 8 bit frame buffer.
///
/// # Basic usage
/// ```rust
/// let ppu = Ppu::new();
/// ppu.tick();
/// ```
pub struct Ppu {
    /// The 8 bit based RGB frame buffer with the
    /// processed set of pixels ready to be displayed on screen.
    pub frame_buffer: Box<[u8; FRAME_BUFFER_SIZE]>,
    /// Video dedicated memory (VRAM) where both the tiles and
    /// the sprites are going to be stored.
    pub vram: [u8; VRAM_SIZE],
    /// High RAM memory that should provide extra speed for regular
    /// operations.
    pub hram: [u8; HRAM_SIZE],
    /// The current set of processed tiles that are store in the
    /// PPU related structures.
    tiles: [[[u8; 8]; 8]; TILE_COUNT],
    /// The palette of colors that is currently loaded in Game Boy.
    palette: [[u8; RGB_SIZE]; PALETTE_SIZE],
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
    /// Controls if the background is going to be drawn to screen.
    switch_bg: bool,
    /// Controls if the sprites are going to be drawn to screen.
    switch_sprites: bool,
    /// Controls the map that is going to be drawn to screen, the
    /// offset in VRAM will be adjusted according to this.
    bg_map: bool,
    /// If the background tile set is active meaning that the
    /// negative based indexes are going to be used.
    bg_tile: bool,
    /// Flag that controls if the LCD screen is ON and displaying
    /// content.
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
            frame_buffer: Box::new([0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT * RGB_SIZE]),
            vram: [0u8; VRAM_SIZE],
            hram: [0u8; HRAM_SIZE],
            tiles: [[[0u8; 8]; 8]; TILE_COUNT],
            palette: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            scy: 0x0,
            scx: 0x0,
            line: 0x0,
            switch_bg: false,
            switch_sprites: false,
            bg_map: false,
            bg_tile: false,
            switch_lcd: false,
            mode: PpuMode::OamRead,
            mode_clock: 0,
        }
    }

    pub fn clock(&mut self, cycles: u8) {
        self.mode_clock += cycles as u16;
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
            PpuMode::Hblank => {
                if self.mode_clock >= 204 {
                    self.line += 1;

                    // in case we've reached the end of the
                    // screen we're now entering the v-blank
                    if self.line == 143 {
                        self.mode = PpuMode::VBlank;
                        // self.drawData
                        // @todo implement this one
                    } else {
                        self.mode = PpuMode::OamRead;
                    }

                    self.mode_clock = 0;
                }
            }
            PpuMode::VBlank => {
                if self.mode_clock >= 456 {
                    self.line += 1;

                    // in case the end of v-blank has been reached then
                    // we must jump again to the OAM read mode and reset
                    // the scan line counter to the zero value
                    if self.line == 153 {
                        self.mode = PpuMode::OamRead;
                        self.line = 0;
                    }

                    self.mode_clock = 0;
                }
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x0040 => {
                let value = if self.switch_bg { 0x01 } else { 0x00 }
                    | if self.switch_sprites { 0x02 } else { 0x00 }
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
                self.switch_sprites = value & 0x02 == 0x02;
                self.bg_map = value & 0x08 == 0x08;
                self.bg_tile = value & 0x10 == 0x10;
                self.switch_lcd = value & 0x80 == 0x80;
            }
            0x0042 => self.scy = value,
            0x0043 => self.scx = value,
            0x0047 => {
                for index in 0..PALETTE_SIZE {
                    match (value >> (index * 2)) & 3 {
                        0 => self.palette[index] = [255, 255, 255],
                        1 => self.palette[index] = [192, 192, 192],
                        2 => self.palette[index] = [96, 96, 96],
                        3 => self.palette[index] = [0, 0, 0],
                        color_index => panic!("Invalid palette color index {:04x}", color_index),
                    }
                }
            }
            addr => panic!("Writing in unknown PPU location 0x{:04x}", addr),
        }
    }

    /// Updates the tile structure with the value that has
    /// just been written to a location on the VRAM associated
    /// with tiles.
    pub fn update_tile(&mut self, addr: u16, _value: u8) {
        let addr = (addr & 0x1ffe) as usize;
        let tile_index = ((addr >> 4) & 0x01ff) as usize;
        let y = ((addr >> 1) & 0x0007) as usize;

        let mut mask;

        for x in 0..8 {
            mask = 1 << (7 - x);
            self.tiles[tile_index][y][x] = if self.vram[addr] & mask > 0 { 0x1 } else { 0x0 }
                | if self.vram[addr + 1] & mask > 0 {
                    0x2
                } else {
                    0x0
                }
        }
    }

    fn render_line(&mut self) {
        let mut map_offset: usize = if self.bg_map { 0x1c00 } else { 0x1800 };
        map_offset += (((self.line + self.scy) & 0xff) >> 3) as usize;

        // calculates the sprite line offset by using the SCX register
        // shifted by 3 meaning as the tiles are 8x8
        let mut line_offset: usize = (self.scx >> 3) as usize;

        // calculates both the current Y and X positions within the tiles
        let y = ((self.scy + self.line) & 0x07) as usize;
        let mut x = (self.scx & 0x07) as usize;

        // calculates the index of the initial tile in drawing,
        // if the tile data set in use is #1, the indices are
        // signed, then calculates a real tile offset
        let mut tile_index = self.vram[map_offset + line_offset] as usize;
        if self.bg_tile && tile_index < 128 {
            tile_index += 256;
        }

        // calculates the frame buffer offset position assuming the proper
        // Game Boy screen width and RGB pixel (3 bytes) size
        let mut frame_offset = self.line as usize * DISPLAY_WIDTH * RGB_SIZE;

        for _index in 0..DISPLAY_WIDTH {
            // obtains the current pixel data from the tile and
            // re-maps it according to the current palette
            let pixel = self.tiles[tile_index][y][x];
            let color = self.palette[pixel as usize];

            // set the color pixel in the frame buffer
            self.frame_buffer[frame_offset] = color[0];
            self.frame_buffer[frame_offset + 1] = color[1];
            self.frame_buffer[frame_offset + 2] = color[2];

            // increments the offset of the frame buffer by the
            // size of an RGB pixel (which is 3 bytes)
            frame_offset += RGB_SIZE;

            // increments the current tile X position in drawing
            x += 1;

            // in case the end of tile width has been reached then
            // a new tile must be retrieved for plotting
            if x == 8 {
                // resets the tile X position to the base value
                // as a new tile is going to be drawn
                x = 0;

                // calculates the new line tile offset making sure that
                // the maximum of 32 is not overflown
                line_offset = (line_offset + 1) & 31;

                // calculates the tile index nad makes sure the value
                // takes into consideration the bg tile value
                tile_index = self.vram[map_offset + line_offset] as usize;
                if self.bg_tile && tile_index < 128 {
                    tile_index += 256;
                }
            }
        }
    }

    /// Prints the tile data information to the stdout, this is
    /// useful for debugging purposes.
    pub fn draw_tile_stdout(&self, tile_index: usize) {
        for y in 0..8 {
            for x in 0..8 {
                print!("{}", self.tiles[tile_index][y as usize][x as usize]);
            }
            print!("\n");
        }
    }
}
