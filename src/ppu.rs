//! PPU (Picture Processing Unit) functions and structures.

use core::fmt;
use std::{
    borrow::BorrowMut,
    cmp::max,
    convert::TryInto,
    fmt::{Display, Formatter},
    sync::{Arc, Mutex},
};

use crate::{
    color::{
        rgb555_to_rgb888, rgb888_to_rgb1555_array, rgb888_to_rgb1555_u16, rgb888_to_rgb565,
        rgb888_to_rgb565_u16, Pixel, PixelAlpha, RGB1555_SIZE, RGB565_SIZE, RGB888_SIZE, RGB_SIZE,
        XRGB8888_SIZE,
    },
    consts::{
        BGP_ADDR, LCDC_ADDR, LYC_ADDR, LY_ADDR, OBP0_ADDR, OBP1_ADDR, SCX_ADDR, SCY_ADDR,
        STAT_ADDR, WX_ADDR, WY_ADDR,
    },
    gb::{GameBoyConfig, GameBoyMode},
    mmu::BusComponent,
    panic_gb,
    util::SharedThread,
    warnln,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub const VRAM_SIZE_DMG: usize = 8192;
pub const VRAM_SIZE_CGB: usize = 16384;
pub const VRAM_SIZE: usize = VRAM_SIZE_CGB;
pub const HRAM_SIZE: usize = 128;
pub const OAM_SIZE: usize = 260;
pub const PALETTE_SIZE: usize = 4;
pub const TILE_WIDTH: usize = 8;
pub const TILE_HEIGHT: usize = 8;
pub const TILE_WIDTH_I: usize = 7;
pub const TILE_HEIGHT_I: usize = 7;
pub const TILE_DOUBLE_HEIGHT: usize = 16;

pub const TILE_COUNT_DMG: usize = 384;
pub const TILE_COUNT_CGB: usize = 768;

/// The number of tiles that can be store in Game Boy's
/// VRAM memory according to specifications.
pub const TILE_COUNT: usize = TILE_COUNT_CGB;

/// The number of objects/sprites that can be handled at
/// the same time by the Game Boy.
pub const OBJ_COUNT: usize = 40;

/// The width of the Game Boy screen in pixels.
pub const DISPLAY_WIDTH: usize = 160;

/// The height of the Game Boy screen in pixels.
pub const DISPLAY_HEIGHT: usize = 144;

/// The size in pixels of the display.
pub const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

/// The size to be used by the buffer of color ids
/// for the Game Boy screen, the values there should
/// range from 0 to 3.
pub const COLOR_BUFFER_SIZE: usize = DISPLAY_SIZE;

/// The size of the buffer that will hold the concrete shade
/// index (0 to 3) for each of the pixel in the screen, so that
/// it is possible to rebuild the pixel buffer from scratch.
pub const SHADE_BUFFER_SIZE: usize = DISPLAY_SIZE;

/// The size of the RGB frame buffer in bytes.
pub const FRAME_BUFFER_SIZE: usize = DISPLAY_SIZE * RGB_SIZE;

/// The size of the RGB888 frame buffer in bytes.
pub const FRAME_BUFFER_RGB888_SIZE: usize = DISPLAY_SIZE * RGB888_SIZE;

/// The size of the XRGB8888 frame buffer in bytes.
pub const FRAME_BUFFER_XRGB8888_SIZE: usize = DISPLAY_SIZE * XRGB8888_SIZE;

/// The size of the RGB1555 frame buffer in bytes.
pub const FRAME_BUFFER_RGB1555_SIZE: usize = DISPLAY_SIZE * RGB1555_SIZE;

/// The size of the RGB565 frame buffer in bytes.
pub const FRAME_BUFFER_RGB565_SIZE: usize = DISPLAY_SIZE * RGB565_SIZE;

/// The base colors to be used to populate the
/// custom palettes of the Game Boy.
pub const PALETTE_COLORS: Palette = [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]];

/// Default tile data to be used in the DMG compatibility
/// mode of tile processing (avoids algorithmic forking).
pub const DEFAULT_TILE_ATTR: TileData = TileData {
    palette: 0,
    vram_bank: 0,
    xflip: false,
    yflip: false,
    priority: false,
};

/// A basic palette for DMG with the typical high contrast
/// color characteristic of the Game Boy.
pub const BASIC_PALETTE: Palette = [
    [0xff, 0xff, 0xff],
    [0xc0, 0xc0, 0xc0],
    [0x60, 0x60, 0x60],
    [0x00, 0x00, 0x00],
];

/// Defines a type that represents a color palette
/// within the Game Boy context.
pub type Palette = [Pixel; PALETTE_SIZE];

/// Defines a type that represents a color palette
/// with alpha within the Game Boy context.
pub type PaletteAlpha = [PixelAlpha; PALETTE_SIZE];

/// Represents a palette together with the metadata
/// that is associated with it.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, PartialEq, Eq)]
pub struct PaletteInfo {
    name: String,
    colors: Palette,
}

impl PaletteInfo {
    pub fn new(name: &str, colors: Palette) -> Self {
        Self {
            name: String::from(name),
            colors,
        }
    }

    pub fn from_colors_hex(name: &str, colors_hex: &str) -> Self {
        let colors = Self::parse_colors_hex(colors_hex);
        Self::new(name, colors)
    }

    pub fn parse_colors_hex(colors_hex: &str) -> Palette {
        let mut colors = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        for (index, color) in colors_hex.split(',').enumerate() {
            let color = color.trim();
            let color = u32::from_str_radix(color, 16).unwrap_or(0);
            let r = ((color >> 16) & 0xff) as u8;
            let g = ((color >> 8) & 0xff) as u8;
            let b = (color & 0xff) as u8;
            colors[index] = [r, g, b];
        }
        colors
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns the colors in RGB format.
    pub fn colors(&self) -> &Palette {
        &self.colors
    }

    /// Returns the colors in hex format, separated by comma.
    pub fn colors_hex(&self) -> String {
        let mut buffer = String::new();
        let mut is_first = true;
        for color in self.colors.iter() {
            let r = color[0];
            let g = color[1];
            let b = color[2];
            let color = (r as u32) << 16 | (g as u32) << 8 | b as u32;
            if is_first {
                is_first = false;
            } else {
                buffer.push(',');
            }
            buffer.push_str(format!("{:06x}", color).as_str());
        }
        buffer
    }
}

/// Represents a tile within the Game Boy context,
/// should contain the pixel buffer of the tile.
/// The tiles are always 8x8 pixels in size.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// The buffer for the tile, should contain a byte
    /// per each pixel of the tile with values ranging
    /// from 0 to 3 (4 colors).
    buffer: [u8; 64],
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Tile {
    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.buffer[y * TILE_WIDTH + x]
    }

    pub fn get_flipped(&self, x: usize, y: usize, xflip: bool, yflip: bool) -> u8 {
        let x: usize = if xflip { TILE_WIDTH_I - x } else { x };
        let y = if yflip { TILE_HEIGHT_I - y } else { y };
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
            buffer.push('\n');
        }
        write!(f, "{}", buffer)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectData {
    x: i16,
    y: i16,
    tile: u8,
    palette_cgb: u8,
    tile_bank: u8,
    palette: u8,
    xflip: bool,
    yflip: bool,
    bg_over: bool,
    index: u8,
}

impl ObjectData {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            tile: 0,
            palette_cgb: 0,
            tile_bank: 0,
            palette: 0,
            xflip: false,
            yflip: false,
            bg_over: false,
            index: 0,
        }
    }
}

impl Default for ObjectData {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ObjectData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Index: {}, X: {}, Y: {}, Tile: {}",
            self.index, self.x, self.y, self.tile
        )
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TileData {
    palette: u8,
    vram_bank: u8,
    xflip: bool,
    yflip: bool,
    priority: bool,
}

impl TileData {
    pub fn new() -> Self {
        Self {
            palette: 0,
            vram_bank: 0,
            xflip: false,
            yflip: false,
            priority: false,
        }
    }
}

impl Default for TileData {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for TileData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Palette: {}, VRAM Bank: {}, X Flip: {}, Y Flip: {}",
            self.palette, self.vram_bank, self.xflip, self.yflip
        )
    }
}

pub struct PpuRegisters {
    pub scy: u8,
    pub scx: u8,
    pub wy: u8,
    pub wx: u8,
    pub ly: u8,
    pub lyc: u8,
}

