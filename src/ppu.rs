use core::fmt;
use std::{
    borrow::BorrowMut,
    fmt::{Display, Formatter},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::warnln;

pub const VRAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;
pub const OAM_SIZE: usize = 260;
pub const PALETTE_SIZE: usize = 4;
pub const RGB_SIZE: usize = 3;
pub const TILE_WIDTH: usize = 8;
pub const TILE_HEIGHT: usize = 8;

/// The number of tiles that can be store in Game Boy's
/// VRAM memory according to specifications.
pub const TILE_COUNT: usize = 384;

/// The number of objects/sprites that can be handled at
/// the same time by the Game Boy.
pub const OBJ_COUNT: usize = 40;

/// The width of the Game Boy screen in pixels.
pub const DISPLAY_WIDTH: usize = 160;

/// The height of the Game Boy screen in pixels.
pub const DISPLAY_HEIGHT: usize = 144;

/// The size to be used by the buffer of colors
/// for the Game Boy screen the values there should
/// range from 0 to 3.
pub const COLOR_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

/// The size of the RGB frame buffer in bytes.
pub const FRAME_BUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * RGB_SIZE;

/// Defines the Game Boy pixel type as a buffer
/// with the size of RGB (3 bytes).
pub type Pixel = [u8; RGB_SIZE];

/// Defines a type that represents a color palette
/// within the Game Boy context.
pub type Palette = [Pixel; PALETTE_SIZE];

/// Represents a tile within the Game Boy context,
/// should contain the pixel buffer of the tile.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq)]
pub struct Tile {
    buffer: [u8; 64],
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Tile {
    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.buffer[y * TILE_WIDTH + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        self.buffer[y * TILE_WIDTH + x] = value;
    }

    pub fn buffer(&self) -> Vec<u8> {
        self.buffer.to_vec()
    }
}

impl Tile {
    pub fn get_row(&self, y: usize) -> &[u8] {
        &self.buffer[y * TILE_WIDTH..(y + 1) * TILE_WIDTH]
    }
}

impl Tile {
    pub fn palette_buffer(&self, palette: Palette) -> Vec<u8> {
        self.buffer
            .iter()
            .flat_map(|p| palette[*p as usize])
            .collect()
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        for y in 0..8 {
            for x in 0..8 {
                buffer.push_str(format!("{}", self.get(x, y)).as_str());
            }
            buffer.push_str("\n");
        }
        write!(f, "{}", buffer)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq)]
pub struct ObjectData {
    x: i16,
    y: i16,
    tile: u8,
    palette: u8,
    xflip: bool,
    yflip: bool,
    priority: bool,
    index: u8,
}

impl Display for ObjectData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Index => {}\nX => {}\nY => {}\nTile => {}",
            self.index, self.x, self.y, self.tile
        )
    }
}

/// Represents the Game Boy PPU (Pixel Processing Unit) and controls
/// all of the logic behind the graphics processing and presentation.
/// Should store both the VRAM and HRAM together with the internal
/// graphic related registers.
/// Outputs the screen as a RGB 8 bit frame buffer.
///
/// # Basic usage
/// ```rust
/// let ppu = Ppu::new();
/// ppu.clock();
/// ```
pub struct Ppu {
    /// The color buffer that is going to store the colors
    /// (from 0 to 3) for all the pixels in the screen.
    pub color_buffer: Box<[u8; COLOR_BUFFER_SIZE]>,

    /// The 8 bit based RGB frame buffer with the
    /// processed set of pixels ready to be displayed on screen.
    pub frame_buffer: Box<[u8; FRAME_BUFFER_SIZE]>,

    /// Video dedicated memory (VRAM) where both the tiles and
    /// the sprites/objects are going to be stored.
    pub vram: [u8; VRAM_SIZE],

    /// High RAM memory that should provide extra speed for regular
    /// operations.
    pub hram: [u8; HRAM_SIZE],

    /// OAM RAM (Sprite Attribute Table ) used for the storage of the
    /// sprite attributes for each of the 40 sprites of the Game Boy.
    pub oam: [u8; OAM_SIZE],

    /// The current set of processed tiles that are store in the
    /// PPU related structures.
    tiles: [Tile; TILE_COUNT],