/// Represents the Game Boy PPU (Pixel Processing Unit) and controls
/// all of the logic behind the graphics processing and presentation.
/// Should store both the VRAM and HRAM together with the internal
/// graphic related registers.
/// Outputs the screen as a RGB 8 bit frame buffer.
///
/// # Basic usage
///
/// ```rust
/// use boytacean::ppu::Ppu;
/// let mut ppu = Ppu::default();
/// ppu.clock(8);
/// ```
pub struct Ppu {
    /// The color buffer that is going to store the colors
    /// (from 0 to 3) for all the pixels in the screen.
    pub color_buffer: Box<[u8; COLOR_BUFFER_SIZE]>,

    /// The shades buffer that is going to store the shade
    /// tone value (0 to 3) for all the pixels in the screen (DMG only).
    pub shade_buffer: Box<[u8; SHADE_BUFFER_SIZE]>,

    /// The 8 bit based RGB frame buffer with the processed
    /// set of pixels ready to be displayed on screen.
    /// This value may be lazy computed in case the DMG mode
    /// is in use, the lazy evaluation is controlled by
    /// the `frame_buffer_index` value.
    frame_buffer: Box<[u8; FRAME_BUFFER_SIZE]>,

    /// The buffer that will control the background to OAM
    /// priority, allowing the background to be drawn over
    /// the sprites/objects if necessary.
    priority_buffer: Box<[bool; COLOR_BUFFER_SIZE]>,

    /// Video dedicated memory (VRAM) where both the tiles and
    /// the sprites/objects are going to be stored.
    vram: [u8; VRAM_SIZE],

    /// High RAM memory that should provide extra speed for regular
    /// operations.
    hram: [u8; HRAM_SIZE],

    /// OAM RAM (Sprite Attribute Table ) used for the storage of the
    /// sprite attributes for each of the 40 sprites of the Game Boy.
    oam: [u8; OAM_SIZE],

    /// The VRAM bank to be used in the read and write operation of
    /// the 0x8000-0x9FFF memory range (CGB only).
    vram_bank: u8,

    /// The offset to be used in the read and write operation of
    /// the VRAM, this value should be consistent with the VRAM bank
    /// that is currently selected (CGB only).
    vram_offset: u16,

    /// The current set of processed tiles that are store in the
    /// PPU related structures.
    tiles: [Tile; TILE_COUNT],

    /// The meta information about the sprites/objects that are going
    /// to be drawn to the screen,
    obj_data: [ObjectData; OBJ_COUNT],

    /// The base colors that are going to be used in the registration
    /// of the concrete palettes, this value basically controls the
    /// colors that are going to be shown for each of the four base
    /// values - 0x00, 0x01, 0x02, and 0x03.
    palette_colors: Palette,

    /// The palette of colors that is currently loaded in Game Boy
    /// and used for background (tiles) and window.
    palette_bg: Palette,

    /// The palette that is going to be used for sprites/objects #0.
    palette_obj_0: Palette,

    /// The palette that is going to be used for sprites/objects #1.
    palette_obj_1: Palette,

    /// The complete set of background palettes that are going to be
    /// used in CGB emulation to provide the full set of colors (CGB only).
    palettes_color_bg: [Palette; 8],

    /// The complete set of object/sprite palettes that are going to be
    /// used in CGB emulation to provide the full set of colors (CGB only).
    palettes_color_obj: [Palette; 8],

    /// The complete set of palettes in binary data so that they can
    /// be re-read if required by the system.
    palettes: [u8; 3],

    /// The raw binary information (64 bytes) for the color palettes,
    /// contains binary information for both the background and
    /// the objects palettes (CGB only).
    palettes_color: [[u8; 64]; 2],

    /// The complete list of attributes for the first background
    /// map that is located in 0x9800-0x9BFF (CGB only).
    bg_map_attrs_0: [TileData; 1024],

    /// The complete list of attributes for the second background
    /// map that is located in 0x9C00-0x9FFF (CGB only).
    bg_map_attrs_1: [TileData; 1024],

    /// The flag that controls if the object/sprite priority
    /// if set means that the priority mode to be used is the
    /// X coordinate otherwise the normal CGB OAM memory mode
    /// mode is used, the value of this flag is controlled by
    /// the OPRI register (CGB only)
    obj_priority: bool,

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
    /// the 154 lines plus 10 extra V-Blank lines.
    ly: u8,

    /// The line compare register that is going to be used
    /// in the STATE and associated interrupts.
    lyc: u8,

    /// The current execution mode of the PPU, should change
    /// between states over the drawing of a frame.
    mode: PpuMode,

    /// Internal clock counter used to control the time in ticks
    /// spent in each of the PPU modes.
    mode_clock: u16,

    /// Controls if the background is going to be drawn to screen.
    /// In CGB mode this flag controls the master priority instead
    /// enabling or disabling complex priority rules.
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

    /// Controls if the window is meant to be drawn.
    switch_window: bool,

    /// Controls the offset of the map that is going to be drawn
    /// for the window section of the screen.
    window_map: bool,

    /// Flag that controls if the LCD screen is ON and displaying
    /// content.
    switch_lcd: bool,

    // Internal window counter value used to control the lines that
    // were effectively rendered as part of the window tile drawing process.
    // A line is only considered rendered when the WX and WY registers
    // are within the valid screen range and the window switch register
    // is valid.
    window_counter: u8,

    /// If the auto increment of the background color palette is enabled
    /// so that the next address is going to be set on every write.
    auto_increment_bg: bool,

    /// The current address in usage for the background color palettes.
    palette_address_bg: u8,

    /// If the auto increment of the object/sprite color palette is enabled
    /// so that the next address is going to be set on every write.
    auto_increment_obj: bool,

    /// The current address in usage for the object/sprite color palettes.
    palette_address_obj: u8,

    /// Flag that controls if the frame currently in rendering is the
    /// first one, preventing actions.
    first_frame: bool,

    /// Almost unique identifier of the frame that can be used to debug
    /// and uniquely identify the frame that is currently ind drawing,
    /// the identifier wraps on the u16 edges.
    frame_index: u16,

    /// Index of the last frame that was rendered, this value is used
    /// to control the deferred rendering of the frame buffer and should
    /// prevent unnecessary resource usage.
    frame_buffer_index: u16,

    stat_hblank: bool,
    stat_vblank: bool,
    stat_oam: bool,
    stat_lyc: bool,

    /// Boolean value set when the V-Blank interrupt should be handled
    /// by the next CPU clock operation.
    int_vblank: bool,

    /// Boolean value when the LCD STAT interrupt should be handled by
    /// the next CPU clock operation.
    int_stat: bool,

    /// Flag that controls if the DMG compatibility mode is
    /// enabled meaning that some of the PPU decisions will
    /// be made differently to address this special situation
    /// (CGB only).
    dmg_compat: bool,

    /// The current running mode of the emulator, this
    /// may affect many aspects of the emulation.
    gb_mode: GameBoyMode,

    /// The pointer to the parent configuration of the running
    /// Game Boy emulator, that can be used to control the behaviour
    /// of Game Boy emulation.
    gbc: SharedThread<GameBoyConfig>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    OamRead = 2,
    VramRead = 3,
}

impl Ppu {
    pub fn new(mode: GameBoyMode, gbc: SharedThread<GameBoyConfig>) -> Self {
        Self {
            color_buffer: Box::new([0u8; COLOR_BUFFER_SIZE]),
            shade_buffer: Box::new([0u8; COLOR_BUFFER_SIZE]),
            frame_buffer: Box::new([0u8; FRAME_BUFFER_SIZE]),
            priority_buffer: Box::new([false; COLOR_BUFFER_SIZE]),
            vram: [0u8; VRAM_SIZE],
            hram: [0u8; HRAM_SIZE],
            oam: [0u8; OAM_SIZE],
            vram_bank: 0x0,
            vram_offset: 0x0000,
            tiles: [Tile { buffer: [0u8; 64] }; TILE_COUNT],
            obj_data: [ObjectData::default(); OBJ_COUNT],
            palette_colors: PALETTE_COLORS,
            palette_bg: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            palette_obj_0: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            palette_obj_1: [[0u8; RGB_SIZE]; PALETTE_SIZE],
            palettes_color_bg: [[[0u8; RGB_SIZE]; PALETTE_SIZE]; 8],
            palettes_color_obj: [[[0u8; RGB_SIZE]; PALETTE_SIZE]; 8],
            palettes: [0u8; 3],
            palettes_color: [[0u8; 64]; 2],
            bg_map_attrs_0: [TileData::default(); 1024],
            bg_map_attrs_1: [TileData::default(); 1024],
            obj_priority: false,
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
            window_counter: 0x0,
            auto_increment_bg: false,
            palette_address_bg: 0x0,
            auto_increment_obj: false,
            palette_address_obj: 0x0,
            first_frame: false,
            frame_index: 0,
            frame_buffer_index: u16::MAX,
            stat_hblank: false,
            stat_vblank: false,
            stat_oam: false,
            stat_lyc: false,
            int_vblank: false,
            int_stat: false,
            dmg_compat: false,
            gb_mode: mode,
            gbc,
        }
    }