    /// The meta information about the sprites/objects that are going
    /// to be drawn to the screen,
    obj_data: [ObjectData; OBJ_COUNT],

    /// The palette of colors that is currently loaded in Game Boy
    /// and used for background (tiles).
    palette: Palette,

    // The palette that is going to be used for sprites/objects #0.
    palette_obj_0: Palette,

    // The palette that is going to be used for sprites/objects #1.
    palette_obj_1: Palette,

    /// The scroll Y register that controls the Y offset
    /// of the background.
    scy: u8,

    /// The scroll X register that controls the X offset
    /// of the background.
    scx: u8,

    /// The top most Y coordinate of the window,
    /// going to be used while drawing the window.
    wy: u8,

    /// The top most X coordinate of the window plus 7,
    /// going to be used while drawing the window.
    wx: u8,

    /// The current scan line in processing, should
    /// range between 0 (0x00) and 153 (0x99), representing
    /// the 154 lines plus 10 extra v-blank lines.
    ly: u8,

    // The line compare register that is going to be used
    // in the STATE and associated interrupts.
    lyc: u8,

    /// The current execution mode of the PPU, should change
    /// between states over the drawing of a frame.
    mode: PpuMode,

    /// Internal clock counter used to control the time in ticks
    /// spent in each of the PPU modes.
    mode_clock: u16,

    /// Controls if the background is going to be drawn to screen.
    switch_bg: bool,

    /// Controls if the sprites/objects are going to be drawn to screen.
    switch_obj: bool,

    /// Defines the size in pixels of the object (false=8x8, true=8x16).
    obj_size: bool,

    /// Controls the map that is going to be drawn to screen, the
    /// offset in VRAM will be adjusted according to this
    /// (false=0x9800, true=0x9c000).
    bg_map: bool,

    /// If the background tile set is active meaning that the
    /// negative based indexes are going to be used.
    bg_tile: bool,

    // Controls if the window is meant to be drawn.
    switch_window: bool,

    // Controls the offset of the map that is going to be drawn
    // for the window section of the screen.
    window_map: bool,

    /// Flag that controls if the LCD screen is ON and displaying
    /// content.
    switch_lcd: bool,

    stat_hblank: bool,
    stat_vblank: bool,
    stat_oam: bool,
    stat_lyc: bool,

    // Boolean value set when the V-Blank interrupt should be handled
    // by the next CPU clock operation.
    int_vblank: bool,