    pub fn reset(&mut self) {
        self.color_buffer = Box::new([0u8; COLOR_BUFFER_SIZE]);
        self.shade_buffer = Box::new([0u8; SHADE_BUFFER_SIZE]);
        self.frame_buffer = Box::new([0u8; FRAME_BUFFER_SIZE]);
        self.priority_buffer = Box::new([false; COLOR_BUFFER_SIZE]);
        self.vram = [0u8; VRAM_SIZE_CGB];
        self.hram = [0u8; HRAM_SIZE];
        self.vram_bank = 0x0;
        self.vram_offset = 0x0000;
        self.tiles = [Tile { buffer: [0u8; 64] }; TILE_COUNT];
        self.obj_data = [ObjectData::default(); OBJ_COUNT];
        self.palette_bg = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.palette_obj_0 = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.palette_obj_1 = [[0u8; RGB_SIZE]; PALETTE_SIZE];
        self.palettes_color_bg = [[[0u8; RGB_SIZE]; PALETTE_SIZE]; 8];
        self.palettes_color_obj = [[[0u8; RGB_SIZE]; PALETTE_SIZE]; 8];
        self.palettes = [0u8; 3];
        self.palettes_color = [[0u8; 64]; 2];
        self.bg_map_attrs_0 = [TileData::default(); 1024];
        self.bg_map_attrs_1 = [TileData::default(); 1024];
        self.obj_priority = false;
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
        self.window_counter = 0;
        self.auto_increment_bg = false;
        self.palette_address_bg = 0x0;
        self.auto_increment_obj = false;
        self.palette_address_obj = 0x0;
        self.first_frame = false;
        self.frame_index = 0;
        self.frame_buffer_index = u16::MAX;
        self.stat_hblank = false;
        self.stat_vblank = false;
        self.stat_oam = false;
        self.stat_lyc = false;
        self.int_vblank = false;
        self.int_stat = false;
        self.dmg_compat = false;
    }

    pub fn clock(&mut self, cycles: u16) {
        // in case the LCD is currently off then we skip the current
        // clock operation the PPU should not work
        if !self.switch_lcd {
            return;
        }

        // increments the current mode clock by the provided amount
        // of CPU cycles (probably coming from a previous CPU clock)
        self.mode_clock += cycles;

        match self.mode {
            PpuMode::OamRead => {
                if self.mode_clock >= 80 {
                    self.mode = PpuMode::VramRead;
                    self.mode_clock -= 80;
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
                    // increments the window counter making sure that the
                    // valid is only incremented when both the WX and WY
                    // registers make sense (are within range), the window
                    // switch is on and the line in drawing is above WY
                    if self.switch_window
                        && self.wx as i16 - 7 < DISPLAY_WIDTH as i16
                        && self.wy < DISPLAY_HEIGHT as u8
                        && self.ly >= self.wy
                    {
                        self.window_counter += 1;
                    }

                    // increments the register that holds the
                    // information about the current line in drawing
                    self.ly += 1;

                    // in case we've reached the end of the
                    // screen we're now entering the V-Blank
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
                    // increments the register that controls the line count,
                    // notice that these represent the extra 10 horizontal
                    // scanlines that are virtual and not real (off-screen)
                    self.ly += 1;

                    // in case the end of V-Blank has been reached then
                    // we must jump again to the OAM read mode and reset
                    // the scan line counter to the zero value
                    if self.ly == 154 {
                        self.mode = PpuMode::OamRead;
                        self.ly = 0;
                        self.window_counter = 0;
                        self.first_frame = false;
                        self.frame_index = self.frame_index.wrapping_add(1);
                        self.update_stat()
                    }

                    self.mode_clock -= 456;
                }
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9fff => self.vram[(self.vram_offset + (addr & 0x1fff)) as usize],
            0xfe00..=0xfe9f => self.oam[(addr & 0x009f) as usize],
            // Not Usable
            0xfea0..=0xfeff => 0xff,
            0xff80..=0xfffe => self.hram[(addr & 0x007f) as usize],
            LCDC_ADDR =>
            {
                #[allow(clippy::bool_to_int_with_if)]
                (if self.switch_bg { 0x01 } else { 0x00 }
                    | if self.switch_obj { 0x02 } else { 0x00 }
                    | if self.obj_size { 0x04 } else { 0x00 }
                    | if self.bg_map { 0x08 } else { 0x00 }
                    | if self.bg_tile { 0x10 } else { 0x00 }
                    | if self.switch_window { 0x20 } else { 0x00 }
                    | if self.window_map { 0x40 } else { 0x00 }
                    | if self.switch_lcd { 0x80 } else { 0x00 })
            }
            STAT_ADDR => {
                (if self.stat_hblank { 0x08 } else { 0x00 }
                    | if self.stat_vblank { 0x10 } else { 0x00 }
                    | if self.stat_oam { 0x20 } else { 0x00 }
                    | if self.stat_lyc { 0x40 } else { 0x00 }
                    | if self.lyc == self.ly { 0x04 } else { 0x00 }
                    | (self.mode as u8 & 0x03)
                    | 0x80)
            }
            SCY_ADDR => self.scy,
            SCX_ADDR => self.scx,
            LY_ADDR => self.ly,
            // 0xFF45 — LYC
            LYC_ADDR => self.lyc,
            // 0xFF47 — BGP (Non-CGB Mode only)
            BGP_ADDR => self.palettes[0],
            // 0xFF48 — OBP0 (Non-CGB Mode only)
            OBP0_ADDR => self.palettes[1],
            // 0xFF49 — OBP1 (Non-CGB Mode only)
            OBP1_ADDR => self.palettes[2],
            // 0xFF4A — WX
            WX_ADDR => self.wy,
            // 0xFF4B — WY
            WY_ADDR => self.wx,
            // 0xFF4F — VBK (CGB only)
            0xff4f => self.vram_bank | 0xfe,
            // 0xFF68 — BCPS/BGPI (CGB only)
            0xff68 => self.palette_address_bg | if self.auto_increment_bg { 0x80 } else { 0x00 },
            // 0xFF69 — BCPD/BGPD (CGB only)
            0xff69 => self.palettes_color[0][self.palette_address_bg as usize],
            // 0xFF6A — OCPS/OBPI (CGB only)
            0xff6a => self.palette_address_obj | if self.auto_increment_obj { 0x80 } else { 0x00 },
            // 0xFF6B — OCPD/OBPD (CGB only)
            0xff6b => self.palettes_color[1][self.palette_address_obj as usize],
            // 0xFF6C — OPRI (CGB only)
            0xff6c => (if self.obj_priority { 0x01 } else { 0x00 }) | 0xfe,
            _ => {
                warnln!("Reading from unknown PPU location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9fff => {
                self.vram[(self.vram_offset + (addr & 0x1fff)) as usize] = value;
                if addr < 0x9800 {
                    self.update_tile(addr, value);
                } else if self.vram_bank == 0x1 {
                    self.update_bg_map_attrs(addr, value);
                }
            }
            0xfe00..=0xfe9f => {
                self.oam[(addr & 0x009f) as usize] = value;
                self.update_object(addr, value);
            }
            // Not Usable
            0xfea0..=0xfeff => (),
            0xff80..=0xfffe => self.hram[(addr & 0x007f) as usize] = value,
            LCDC_ADDR => {
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
                    self.mode = PpuMode::HBlank;
                    self.mode_clock = 0;
                    self.ly = 0;
                    self.int_vblank = false;
                    self.int_stat = false;
                    self.window_counter = 0;
                    self.first_frame = true;
                    self.clear_frame_buffer();
                }
            }
            STAT_ADDR => {
                self.stat_hblank = value & 0x08 == 0x08;
                self.stat_vblank = value & 0x10 == 0x10;
                self.stat_oam = value & 0x20 == 0x20;
                self.stat_lyc = value & 0x40 == 0x40;
            }
            SCY_ADDR => self.scy = value,
            SCX_ADDR => self.scx = value,
            // 0xFF45 — LYC: LY compare
            LYC_ADDR => self.lyc = value,
            // 0xFF47 — BGP (Non-CGB Mode only)
            BGP_ADDR => {
                if value == self.palettes[0] {
                    return;
                }
                if self.dmg_compat {
                    Self::compute_palette(&mut self.palette_bg, &self.palettes_color_bg[0], value);
                } else {
                    Self::compute_palette(&mut self.palette_bg, &self.palette_colors, value);
                }
                self.palettes[0] = value;
            }
            // 0xFF48 — OBP0 (Non-CGB Mode only)
            OBP0_ADDR => {
                if value == self.palettes[1] {
                    return;
                }
                if self.dmg_compat {
                    Self::compute_palette(
                        &mut self.palette_obj_0,
                        &self.palettes_color_obj[0],
                        value,
                    );
                } else {
                    Self::compute_palette(&mut self.palette_obj_0, &self.palette_colors, value);
                }
                self.palettes[1] = value;
            }
            // 0xFF49 — OBP0 (Non-CGB Mode only)
            OBP1_ADDR => {
                if value == self.palettes[2] {
                    return;
                }
                if self.dmg_compat {
                    Self::compute_palette(
                        &mut self.palette_obj_1,
                        &self.palettes_color_obj[1],
                        value,
                    );
                } else {
                    Self::compute_palette(&mut self.palette_obj_1, &self.palette_colors, value);
                }
                self.palettes[2] = value;
            }
            // 0xFF4A — WX
            WX_ADDR => self.wy = value,
            // 0xFF4B — WY
            WY_ADDR => self.wx = value,
            // 0xFF4F — VBK (CGB only)
            0xff4f => {
                self.vram_bank = value & 0x01;
                self.vram_offset = self.vram_bank as u16 * 0x2000;
            }
            // 0xFF68 — BCPS/BGPI (CGB only)
            0xff68 => {
                self.palette_address_bg = value & 0x3f;
                self.auto_increment_bg = value & 0x80 == 0x80;
            }
            // 0xFF69 — BCPD/BGPD (CGB only)
            0xff69 => {
                let palette_index = self.palette_address_bg / 8;
                let color_index = (self.palette_address_bg % 8) / 2;

                let palette_color = &mut self.palettes_color[0];
                palette_color[self.palette_address_bg as usize] = value;
                let palette = &mut self.palettes_color_bg[palette_index as usize];
                Self::compute_palette_color(palette, palette_color, palette_index, color_index);

                if self.auto_increment_bg {
                    self.palette_address_bg = (self.palette_address_bg + 1) & 0x3f;
                }
            }
            // 0xFF6A — OCPS/OBPI (CGB only)
            0xff6a => {
                self.palette_address_obj = value & 0x3f;
                self.auto_increment_obj = value & 0x80 == 0x80;
            }
            // 0xFF6B — OCPD/OBPD (CGB only)
            0xff6b => {
                let palette_index = self.palette_address_obj / 8;
                let color_index = (self.palette_address_obj % 8) / 2;

                let palette_color = &mut self.palettes_color[1];
                palette_color[self.palette_address_obj as usize] = value;
                let palette = &mut self.palettes_color_obj[palette_index as usize];
                Self::compute_palette_color(palette, palette_color, palette_index, color_index);

                if self.auto_increment_obj {
                    self.palette_address_obj = (self.palette_address_obj + 1) & 0x3f;
                }
            }
            // 0xFF6C — OPRI (CGB only)
            0xff6c => self.obj_priority = value & 0x01 == 0x01,
            0xff7f => (),
            _ => warnln!("Writing in unknown PPU location 0x{:04x}", addr),
        }
    }

    pub fn frame_buffer(&mut self) -> &[u8; FRAME_BUFFER_SIZE] {
        if self.gb_mode != GameBoyMode::Dmg {
            return &self.frame_buffer;
        }

        if self.frame_index == self.frame_buffer_index {
            return &self.frame_buffer;
        }

        for (index, pixel) in self.frame_buffer.chunks_mut(RGB_SIZE).enumerate() {
            let shade_index = self.shade_buffer[index];
            let color = &self.palette_colors[shade_index as usize];
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }

        self.frame_buffer_index = self.frame_index;
        &self.frame_buffer
    }

    pub fn frame_buffer_xrgb8888(&mut self) -> [u8; FRAME_BUFFER_XRGB8888_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u8; FRAME_BUFFER_XRGB8888_SIZE];
        for index in 0..DISPLAY_SIZE {
            let (r, g, b) = (
                frame_buffer[index * RGB_SIZE],
                frame_buffer[index * RGB_SIZE + 1],
                frame_buffer[index * RGB_SIZE + 2],
            );
            buffer[index * XRGB8888_SIZE] = b;
            buffer[index * XRGB8888_SIZE + 1] = g;
            buffer[index * XRGB8888_SIZE + 2] = r;
            buffer[index * XRGB8888_SIZE + 3] = 0xff;
        }
        buffer
    }