    // Boolean value when the LCD STAT interrupt should be handled by
    // the next CPU clock operation.
    int_stat: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    OamRead = 2,
    VramRead = 3,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            color_buffer: Box::new([0u8; COLOR_BUFFER_SIZE]),
            frame_buffer: Box::new([0u8; FRAME_BUFFER_SIZE]),
            vram: [0u8; VRAM_SIZE],
            hram: [0u8; HRAM_SIZE],
            oam: [0u8; OAM_SIZE],
            tiles: [Tile { buffer: [0u8; 64] }; TILE_COUNT],
            obj_data: [ObjectData {
                x: 0,
                y: 0,
                tile: 0,
                palette: 0,
                xflip: false,
                yflip: false,
                priority: false,
                index: 0,
            }; OBJ_COUNT],
            palette: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            palette_obj_0: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            palette_obj_1: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            scy: 0x0,
            scx: 0x0,
            wy: 0x0,
            wx: 0x0,
            ly: 0x0,
            lyc: 0x0,
            mode: PpuMode::OamRead,
            mode_clock: 0,
            switch_bg: false,
            switch_obj: false,
            obj_size: false,
            bg_map: false,
            bg_tile: false,
            switch_window: false,
            window_map: false,
            switch_lcd: false,
            stat_hblank: false,
            stat_vblank: false,
            stat_oam: false,
            stat_lyc: false,
            int_vblank: false,
            int_stat: false,
        }
    }

    pub fn reset(&mut self) {
        self.color_buffer = Box::new([0u8; COLOR_BUFFER_SIZE]);
        self.frame_buffer = Box::new([0u8; FRAME_BUFFER_SIZE]);
        self.vram = [0u8; VRAM_SIZE];
        self.hram = [0u8; HRAM_SIZE];
        self.tiles = [Tile { buffer: [0u8; 64] }; TILE_COUNT];
        self.palette = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.palette_obj_0 = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.palette_obj_1 = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.scy = 0x0;
        self.scx = 0x0;
        self.ly = 0x0;
        self.lyc = 0x0;
        self.mode = PpuMode::OamRead;
        self.mode_clock = 0;
        self.switch_bg = false;
        self.switch_obj = false;
        self.obj_size = false;
        self.bg_map = false;
        self.bg_tile = false;
        self.switch_window = false;
        self.window_map = false;
        self.switch_lcd = false;
        self.stat_hblank = false;
        self.stat_vblank = false;
        self.stat_oam = false;
        self.stat_lyc = false;
        self.int_vblank = false;
        self.int_stat = false;
    }

    pub fn clock(&mut self, cycles: u8) {
        // increments the current mode clock by the provided amount
        // of CPU cycles (probably coming from a previous CPU clock)
        self.mode_clock += cycles as u16;

        match self.mode {
            PpuMode::OamRead => {
                if self.mode_clock >= 80 {
                    self.mode = PpuMode::VramRead;
                    self.mode_clock -= 80;
                    self.update_stat()
                }
            }
            PpuMode::VramRead => {
                if self.mode_clock >= 172 {
                    self.render_line();

                    self.mode = PpuMode::HBlank;
                    self.mode_clock -= 172;
                    self.update_stat()
                }
            }
            PpuMode::HBlank => {
                if self.mode_clock >= 204 {
                    // increments the register that holds the
                    // information about the current line in drawing
                    self.ly += 1;

                    // in case we've reached the end of the
                    // screen we're now entering the v-blank
                    if self.ly == 144 {
                        self.int_vblank = true;
                        self.mode = PpuMode::VBlank;
                    } else {
                        self.mode = PpuMode::OamRead;
                    }

                    self.mode_clock -= 204;
                    self.update_stat()
                }
            }
            PpuMode::VBlank => {
                if self.mode_clock >= 456 {
                    self.ly += 1;

                    // in case the end of v-blank has been reached then
                    // we must jump again to the OAM read mode and reset
                    // the scan line counter to the zero value
                    if self.ly == 154 {
                        self.mode = PpuMode::OamRead;
                        self.ly = 0;
                    }

                    self.mode_clock -= 456;
                    self.update_stat()
                }
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x0040 => {
                let value = if self.switch_bg { 0x01 } else { 0x00 }
                    | if self.switch_obj { 0x02 } else { 0x00 }
                    | if self.obj_size { 0x02 } else { 0x00 }
                    | if self.bg_map { 0x08 } else { 0x00 }
                    | if self.bg_tile { 0x10 } else { 0x00 }
                    | if self.switch_window { 0x20 } else { 0x00 }
                    | if self.window_map { 0x40 } else { 0x00 }
                    | if self.switch_lcd { 0x80 } else { 0x00 };
                value
            }
            0x0041 => {
                let value = if self.stat_hblank { 0x08 } else { 0x00 }
                    | if self.stat_vblank { 0x10 } else { 0x00 }
                    | if self.stat_oam { 0x20 } else { 0x00 }
                    | if self.stat_lyc { 0x40 } else { 0x00 }
                    | if self.lyc == self.ly { 0x04 } else { 0x00 }
                    | (self.mode as u8 & 0x03);
                value
            }
            0x0042 => self.scy,
            0x0043 => self.scx,
            0x0044 => self.ly,
            0x0045 => self.lyc,
            0x004a => self.wy,
            0x004b => self.wx,
            _ => {
                warnln!("Reading from unknown PPU location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0x00ff {
            0x0040 => {
                self.switch_bg = value & 0x01 == 0x01;
                self.switch_obj = value & 0x02 == 0x02;
                self.obj_size = value & 0x04 == 0x04;
                self.bg_map = value & 0x08 == 0x08;
                self.bg_tile = value & 0x10 == 0x10;
                self.switch_window = value & 0x20 == 0x20;
                self.window_map = value & 0x40 == 0x40;
                self.switch_lcd = value & 0x80 == 0x80;

                // in case the LCD is off takes the opportunity
                // to clear the screen, this is the expected
                // behaviour for this specific situation
                if !self.switch_lcd {
                    self.clear_frame_buffer();
                }
            }
            0x0041 => {
                self.stat_hblank = value & 0x08 == 0x08;
                self.stat_vblank = value & 0x10 == 0x10;
                self.stat_oam = value & 0x20 == 0x20;
                self.stat_lyc = value & 0x40 == 0x40;
            }
            0x0042 => self.scy = value,
            0x0043 => self.scx = value,
            0x0045 => self.lyc = value,
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
            0x0048 => {
                for index in 0..PALETTE_SIZE {
                    match (value >> (index * 2)) & 3 {
                        0 => self.palette_obj_0[index] = [255, 255, 255],
                        1 => self.palette_obj_0[index] = [192, 192, 192],
                        2 => self.palette_obj_0[index] = [96, 96, 96],
                        3 => self.palette_obj_0[index] = [0, 0, 0],
                        color_index => panic!("Invalid palette color index {:04x}", color_index),
                    }
                }
            }
            0x0049 => {
                for index in 0..PALETTE_SIZE {
                    match (value >> (index * 2)) & 3 {
                        0 => self.palette_obj_1[index] = [255, 255, 255],
                        1 => self.palette_obj_1[index] = [192, 192, 192],
                        2 => self.palette_obj_1[index] = [96, 96, 96],
                        3 => self.palette_obj_1[index] = [0, 0, 0],
                        color_index => panic!("Invalid palette color index {:04x}", color_index),
                    }
                }
            }
            0x004a => self.wy = value,
            0x004b => self.wx = value,
            0x007f => (),
            _ => warnln!("Writing in unknown PPU location 0x{:04x}", addr),
        }
    }

    pub fn vram(&self) -> &[u8; VRAM_SIZE] {
        &self.vram
    }

    pub fn hram(&self) -> &[u8; HRAM_SIZE] {
        &self.hram
    }

    pub fn tiles(&self) -> &[Tile; TILE_COUNT] {
        &self.tiles
    }

    pub fn palette(&self) -> Palette {
        self.palette
    }

    pub fn palette_obj_0(&self) -> Palette {
        self.palette_obj_0
    }

    pub fn palette_obj_1(&self) -> Palette {
        self.palette_obj_1
    }

    pub fn int_vblank(&self) -> bool {
        self.int_vblank
    }

    pub fn set_int_vblank(&mut self, value: bool) {
        self.int_vblank = value;
    }

    pub fn ack_vblank(&mut self) {
        self.set_int_vblank(false);
    }

    pub fn int_stat(&self) -> bool {
        self.int_stat
    }

    pub fn set_int_stat(&mut self, value: bool) {
        self.int_stat = value;
    }

    pub fn ack_stat(&mut self) {
        self.set_int_stat(false);
    }

    /// Fills the frame buffer with pixels of the provided color,
    /// this method must represent the fastest way of achieving
    /// the fill background with color operation.
    pub fn fill_frame_buffer(&mut self, color: Pixel) {
        self.color_buffer.fill(0);
        for index in (0..self.frame_buffer.len()).step_by(RGB_SIZE) {
            self.frame_buffer[index] = color[0];
            self.frame_buffer[index + 1] = color[1];
            self.frame_buffer[index + 2] = color[2];
        }
    }

    /// Clears the current frame buffer, setting the background color
    /// for all the pixels in the frame buffer.
    pub fn clear_frame_buffer(&mut self) {
        self.fill_frame_buffer([255, 255, 255]);
    }

    /// Prints the tile data information to the stdout, this is
    /// useful for debugging purposes.
    pub fn print_tile_stdout(&self, tile_index: usize) {
        println!("{}", self.tiles[tile_index]);
    }

    /// Updates the tile structure with the value that has
    /// just been written to a location on the VRAM associated
    /// with tiles.
    pub fn update_tile(&mut self, addr: u16, _value: u8) {
        let addr = (addr & 0x1ffe) as usize;
        let tile_index = ((addr >> 4) & 0x01ff) as usize;
        let tile = self.tiles[tile_index].borrow_mut();
        let y = ((addr >> 1) & 0x0007) as usize;

        let mut mask;

        for x in 0..TILE_WIDTH {
            mask = 1 << (7 - x);
            tile.set(
                x,
                y,
                if self.vram[addr] & mask > 0 { 0x1 } else { 0x0 }
                    | if self.vram[addr + 1] & mask > 0 {
                        0x2
                    } else {
                        0x0
                    },
            );
        }
    }

    pub fn update_object(&mut self, addr: u16, value: u8) {
        let addr = (addr & 0x01ff) as usize;
        let obj_index = addr >> 2;
        if obj_index >= OBJ_COUNT {
            return;
        }
        let mut obj = self.obj_data[obj_index].borrow_mut();
        match addr & 0x03 {
            0x00 => obj.y = value as i16 - 16,
            0x01 => obj.x = value as i16 - 8,
            0x02 => obj.tile = value,
            0x03 => {
                obj.palette = if value & 0x10 == 0x10 { 1 } else { 0 };
                obj.xflip = if value & 0x20 == 0x20 { true } else { false };
                obj.yflip = if value & 0x40 == 0x40 { true } else { false };
                obj.priority = if value & 0x80 == 0x80 { false } else { true };
                obj.index = obj_index as u8;
            }
            _ => (),
        }
    }

    fn render_line(&mut self) {
        if !self.switch_lcd {
            return;
        }
        if self.switch_bg {
            self.render_map(self.bg_map, self.scx, self.scy, 0, 0);
        }
        if self.switch_window {
            self.render_map(self.window_map, 0, 0, self.wx - 7, self.wy);
        }
        if self.switch_obj {
            self.render_objects();
        }
    }

    fn render_map(&mut self, map: bool, scx: u8, scy: u8, wx: u8, wy: u8) {
        // in case the target window Y position has not yet been reached
        // then there's nothing to be done, returns control flow immediately
        if self.ly < wy {
            return;
        }

        // calculates the LD (line delta) useful for draw operations where the window
        // is repositioned, as the current line for tile calculus is different from
        // the one currently in drawing
        let ld = self.ly - wy;

        // calculates the row offset for the tile by using the current line
        // index and the DY (scroll Y) divided by 8 (as the tiles are 8x8 pixels),
        // on top of that ensures that the result is modulus 32 meaning that the
        // drawing wraps around the Y axis
        let row_offset = (((ld + scy) & 0xff) >> 3) % 32;

        // obtains the base address of the background map using the bg map flag
        // that control which background map is going to be used
        let mut map_offset: usize = if map { 0x1c00 } else { 0x1800 };

        // increments the map offset by the row offset multiplied by the number
        // of tiles in each row (32)
        map_offset += row_offset as usize * 32;

        // calculates the sprite line offset by using the SCX register
        // shifted by 3 meaning that the tiles are 8x8
        let mut line_offset: usize = (scx >> 3) as usize;

        // calculates the index of the initial tile in drawing,
        // if the tile data set in use is #1, the indices are
        // signed, then calculates a real tile offset
        let mut tile_index = self.vram[map_offset + line_offset] as usize;
        if !self.bg_tile && tile_index < 128 {
            tile_index += 256;
        }

        // calculates the offset that is going to be used in the update of the color buffer
        // which stores Game Boy colors from 0 to 3
        let mut color_offset = self.ly as usize * DISPLAY_WIDTH;

        // calculates the frame buffer offset position assuming the proper
        // Game Boy screen width and RGB pixel (3 bytes) size
        let mut frame_offset = self.ly as usize * DISPLAY_WIDTH * RGB_SIZE;

        // calculates both the current Y and X positions within the tiles
        // using the bitwise and operation as an effective modulus 8
        let y = ((ld + scy) & 0x07) as usize;
        let mut x = (scx & 0x07) as usize;

        for index in 0..DISPLAY_WIDTH {
            // in case the current pixel to be drawn for the line
            // is visible within the window draws it an increments
            // the X coordinate of the tile
            if index as u8 >= wx {
                // obtains the current pixel data from the tile and
                // re-maps it according to the current palette
                let pixel = self.tiles[tile_index].get(x, y);
                let color = self.palette[pixel as usize];

                // updates the pixel in the color buffer
                self.color_buffer[color_offset] = pixel;

                // set the color pixel in the frame buffer
                self.frame_buffer[frame_offset] = color[0];
                self.frame_buffer[frame_offset + 1] = color[1];
                self.frame_buffer[frame_offset + 2] = color[2];

                // increments the current tile X position in drawing
                x += 1;

                // in case the end of tile width has been reached then
                // a new tile must be retrieved for plotting
                if x == TILE_WIDTH {
                    // resets the tile X position to the base value
                    // as a new tile is going to be drawn
                    x = 0;

                    // calculates the new line tile offset making sure that
                    // the maximum of 32 is not overflown
                    line_offset = (line_offset + 1) % 32;

                    // calculates the tile index and makes sure the value
                    // takes into consideration the bg tile value
                    tile_index = self.vram[map_offset + line_offset] as usize;
                    if !self.bg_tile && tile_index < 128 {
                        tile_index += 256;
                    }
                }
            }

            // increments the color offset by one, representing
            // the drawing og one pixel
            color_offset += 1;

            // increments the offset of the frame buffer by the
            // size of an RGB pixel (which is 3 bytes)
            frame_offset += RGB_SIZE;
        }
    }

    fn render_objects(&mut self) {
        for index in 0..OBJ_COUNT {
            // obtains the meta data of the object that is currently
            // under iteration to be checked for drawing
            let obj = self.obj_data[index];

            // verifies if the sprite is currently located at the
            // current line that is going to be drawn and skips it
            // in case it's not
            let is_contained =
                (obj.y <= self.ly as i16) && ((obj.y + TILE_HEIGHT as i16) > self.ly as i16);
            if !is_contained {
                continue;
            }

            let palette = if obj.palette == 0 {
                self.palette_obj_0
            } else {
                self.palette_obj_1
            };

            let mut color_offset = self.ly as usize * DISPLAY_WIDTH + obj.x as usize;

            // calculates the offset in the frame buffer for the sprite
            // that is going to be drawn, this is going to be the starting
            // point for the draw operation to be performed
            let mut frame_offset = (self.ly as usize * DISPLAY_WIDTH + obj.x as usize) * RGB_SIZE;

            let tile_offset = self.ly as i16 - obj.y;

            let tile_row: &[u8];
            if obj.yflip {
                tile_row = self.tiles[obj.tile as usize].get_row((7 - tile_offset) as usize);
            } else {
                tile_row = self.tiles[obj.tile as usize].get_row((tile_offset) as usize);
            }

            for x in 0..TILE_WIDTH {
                let is_contained =
                    (obj.x + x as i16 >= 0) && ((obj.x + x as i16) < DISPLAY_WIDTH as i16);
                if is_contained {
                    // the object is only considered visible if it's a priority
                    // or if the underlying pixel is transparent (zero value)
                    let is_visible = obj.priority || self.color_buffer[color_offset] == 0;
                    let pixel = tile_row[if obj.xflip { 7 - x } else { x }];
                    if is_visible && pixel != 0 {
                        // obtains the current pixel data from the tile row and
                        // re-maps it according to the object palette
                        let color = palette[pixel as usize];

                        // sets the color pixel in the frame buffer
                        self.frame_buffer[frame_offset] = color[0];
                        self.frame_buffer[frame_offset + 1] = color[1];
                        self.frame_buffer[frame_offset + 2] = color[2];
                    }
                }

                // increment the color offset by one as this represents
                // the advance of one color pixel
                color_offset += 1;

                // increments the offset of the frame buffer by the
                // size of an RGB pixel (which is 3 bytes)
                frame_offset += RGB_SIZE;
            }
        }
    }

    /// Runs an update operation on the LCD STAT interrupt meaning
    /// that the flag that control will be updated in case the conditions
    /// required for the LCD STAT interrupt to be triggered are met.
    fn update_stat(&mut self) {
        if self.stat_level() {
            self.int_stat = true;
        }
    }

    /// Obtains the current level of the LCD STAT interrupt by
    /// checking the current PPU state in various sections.
    fn stat_level(&self) -> bool {
        self.stat_lyc && self.lyc == self.ly
            || self.stat_oam && self.mode == PpuMode::OamRead
            || self.stat_vblank && self.mode == PpuMode::VBlank
            || self.stat_vblank && self.mode == PpuMode::HBlank
    }
}