    pub fn frame_buffer_xrgb8888_u32(&mut self) -> [u32; FRAME_BUFFER_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u32; FRAME_BUFFER_SIZE];
        for (index, pixel) in buffer.iter_mut().enumerate().take(DISPLAY_SIZE) {
            let (r, g, b) = (
                frame_buffer[index * RGB_SIZE],
                frame_buffer[index * RGB_SIZE + 1],
                frame_buffer[index * RGB_SIZE + 2],
            );
            *pixel = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        }
        buffer
    }

    pub fn frame_buffer_rgb1555(&mut self) -> [u8; FRAME_BUFFER_RGB1555_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u8; FRAME_BUFFER_RGB1555_SIZE];
        rgb888_to_rgb1555_array(frame_buffer, &mut buffer);
        buffer
    }

    pub fn frame_buffer_rgb1555_u16(&mut self) -> [u16; FRAME_BUFFER_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u16; FRAME_BUFFER_SIZE];
        for (index, pixel) in buffer.iter_mut().enumerate().take(DISPLAY_SIZE) {
            let (r, g, b) = (
                frame_buffer[index * RGB_SIZE],
                frame_buffer[index * RGB_SIZE + 1],
                frame_buffer[index * RGB_SIZE + 2],
            );
            *pixel = rgb888_to_rgb1555_u16(r, g, b);
        }
        buffer
    }

    pub fn frame_buffer_rgb565(&mut self) -> [u8; FRAME_BUFFER_RGB565_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u8; FRAME_BUFFER_RGB565_SIZE];
        for index in 0..DISPLAY_SIZE {
            let (r, g, b) = (
                frame_buffer[index * RGB_SIZE],
                frame_buffer[index * RGB_SIZE + 1],
                frame_buffer[index * RGB_SIZE + 2],
            );
            let rgb565 = rgb888_to_rgb565(r, g, b);
            buffer[index * RGB565_SIZE] = rgb565[0];
            buffer[index * RGB565_SIZE + 1] = rgb565[1];
        }
        buffer
    }

    pub fn frame_buffer_rgb565_u16(&mut self) -> [u16; FRAME_BUFFER_SIZE] {
        let frame_buffer = self.frame_buffer();
        let mut buffer = [0u16; FRAME_BUFFER_SIZE];
        for (index, pixel) in buffer.iter_mut().enumerate().take(DISPLAY_SIZE) {
            let (r, g, b) = (
                frame_buffer[index * RGB_SIZE],
                frame_buffer[index * RGB_SIZE + 1],
                frame_buffer[index * RGB_SIZE + 2],
            );
            *pixel = rgb888_to_rgb565_u16(r, g, b);
        }
        buffer
    }

    /// Obtains the "raw" version of the frame buffer any custom
    /// color palette operation applied to it. This is can be an
    /// extremely slow operation (in DMG devices) and because of
    /// that should be used carefully.
    pub fn frame_buffer_raw(&self) -> [u8; FRAME_BUFFER_SIZE] {
        self.frame_buffer_palette(&BASIC_PALETTE)
    }

    /// Obtains the frame buffer with the colors mapped according
    /// to the provided palette of colors.
    /// This method is very slow and only useful for the DMG mode
    /// which can have its simple colors mapped to palettes.
    pub fn frame_buffer_palette(&self, palette_colors: &Palette) -> [u8; FRAME_BUFFER_SIZE] {
        if self.gb_mode == GameBoyMode::Dmg {
            let mut buffer = [0u8; FRAME_BUFFER_SIZE];
            for (index, pixel) in buffer.chunks_mut(RGB_SIZE).enumerate() {
                let shade_index = self.shade_buffer[index];
                let color = &palette_colors[shade_index as usize];
                pixel[0] = color[0];
                pixel[1] = color[1];
                pixel[2] = color[2];
            }
            buffer
        } else {
            *self.frame_buffer.clone()
        }
    }

    pub fn vram(&self) -> &[u8; VRAM_SIZE] {
        &self.vram
    }

    pub fn vram_dmg(&self) -> &[u8] {
        &self.vram[0..VRAM_SIZE_DMG]
    }

    pub fn vram_cgb(&self) -> &[u8] {
        &self.vram[0..VRAM_SIZE_CGB]
    }

    pub fn vram_device(&self) -> &[u8] {
        match self.gb_mode {
            GameBoyMode::Dmg => self.vram_dmg(),
            GameBoyMode::Cgb => self.vram_cgb(),
            GameBoyMode::Sgb => self.vram_dmg(),
        }
    }

    pub fn set_vram(&mut self, value: &[u8]) {
        self.vram[0..value.len()].copy_from_slice(value);
        self.update_vram();
    }

    pub fn hram(&self) -> &[u8; HRAM_SIZE] {
        &self.hram
    }

    pub fn set_hram(&mut self, value: [u8; HRAM_SIZE]) {
        self.hram = value;
    }

    pub fn tiles(&self) -> &[Tile; TILE_COUNT] {
        &self.tiles
    }

    pub fn set_palette_colors(&mut self, value: &Palette) {
        self.palette_colors = *value;
        self.compute_palettes()
    }

    pub fn palette_bg(&self) -> Palette {
        self.palette_bg
    }

    pub fn palette_obj_0(&self) -> Palette {
        self.palette_obj_0
    }

    pub fn palette_obj_1(&self) -> Palette {
        self.palette_obj_1
    }

    pub fn palettes_color(&self) -> &[[u8; 64]; 2] {
        &self.palettes_color
    }

    pub fn set_palettes_color(&mut self, palettes_color: [[u8; 64]; 2]) {
        self.palettes_color = palettes_color;
        Self::compute_palettes_color(
            &mut [&mut self.palettes_color_bg, &mut self.palettes_color_obj],
            &self.palettes_color,
        );
    }

    pub fn ly(&self) -> u8 {
        self.ly
    }

    pub fn mode(&self) -> PpuMode {
        self.mode
    }

    pub fn frame_index(&self) -> u16 {
        self.frame_index
    }

    #[inline(always)]
    pub fn int_vblank(&self) -> bool {
        self.int_vblank
    }

    #[inline(always)]
    pub fn set_int_vblank(&mut self, value: bool) {
        self.int_vblank = value;
    }

    #[inline(always)]
    pub fn ack_vblank(&mut self) {
        self.set_int_vblank(false);
    }

    #[inline(always)]
    pub fn int_stat(&self) -> bool {
        self.int_stat
    }

    #[inline(always)]
    pub fn set_int_stat(&mut self, value: bool) {
        self.int_stat = value;
    }

    #[inline(always)]
    pub fn ack_stat(&mut self) {
        self.set_int_stat(false);
    }

    pub fn dmg_compat(&self) -> bool {
        self.dmg_compat
    }

    pub fn set_dmg_compat(&mut self, value: bool) {
        self.dmg_compat = value;

        // if we're switching to the DMG compat mode
        // then we need to recompute the palettes so
        // that the colors are correct according to
        // the compat palettes set by the Boot ROM
        if value {
            self.compute_palettes();
        }
    }

    pub fn gb_mode(&self) -> GameBoyMode {
        self.gb_mode
    }

    pub fn set_gb_mode(&mut self, value: GameBoyMode) {
        self.gb_mode = value;
    }

    pub fn set_gbc(&mut self, value: SharedThread<GameBoyConfig>) {
        self.gbc = value;
    }

    /// Fills the frame buffer with pixels of the provided color,
    /// this method should represent the fastest way of achieving
    /// the fill background with color operation.
    pub fn fill_frame_buffer(&mut self, shade_index: u8) {
        let color = &self.palette_colors[shade_index as usize];
        self.color_buffer.fill(0);
        self.shade_buffer.fill(shade_index);
        self.frame_buffer_index = u16::MAX;
        for pixel in self.frame_buffer.chunks_mut(RGB_SIZE) {
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }
    }

    /// Clears the current frame buffer, setting the background color
    /// for all the pixels in the frame buffer.
    pub fn clear_frame_buffer(&mut self) {
        self.fill_frame_buffer(0);
    }

    /// Prints the tile data information to the stdout, this is
    /// useful for debugging purposes.
    pub fn print_tile_stdout(&self, tile_index: usize) {
        println!("{}", self.tiles[tile_index]);
    }

    /// Updates the internal PPU state (calculated values) according
    /// to the VRAM values, this should be called whenever the VRAM
    /// is replaced.
    pub fn update_vram(&mut self) {
        // "saves" the old values of the VRAM bank and offset
        // as they are going to be needed later, this is required
        // as we're going to trick the PPU into switching banks
        // over the update of the calculated values for the new VRAM,
        // essentially required for the `update_tile()` method
        let (vram_bank_old, vram_offset_old) = (self.vram_bank, self.vram_offset);

        // determines the number of VRAM banks available according
        // to the running Game Boy running mode (CGB vs DMG)
        let vram_banks = if self.gb_mode == GameBoyMode::Cgb {
            2u8
        } else {
            1u8
        };

        // goes over all the VRAM banks, and over all the VRAM addresses
        // in those banks to update the internal tiles and background map
        // attributes structures accordingly
        for vram_bank in 0..vram_banks {
            self.vram_bank = vram_bank;
            self.vram_offset = self.vram_bank as u16 * 0x2000;
            for addr in 0x8000..=0x9fff {
                let value = self.vram[(self.vram_offset + (addr & 0x1fff)) as usize];
                if addr < 0x9800 {
                    self.update_tile(addr, value);
                } else if self.vram_bank == 0x1 {
                    self.update_bg_map_attrs(addr, value);
                }
            }
        }

        // restores the "old" values for VRAM bank and offset
        (self.vram_bank, self.vram_offset) = (vram_bank_old, vram_offset_old);
    }

    /// Updates the tile structure with the value that has
    /// just been written to a location on the VRAM associated
    /// with tiles.
    fn update_tile(&mut self, addr: u16, _value: u8) {
        let addr = (self.vram_offset + (addr & 0x1ffe)) as usize;
        let tile_index = ((addr >> 4) & 0x01ff) + (self.vram_bank as usize * TILE_COUNT_DMG);
        let tile = self.tiles[tile_index].borrow_mut();
        let y = (addr >> 1) & 0x0007;

        let mut mask;

        for x in 0..TILE_WIDTH {
            mask = 1 << (TILE_WIDTH_I - x);
            #[allow(clippy::bool_to_int_with_if)]
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

    fn update_object(&mut self, addr: u16, value: u8) {
        let addr = (addr & 0x01ff) as usize;
        let obj_index = addr >> 2;
        if obj_index >= OBJ_COUNT {
            return;
        }
        let obj = self.obj_data[obj_index].borrow_mut();
        match addr & 0x03 {
            0x00 => obj.y = value as i16 - 16,
            0x01 => obj.x = value as i16 - 8,
            0x02 => obj.tile = value,
            0x03 => {
                obj.palette_cgb = value & 0x07;
                obj.tile_bank = (value & 0x08 == 0x08) as u8;
                obj.palette = (value & 0x10 == 0x10) as u8;
                obj.xflip = value & 0x20 == 0x20;
                obj.yflip = value & 0x40 == 0x40;
                obj.bg_over = value & 0x80 == 0x80;
                obj.index = obj_index as u8;
            }
            _ => (),
        }
    }

    fn update_bg_map_attrs(&mut self, addr: u16, value: u8) {
        let bg_map = addr >= 0x9c00;
        let tile_index = if bg_map { addr - 0x9c00 } else { addr - 0x9800 };
        let bg_map_attrs = if bg_map {
            &mut self.bg_map_attrs_1
        } else {
            &mut self.bg_map_attrs_0
        };
        let tile_data: &mut TileData = bg_map_attrs[tile_index as usize].borrow_mut();
        tile_data.palette = value & 0x07;
        tile_data.vram_bank = (value & 0x08 == 0x08) as u8;
        tile_data.xflip = value & 0x20 == 0x20;
        tile_data.yflip = value & 0x40 == 0x40;
        tile_data.priority = value & 0x80 == 0x80;
    }

    pub fn registers(&self) -> PpuRegisters {
        PpuRegisters {
            scy: self.scy,
            scx: self.scx,
            wy: self.wy,
            wx: self.wx,
            ly: self.ly,
            lyc: self.lyc,
        }
    }

    fn render_line(&mut self) {
        if self.gb_mode == GameBoyMode::Dmg {
            self.render_line_dmg();
        } else {
            self.render_line_cgb();
        }
    }

    fn render_line_dmg(&mut self) {
        if self.first_frame {
            return;
        }
        if self.switch_bg {
            self.render_map_dmg(self.bg_map, self.scx, self.scy, 0, 0, self.ly);
        }
        if self.switch_bg && self.switch_window {
            self.render_map_dmg(self.window_map, 0, 0, self.wx, self.wy, self.window_counter);
        }
        if self.switch_obj {
            self.render_objects();
        }
    }

    fn render_line_cgb(&mut self) {
        if self.first_frame {
            return;
        }
        let switch_bg_window = (self.gb_mode.is_cgb() && !self.dmg_compat) || self.switch_bg;
        if switch_bg_window {
            self.render_map(self.bg_map, self.scx, self.scy, 0, 0, self.ly);
        }
        if switch_bg_window && self.switch_window {
            self.render_map(self.window_map, 0, 0, self.wx, self.wy, self.window_counter);
        }
        if self.switch_obj {
            self.render_objects();
        }
    }

    fn render_map(&mut self, map: bool, scx: u8, scy: u8, wx: u8, wy: u8, ld: u8) {
        // in case the target window Y position has not yet been reached
        // then there's nothing to be done, returns control flow immediately
        if self.ly < wy {
            return;
        }

        // selects the correct background attributes map based on the bg map flag
        // because the attributes are separated according to the map they represent
        // this is only relevant for CGB mode
        let bg_map_attrs = if map {
            self.bg_map_attrs_1
        } else {
            self.bg_map_attrs_0
        };

        // obtains the base address of the background map using the bg map flag
        // that control which background map is going to be used
        let map_offset: usize = if map { 0x1c00 } else { 0x1800 };

        // calculates the map row index for the tile by using the current line
        // index and the DY (scroll Y) divided by 8 (as the tiles are 8x8 pixels),
        // on top of that ensures that the result is modulus 32 meaning that the
        // drawing wraps around the Y axis
        let row_index = (((ld as usize + scy as usize) & 0xff) >> 3) % 32;

        // calculates the map offset by the row offset multiplied by the number
        // of tiles in each row (32)
        let row_offset = row_index * 32;

        // calculates the sprite line offset by using the SCX register
        // shifted by 3 meaning that the tiles are 8x8
        let mut line_offset = (scx >> 3) as usize;

        // calculates the index of the initial tile in drawing,
        // if the tile data set in use is #1, the indexes are
        // signed, then calculates a real tile offset
        let mut tile_index = self.vram[map_offset + row_offset + line_offset] as usize;
        if !self.bg_tile && tile_index < 128 {
            tile_index += 256;
        }

        // obtains the reference to the attributes of the new tile in
        // drawing for meta processing (CGB only)
        let mut tile_attr = if self.dmg_compat {
            &DEFAULT_TILE_ATTR
        } else {
            &bg_map_attrs[row_offset + line_offset]
        };

        // retrieves the proper palette for the current tile in drawing
        // taking into consideration if we're running in CGB mode or not
        let mut palette = if self.gb_mode == GameBoyMode::Cgb {
            if self.dmg_compat {
                &self.palette_bg
            } else {
                &self.palettes_color_bg[tile_attr.palette as usize]
            }
        } else {
            &self.palette_bg
        };

        // obtains the current integer value (raw) for the background palette
        // this is going to be used for shade index value computation (DMG only)
        let palette_v = self.palettes[0];

        // obtains the values of both X and Y flips for the current tile
        // they will be applied by the get tile pixel method
        let mut xflip = tile_attr.xflip;
        let mut yflip = tile_attr.yflip;

        // obtains the value the BG-to-OAM priority to be used in the computation
        // of the final pixel value (CGB only)
        let mut priority = tile_attr.priority;

        // increments the tile index value by the required offset for the VRAM
        // bank in which the tile is stored, this is only required for CGB mode
        tile_index += tile_attr.vram_bank as usize * TILE_COUNT_DMG;

        // obtains the reference to the tile that is going to be drawn
        let mut tile = &self.tiles[tile_index];

        // calculates the offset that is going to be used in the update of the color buffer
        // which stores Game Boy colors from 0 to 3
        let mut color_offset = self.ly as usize * DISPLAY_WIDTH;

        // calculates the frame buffer offset position assuming the proper
        // Game Boy screen width and RGB pixel (3 bytes) size
        let mut frame_offset = self.ly as usize * DISPLAY_WIDTH * RGB_SIZE;

        // calculates both the current Y and X positions within the tiles
        // using the bitwise and operation as an effective modulus 8
        let y = (ld as usize + scy as usize) & 0x07;
        let mut x = (scx & 0x07) as usize;

        // calculates the initial tile X position in drawing, doing this
        // allows us to position the background map properly in the display
        let initial_index = max(wx as i16 - 7, 0) as usize;
        color_offset += initial_index;
        frame_offset += initial_index * RGB_SIZE;

        // iterates over all the pixels in the current line of the display
        // to draw the background map, note that the initial index is used
        // to skip the drawing of the tiles that are not visible (WX)
        for _ in initial_index..DISPLAY_WIDTH {
            // obtains the current pixel data from the tile
            let pixel = tile.get_flipped(x, y, xflip, yflip);

            // updates the pixel in the color buffer, which stores
            // the raw pixel color information (unmapped) and then
            // updates the shade buffer with the shade index
            self.color_buffer[color_offset] = pixel;
            self.shade_buffer[color_offset] = (palette_v >> (pixel * 2)) & 3;

            // re-maps the pixel according to the current palette
            // and sets the color pixel in the frame buffer
            let color = &palette[pixel as usize];
            self.frame_buffer[frame_offset] = color[0];
            self.frame_buffer[frame_offset + 1] = color[1];
            self.frame_buffer[frame_offset + 2] = color[2];

            // updates the priority buffer with the current pixel
            // the priority is only set in case the priority of
            // the background (over OAM) is set in the attributes
            // and the pixel is not transparent
            self.priority_buffer[color_offset] = priority && pixel != 0;

            // increments the current tile X position in drawing
            x += 1;

            // in case the end of tile width has been reached then
            // a new tile must be retrieved for rendering
            if x == TILE_WIDTH {
                // resets the tile X position to the base value
                // as a new tile is going to be drawn
                x = 0;

                // calculates the new line tile offset making sure that
                // the maximum of 32 is not overflown
                line_offset = (line_offset + 1) % 32;

                // calculates the tile index and makes sure the value
                // takes into consideration the bg tile value
                tile_index = self.vram[map_offset + row_offset + line_offset] as usize;
                if !self.bg_tile && tile_index < 128 {
                    tile_index += 256;
                }

                // in case the current mode is CGB and the DMG compatibility
                // flag is not set then a series of tile values must be
                // updated according to the tile attributes field
                if self.gb_mode == GameBoyMode::Cgb && !self.dmg_compat {
                    tile_attr = &bg_map_attrs[row_offset + line_offset];
                    palette = &self.palettes_color_bg[tile_attr.palette as usize];
                    xflip = tile_attr.xflip;
                    yflip = tile_attr.yflip;
                    priority = tile_attr.priority;
                    tile_index += tile_attr.vram_bank as usize * TILE_COUNT_DMG;
                }

                // obtains the reference to the new tile in drawing
                tile = &self.tiles[tile_index];
            }

            // increments the color offset by one, representing
            // the drawing of one pixel
            color_offset += 1;

            // increments the offset of the frame buffer by the
            // size of an RGB pixel (which is 3 bytes)
            frame_offset += RGB_SIZE;
        }
    }

    fn render_map_dmg(&mut self, map: bool, scx: u8, scy: u8, wx: u8, wy: u8, ld: u8) {
        // in case the target window Y position has not yet been reached
        // then there's nothing to be done, returns control flow immediately
        if self.ly < wy {
            return;
        }

        // obtains the base address of the background map using the bg map flag
        // that control which background map is going to be used
        let map_offset: usize = if map { 0x1c00 } else { 0x1800 };

        // calculates the map row index for the tile by using the current line
        // index and the DY (scroll Y) divided by 8 (as the tiles are 8x8 pixels),
        // on top of that ensures that the result is modulus 32 meaning that the
        // drawing wraps around the Y axis
        let row_index = (((ld as usize + scy as usize) & 0xff) >> 3) % 32;

        // calculates the map offset by the row offset multiplied by the number
        // of tiles in each row (32)
        let row_offset = row_index * 32;

        // calculates the sprite line offset by using the SCX register
        // shifted by 3 meaning that the tiles are 8x8
        let mut line_offset = (scx >> 3) as usize;

        // calculates the index of the initial tile in drawing,
        // if the tile data set in use is #1, the indexes are
        // signed, then calculates a real tile offset
        let mut tile_index = self.vram[map_offset + row_offset + line_offset] as usize;
        if !self.bg_tile && tile_index < 128 {
            tile_index += 256;
        }

        // obtains the reference to the tile that is going to be drawn
        let mut tile = &self.tiles[tile_index];

        // calculates the offset that is going to be used in the update of the color buffer
        // which stores Game Boy colors from 0 to 3
        let mut color_offset = self.ly as usize * DISPLAY_WIDTH;

        // obtains the current integer value (raw) for the background palette
        // this is going to be used for shade index value computation (DMG only)
        let palette_v = self.palettes[0];

        // calculates both the current Y and X positions within the tiles
        // using the bitwise and operation as an effective modulus 8
        let y = (ld as usize + scy as usize) & 0x07;
        let mut x = (scx & 0x07) as usize;

        // calculates the initial tile X position in drawing, doing this
        // allows us to position the background map properly in the display
        let initial_index = max(wx as i16 - 7, 0) as usize;
        color_offset += initial_index;

        // iterates over all the pixels in the current line of the display
        // to draw the background map, note that the initial index is used
        // to skip the drawing of the tiles that are not visible (WX)
        for _ in initial_index..DISPLAY_WIDTH {
            // obtains the current pixel data from the tile
            let pixel = tile.get(x, y);

            // updates the pixel in the color buffer, which stores
            // the raw pixel color information (unmapped) and then
            // updates the shade buffer with the shade index
            self.color_buffer[color_offset] = pixel;
            self.shade_buffer[color_offset] = (palette_v >> (pixel * 2)) & 3;

            // increments the current tile X position in drawing
            x += 1;

            // in case the end of tile width has been reached then
            // a new tile must be retrieved for rendering
            if x == TILE_WIDTH {
                // resets the tile X position to the base value
                // as a new tile is going to be drawn
                x = 0;

                // calculates the new line tile offset making sure that
                // the maximum of 32 is not overflown
                line_offset = (line_offset + 1) % 32;

                // calculates the tile index and makes sure the value
                // takes into consideration the bg tile value
                tile_index = self.vram[map_offset + row_offset + line_offset] as usize;
                if !self.bg_tile && tile_index < 128 {
                    tile_index += 256;
                }

                // obtains the reference to the new tile in drawing
                tile = &self.tiles[tile_index];
            }

            // increments the color offset by one, representing
            // the drawing of one pixel
            color_offset += 1;
        }
    }

    fn render_objects(&mut self) {
        // the mode in which the object priority should be computed
        // if true this means that the X coordinate priority mode will
        // be used otherwise the object priority will be defined according
        // to the object's index in the OAM memory, notice that this
        // control of priority is only present in the CGB and to be able
        // to offer retro-compatibility with DMG
        let obj_priority_mode = self.gb_mode != GameBoyMode::Cgb || self.obj_priority;

        // creates a local counter object to count the total number
        // of object that were drawn in the current line, this will
        // be used for instance to limit the total number of objects
        // to 10 per line (Game Boy limitation)
        let mut draw_count = 0u8;

        // allocates the buffer that is going to be used to determine
        // drawing priority for overlapping pixels between different
        // objects, in MBR mode the object that has the smallest X
        // coordinate takes priority in drawing the pixel
        let mut index_buffer = [-256i16; DISPLAY_WIDTH];

        // determines if the object should always be placed over the
        // possible background, this is only required for CGB mode
        let always_over = if self.gb_mode == GameBoyMode::Cgb && !self.dmg_compat {
            !self.switch_bg
        } else {
            false
        };

        // iterates over the complete set of available object to check
        // the ones that require drawing and draws them
        for index in 0..OBJ_COUNT {
            // in case the limit on the number of objects to be draw per
            // line has been reached breaks the loop avoiding more draws
            if draw_count == 10 {
                break;
            }

            // obtains the meta data of the object that is currently
            // under iteration to be checked for drawing
            let obj = &self.obj_data[index];

            let obj_height = if self.obj_size {
                TILE_DOUBLE_HEIGHT
            } else {
                TILE_HEIGHT
            };

            // verifies if the sprite is currently located at the
            // current line that is going to be drawn and skips it
            // in case it's not
            let is_contained =
                (obj.y <= self.ly as i16) && ((obj.y + obj_height as i16) > self.ly as i16);
            if !is_contained {
                continue;
            }

            let (palette, palette_index) = if self.gb_mode == GameBoyMode::Cgb {
                if self.dmg_compat {
                    if obj.palette == 0 {
                        (&self.palette_obj_0, 0_u8)
                    } else if obj.palette == 1 {
                        (&self.palette_obj_1, 0_u8)
                    } else {
                        panic_gb!("Invalid object palette: {:02x}", obj.palette);
                    }
                } else {
                    (&self.palettes_color_obj[obj.palette_cgb as usize], 0_u8)
                }
            } else if obj.palette == 0 {
                (&self.palette_obj_0, 1_u8)
            } else if obj.palette == 1 {
                (&self.palette_obj_1, 2_u8)
            } else {
                panic_gb!("Invalid object palette: {:02x}", obj.palette);
            };

            // obtains the current integer value (raw) for the palette in use
            // this is going to be used for shade index value computation (DMG only)
            let palette_v = self.palettes[palette_index as usize];

            // calculates the offset in the color buffer (raw color information
            // from 0 to 3) for the sprit that is going to be drawn, this value
            // is kept as a signed integer to allow proper negative number math
            let mut color_offset = self.ly as i32 * DISPLAY_WIDTH as i32 + obj.x as i32;

            // calculates the offset in the frame buffer for the sprite
            // that is going to be drawn, this is going to be the starting
            // point for the draw operation to be performed
            let mut frame_offset =
                (self.ly as i32 * DISPLAY_WIDTH as i32 + obj.x as i32) * RGB_SIZE as i32;

            // the relative title offset should range from 0 to 7 in 8x8
            // objects and from 0 to 15 in 8x16 objects
            let mut tile_offset = self.ly as i16 - obj.y;

            // in case we're flipping the object we must recompute the
            // tile offset as an inverted value using the object's height
            if obj.yflip {
                tile_offset = obj_height as i16 - tile_offset - 1;
            }

            // saves some space for the reference to the tile that
            // is going to be used in the current operation
            let tile: &Tile;

            // "calculates" the index offset that is going to be applied
            // to the tile index to retrieve the proper tile taking into
            // consideration the VRAM in which the tile is stored
            let tile_bank_offset = if self.dmg_compat {
                0
            } else {
                obj.tile_bank as usize * TILE_COUNT_DMG
            };

            // in case we're facing a 8x16 object then we must
            // differentiate between the handling of the top tile
            // and the bottom tile through bitwise manipulation
            // of the tile index
            if self.obj_size {
                if tile_offset < 8 {
                    let tile_index = (obj.tile as usize & 0xfe) + tile_bank_offset;
                    tile = &self.tiles[tile_index];
                } else {
                    let tile_index = (obj.tile as usize | 0x01) + tile_bank_offset;
                    tile = &self.tiles[tile_index];
                    tile_offset -= 8;
                }
            }
            // otherwise we're facing a 8x8 sprite and we should grab
            // the tile directly from the object's tile index
            else {
                let tile_index = obj.tile as usize + tile_bank_offset;
                tile = &self.tiles[tile_index];
            }

            let tile_row = tile.get_row(tile_offset as usize);

            // determines if the object should always be placed over the
            // previously placed background or window pixels
            let obj_over = always_over || !obj.bg_over;

            for tile_x in 0..TILE_WIDTH {
                let x = obj.x + tile_x as i16;
                let is_contained = (x >= 0) && (x < DISPLAY_WIDTH as i16);
                if is_contained {
                    // the object is only considered visible if no background or
                    // window should be drawn over or if the underlying pixel
                    // is transparent (zero value) meaning there's no background
                    // or window for the provided pixel
                    let mut is_visible = obj_over || self.color_buffer[color_offset as usize] == 0;

                    // additionally (in CCG mode) the object is only considered to
                    // be visible if the priority buffer is not set for the current
                    // pixel, this means that the background is capturing priority
                    // by having the BG-to-OAM priority bit set in the bg map attributes
                    is_visible &= always_over || !self.priority_buffer[color_offset as usize];

                    // determines if the current pixel has priority over a possible
                    // one that has been drawn by a previous object, this happens
                    // in case the current object has a small X coordinate according
                    // to the MBR algorithm
                    let has_priority = index_buffer[x as usize] == -256
                        || (obj_priority_mode && obj.x < index_buffer[x as usize]);

                    let pixel = tile_row[if obj.xflip {
                        TILE_WIDTH_I - tile_x
                    } else {
                        tile_x
                    }];
                    if is_visible && has_priority && pixel != 0 {
                        // marks the current pixel in iteration as "owned"
                        // by the object with the defined X base position,
                        // to be used in priority calculus
                        index_buffer[x as usize] = obj.x;

                        // updates the pixel in the color buffer, which stores
                        // the raw pixel color information (unmapped) and then
                        // updates the shade buffer with the shade index
                        self.color_buffer[color_offset as usize] = pixel;
                        self.shade_buffer[color_offset as usize] = (palette_v >> (pixel * 2)) & 3;

                        // re-maps the pixel according to the object palette
                        // and then sets the color pixel in the frame buffer
                        let color = &palette[pixel as usize];
                        self.frame_buffer[frame_offset as usize] = color[0];
                        self.frame_buffer[frame_offset as usize + 1] = color[1];
                        self.frame_buffer[frame_offset as usize + 2] = color[2];
                    }
                }

                // increment the color offset by one as this represents
                // the advance of one color pixel
                color_offset += 1;

                // increments the offset of the frame buffer by the
                // size of an RGB pixel (which is 3 bytes)
                frame_offset += RGB_SIZE as i32;
            }

            // increments the counter so that we're able to keep
            // track on the number of object drawn
            draw_count += 1;
        }
    }

    /// Runs an update operation on the LCD STAT interrupt meaning
    /// that the flag that control will be updated in case the conditions
    /// required for the LCD STAT interrupt to be triggered are met.
    fn update_stat(&mut self) {
        self.int_stat = self.stat_level();
    }

    /// Obtains the current level of the LCD STAT interrupt by
    /// checking the current PPU state in various sections.
    fn stat_level(&self) -> bool {
        self.stat_lyc && self.lyc == self.ly
            || self.stat_oam && self.mode == PpuMode::OamRead
            || self.stat_vblank && self.mode == PpuMode::VBlank
            || self.stat_hblank && self.mode == PpuMode::HBlank
    }

    /// Computes the values for all of the palettes, this method
    /// is useful to "flush" color computation whenever the base
    /// palette colors are changed.
    /// Notice that this is only applicable to the DMG running mode
    /// either in the original DMG or in CGB with DMG compatibility.
    fn compute_palettes(&mut self) {
        if self.dmg_compat {
            Self::compute_palette(
                &mut self.palette_bg,
                &self.palettes_color_bg[0],
                self.palettes[0],
            );
            Self::compute_palette(
                &mut self.palette_obj_0,
                &self.palettes_color_obj[0],
                self.palettes[1],
            );
            Self::compute_palette(
                &mut self.palette_obj_1,
                &self.palettes_color_obj[1],
                self.palettes[2],
            );
        } else {
            // re-computes the complete set of palettes according to
            // the currently set palette colors (that may have changed)
            Self::compute_palette(&mut self.palette_bg, &self.palette_colors, self.palettes[0]);
            Self::compute_palette(
                &mut self.palette_obj_0,
                &self.palette_colors,
                self.palettes[1],
            );
            Self::compute_palette(
                &mut self.palette_obj_1,
                &self.palette_colors,
                self.palettes[2],
            );
        }

        // clears the frame buffer to allow the new background
        // color to be used
        self.clear_frame_buffer();
    }

    /// Static method used for the base logic of computation of RGB
    /// based palettes from the internal Game Boy color indexes.
    /// This method should be called whenever the palette indexes
    /// are changed.
    fn compute_palette(palette: &mut Palette, palette_colors: &Palette, value: u8) {
        for (index, palette_item) in palette.iter_mut().enumerate() {
            let color_index: usize = (value as usize >> (index * 2)) & 3;
            match color_index {
                0..=3 => *palette_item = palette_colors[color_index],
                color_index => panic_gb!("Invalid palette color index {:04x}", color_index),
            }
        }
    }

    /// Static method that computes an RGB888 color palette ready to
    /// be used for frame buffer operations from 4 (colors) x 2 bytes (RGB555)
    /// that represent an RGB555 set of colors. This method should be
    /// used for CGB mode where colors are represented using RGB555.
    fn compute_palette_color(
        palette: &mut Palette,
        palette_color: &[u8; 64],
        palette_index: u8,
        color_index: u8,
    ) {
        let palette_offset = (palette_index * 4 * 2) as usize;
        let color_offset = (color_index * 2) as usize;
        palette[color_index as usize] = rgb555_to_rgb888(
            palette_color[palette_offset + color_offset],
            palette_color[palette_offset + color_offset + 1],
        );
    }

    /// Re-computes the complete set of CGB only color palettes using the
    /// raw `palettes_color` information and computing the `Palette` structure
    /// for both background and objects palettes.
    fn compute_palettes_color(
        palettes: &mut [&mut [Palette; 8]; 2],
        palettes_color: &[[u8; 64]; 2],
    ) {
        for index in 0..2 {
            let palette = &mut palettes[index];
            let palette_color = &palettes_color[index];
            for palette_index in 0..palette.len() {
                Self::compute_color_palette(
                    &mut palette[palette_index],
                    &palette_color[palette_index * 8..(palette_index + 1) * 8]
                        .try_into()
                        .unwrap(),
                );
            }
        }
    }

    /// Computes an individual structured CGB color palette from 8 raw bytes
    /// coming from the raw `palette_color` information, this 8 bytes should
    /// represent the 4 colors of the palette in the RGB555 format.
    fn compute_color_palette(palette: &mut Palette, palette_color: &[u8; 8]) {
        for color_index in 0..palette.len() {
            palette[color_index] = rgb555_to_rgb888(
                palette_color[color_index * 2],
                palette_color[color_index * 2 + 1],
            );
        }
    }
}

impl BusComponent for Ppu {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new(
            GameBoyMode::Dmg,
            Arc::new(Mutex::new(GameBoyConfig::default())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Ppu;

    #[test]
    fn test_update_tile_simple() {
        let mut ppu = Ppu::default();
        ppu.vram[0x0000] = 0xff;
        ppu.vram[0x0001] = 0xff;

        let result = ppu.tiles()[0].get(0, 0);
        assert_eq!(result, 0);

        ppu.update_tile(0x8000, 0x00);
        let result = ppu.tiles()[0].get(0, 0);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_update_tile_upper() {
        let mut ppu = Ppu::default();
        ppu.vram[0x1000] = 0xff;
        ppu.vram[0x1001] = 0xff;

        let result = ppu.tiles()[256].get(0, 0);
        assert_eq!(result, 0);

        ppu.update_tile(0x9000, 0x00);
        let result = ppu.tiles()[256].get(0, 0);
        assert_eq!(result, 3);
    }
}
