//! GBA PPU (Picture Processing Unit) with scanline-based rendering.
//!
//! Supports video modes 0-2 with text and affine background layers,
//! sprite rendering, priority compositing, and alpha blending.

use crate::gba::consts::{
    CYCLES_PER_SCANLINE, DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE, TOTAL_LINES,
    VISIBLE_DOTS,
};

/// number of OBJ entries in OAM
pub const OBJ_COUNT: usize = 128;

/// maximum number of BG layers
pub const BG_COUNT: usize = 4;

/// pixel type: index into the line buffer priority system
#[allow(dead_code)]
#[derive(Clone, Copy, Default)]
struct PixelEntry {
    color: u16,
    priority: u8,
    is_transparent: bool,
}

/// PPU rendering mode derived from DISPCNT bits 0-2
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VideoMode {
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
    Mode4 = 4,
    Mode5 = 5,
}

impl VideoMode {
    pub fn from_u16(value: u16) -> Self {
        match value & 0x07 {
            0 => VideoMode::Mode0,
            1 => VideoMode::Mode1,
            2 => VideoMode::Mode2,
            3 => VideoMode::Mode3,
            4 => VideoMode::Mode4,
            5 => VideoMode::Mode5,
            _ => VideoMode::Mode0,
        }
    }
}

pub struct GbaPpu {
    /// current scanline (VCOUNT, 0-227)
    vcount: u16,

    /// dot counter within the current scanline
    dot: u32,

    /// display control register (DISPCNT, 0x0400_0000)
    dispcnt: u16,

    /// display status register (DISPSTAT, 0x0400_0004)
    dispstat: u16,

    /// BG control registers (BG0CNT-BG3CNT)
    bgcnt: [u16; BG_COUNT],

    /// BG horizontal scroll offsets
    bg_hofs: [u16; BG_COUNT],

    /// BG vertical scroll offsets
    bg_vofs: [u16; BG_COUNT],

    /// BG2/BG3 affine parameters (PA, PB, PC, PD)
    bg_pa: [i16; 2],
    bg_pb: [i16; 2],
    bg_pc: [i16; 2],
    bg_pd: [i16; 2],

    /// BG2/BG3 reference point X (28-bit signed, internal latched)
    bg_ref_x: [i32; 2],
    bg_ref_y: [i32; 2],

    /// BG2/BG3 reference point X (written values)
    bg_ref_x_write: [i32; 2],
    bg_ref_y_write: [i32; 2],

    /// window registers
    winh: [u16; 2],
    winv: [u16; 2],
    winin: u16,
    winout: u16,

    /// blend control registers
    bldcnt: u16,
    bldalpha: u16,
    bldy: u16,

    /// mosaic register
    mosaic: u16,

    /// frame buffer (RGB888, 240x160x3 bytes)
    frame_buffer: Vec<u8>,

    /// frame counter for tracking frame completion
    frame: u64,

    /// interrupt flags
    int_vblank: bool,
    int_hblank: bool,
    int_vcount: bool,
}

impl GbaPpu {
    pub fn new() -> Self {
        Self {
            vcount: 0,
            dot: 0,
            dispcnt: 0,
            dispstat: 0,
            bgcnt: [0; BG_COUNT],
            bg_hofs: [0; BG_COUNT],
            bg_vofs: [0; BG_COUNT],
            bg_pa: [0x100, 0x100],
            bg_pb: [0; 2],
            bg_pc: [0; 2],
            bg_pd: [0x100, 0x100],
            bg_ref_x: [0; 2],
            bg_ref_y: [0; 2],
            bg_ref_x_write: [0; 2],
            bg_ref_y_write: [0; 2],
            winh: [0; 2],
            winv: [0; 2],
            winin: 0,
            winout: 0,
            bldcnt: 0,
            bldalpha: 0,
            bldy: 0,
            mosaic: 0,
            frame_buffer: vec![0u8; FRAME_BUFFER_SIZE],
            frame: 0,
            int_vblank: false,
            int_hblank: false,
            int_vcount: false,
        }
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn vcount(&self) -> u16 {
        self.vcount
    }

    pub fn dispcnt(&self) -> u16 {
        self.dispcnt
    }

    pub fn set_dispcnt(&mut self, value: u16) {
        self.dispcnt = value;
    }

    pub fn dispstat(&self) -> u16 {
        let mut value = self.dispstat & 0xFF38;
        // set vblank flag (bit 0)
        if self.vcount >= DISPLAY_HEIGHT as u16 {
            value |= 1;
        }
        // set hblank flag (bit 1)
        if self.dot >= VISIBLE_DOTS {
            value |= 2;
        }
        // set vcount match flag (bit 2)
        let lyc = (self.dispstat >> 8) & 0xFF;
        if self.vcount == lyc {
            value |= 4;
        }
        value
    }

    pub fn set_dispstat(&mut self, value: u16) {
        // only bits 3-15 are writable (bits 0-2 are read-only status)
        self.dispstat = (self.dispstat & 0x07) | (value & 0xFFF8);
    }

    pub fn bgcnt(&self, index: usize) -> u16 {
        self.bgcnt[index]
    }

    pub fn set_bgcnt(&mut self, index: usize, value: u16) {
        self.bgcnt[index] = value;
    }

    pub fn set_bg_hofs(&mut self, index: usize, value: u16) {
        self.bg_hofs[index] = value & 0x01FF;
    }

    pub fn set_bg_vofs(&mut self, index: usize, value: u16) {
        self.bg_vofs[index] = value & 0x01FF;
    }

    pub fn set_bg_pa(&mut self, index: usize, value: u16) {
        self.bg_pa[index] = value as i16;
    }

    pub fn set_bg_pb(&mut self, index: usize, value: u16) {
        self.bg_pb[index] = value as i16;
    }

    pub fn set_bg_pc(&mut self, index: usize, value: u16) {
        self.bg_pc[index] = value as i16;
    }

    pub fn set_bg_pd(&mut self, index: usize, value: u16) {
        self.bg_pd[index] = value as i16;
    }

    pub fn set_bg_ref_x(&mut self, index: usize, value: u32) {
        // 28-bit signed, sign-extend from bit 27
        let signed = ((value as i32) << 4) >> 4;
        self.bg_ref_x_write[index] = signed;
        self.bg_ref_x[index] = signed;
    }

    pub fn set_bg_ref_y(&mut self, index: usize, value: u32) {
        let signed = ((value as i32) << 4) >> 4;
        self.bg_ref_y_write[index] = signed;
        self.bg_ref_y[index] = signed;
    }

    pub fn set_winh(&mut self, index: usize, value: u16) {
        self.winh[index] = value;
    }

    pub fn set_winv(&mut self, index: usize, value: u16) {
        self.winv[index] = value;
    }

    pub fn set_winin(&mut self, value: u16) {
        self.winin = value;
    }

    pub fn set_winout(&mut self, value: u16) {
        self.winout = value;
    }

    pub fn set_bldcnt(&mut self, value: u16) {
        self.bldcnt = value;
    }

    pub fn set_bldalpha(&mut self, value: u16) {
        self.bldalpha = value;
    }

    pub fn set_bldy(&mut self, value: u16) {
        self.bldy = value;
    }

    pub fn set_mosaic(&mut self, value: u16) {
        self.mosaic = value;
    }

    pub fn int_vblank(&self) -> bool {
        self.int_vblank
    }

    pub fn int_hblank(&self) -> bool {
        self.int_hblank
    }

    pub fn int_vcount(&self) -> bool {
        self.int_vcount
    }

    pub fn ack_vblank(&mut self) {
        self.int_vblank = false;
    }

    pub fn ack_hblank(&mut self) {
        self.int_hblank = false;
    }

    pub fn ack_vcount(&mut self) {
        self.int_vcount = false;
    }

    pub fn video_mode(&self) -> VideoMode {
        VideoMode::from_u16(self.dispcnt)
    }

    /// clocks the PPU by the given number of CPU cycles.
    /// returns flags: bit 0 = entered hblank, bit 1 = entered vblank
    pub fn clock(&mut self, cycles: u32, vram: &[u8], palette: &[u8], oam: &[u8]) -> u8 {
        let mut events = 0u8;

        self.dot += cycles;

        // handle hblank transition
        if self.dot >= VISIBLE_DOTS && self.dot - cycles < VISIBLE_DOTS {
            // entering hblank
            if self.vcount < DISPLAY_HEIGHT as u16 {
                self.render_scanline(vram, palette, oam);
            }

            // check hblank IRQ
            if self.dispstat & (1 << 4) != 0 {
                self.int_hblank = true;
                events |= 1;
            }
        }

        // handle end of scanline
        if self.dot >= CYCLES_PER_SCANLINE {
            self.dot -= CYCLES_PER_SCANLINE;
            self.vcount += 1;

            if self.vcount == DISPLAY_HEIGHT as u16 {
                // entering vblank
                self.frame += 1;

                // latch affine reference points
                self.bg_ref_x[0] = self.bg_ref_x_write[0];
                self.bg_ref_y[0] = self.bg_ref_y_write[0];
                self.bg_ref_x[1] = self.bg_ref_x_write[1];
                self.bg_ref_y[1] = self.bg_ref_y_write[1];

                if self.dispstat & (1 << 3) != 0 {
                    self.int_vblank = true;
                }
                events |= 2;
            } else if self.vcount >= TOTAL_LINES as u16 {
                self.vcount = 0;
            }

            // check vcount match
            let lyc = (self.dispstat >> 8) & 0xFF;
            if self.vcount == lyc && self.dispstat & (1 << 5) != 0 {
                self.int_vcount = true;
            }

            // update internal affine reference points after each visible scanline
            if self.vcount < DISPLAY_HEIGHT as u16 {
                self.bg_ref_x[0] = self.bg_ref_x[0].wrapping_add(self.bg_pb[0] as i32);
                self.bg_ref_y[0] = self.bg_ref_y[0].wrapping_add(self.bg_pd[0] as i32);
                self.bg_ref_x[1] = self.bg_ref_x[1].wrapping_add(self.bg_pb[1] as i32);
                self.bg_ref_y[1] = self.bg_ref_y[1].wrapping_add(self.bg_pd[1] as i32);
            }
        }

        events
    }

    /// renders a single scanline into the frame buffer
    fn render_scanline(&mut self, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let line = self.vcount as usize;
        if line >= DISPLAY_HEIGHT {
            return;
        }

        // check for forced blank
        if self.dispcnt & (1 << 7) != 0 {
            let offset = line * DISPLAY_WIDTH * 3;
            for i in 0..DISPLAY_WIDTH * 3 {
                self.frame_buffer[offset + i] = 0xFF;
            }
            return;
        }

        let mode = self.video_mode();
        match mode {
            VideoMode::Mode0 => self.render_mode0(line, vram, palette, oam),
            VideoMode::Mode1 => self.render_mode1(line, vram, palette, oam),
            VideoMode::Mode2 => self.render_mode2(line, vram, palette, oam),
            VideoMode::Mode3 => self.render_mode3(line, vram),
            VideoMode::Mode4 => self.render_mode4(line, vram, palette),
            VideoMode::Mode5 => self.render_mode5(line, vram),
        }
    }

    /// mode 0: 4 text BG layers
    fn render_mode0(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut line_buffer = [0u16; DISPLAY_WIDTH];
        let mut priority_buffer = [4u8; DISPLAY_WIDTH];

        // render BGs in reverse priority order (lowest priority first)
        for bg_index in (0..4).rev() {
            if self.dispcnt & (1 << (8 + bg_index)) == 0 {
                continue;
            }
            let bg_priority = (self.bgcnt[bg_index] & 0x03) as u8;
            self.render_text_bg(
                bg_index,
                line,
                vram,
                palette,
                &mut line_buffer,
                &mut priority_buffer,
                bg_priority,
            );
        }

        // render sprites on top
        if self.dispcnt & (1 << 12) != 0 {
            self.render_sprites(
                line,
                vram,
                palette,
                oam,
                &mut line_buffer,
                &mut priority_buffer,
            );
        }

        // write to frame buffer
        self.write_line_buffer(line, &line_buffer);
    }

    /// mode 1: 2 text BGs + 1 affine BG (BG2)
    fn render_mode1(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut line_buffer = [0u16; DISPLAY_WIDTH];
        let mut priority_buffer = [4u8; DISPLAY_WIDTH];

        // BG2 is affine
        if self.dispcnt & (1 << 10) != 0 {
            let bg_priority = (self.bgcnt[2] & 0x03) as u8;
            self.render_affine_bg(
                0,
                2,
                line,
                vram,
                palette,
                &mut line_buffer,
                &mut priority_buffer,
                bg_priority,
            );
        }

        // BG0 and BG1 are text
        for bg_index in (0..2).rev() {
            if self.dispcnt & (1 << (8 + bg_index)) == 0 {
                continue;
            }
            let bg_priority = (self.bgcnt[bg_index] & 0x03) as u8;
            self.render_text_bg(
                bg_index,
                line,
                vram,
                palette,
                &mut line_buffer,
                &mut priority_buffer,
                bg_priority,
            );
        }

        if self.dispcnt & (1 << 12) != 0 {
            self.render_sprites(
                line,
                vram,
                palette,
                oam,
                &mut line_buffer,
                &mut priority_buffer,
            );
        }

        self.write_line_buffer(line, &line_buffer);
    }

    /// mode 2: 2 affine BGs (BG2, BG3)
    fn render_mode2(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut line_buffer = [0u16; DISPLAY_WIDTH];
        let mut priority_buffer = [4u8; DISPLAY_WIDTH];

        // BG3 (lower priority first)
        if self.dispcnt & (1 << 11) != 0 {
            let bg_priority = (self.bgcnt[3] & 0x03) as u8;
            self.render_affine_bg(
                1,
                3,
                line,
                vram,
                palette,
                &mut line_buffer,
                &mut priority_buffer,
                bg_priority,
            );
        }

        // BG2
        if self.dispcnt & (1 << 10) != 0 {
            let bg_priority = (self.bgcnt[2] & 0x03) as u8;
            self.render_affine_bg(
                0,
                2,
                line,
                vram,
                palette,
                &mut line_buffer,
                &mut priority_buffer,
                bg_priority,
            );
        }

        if self.dispcnt & (1 << 12) != 0 {
            self.render_sprites(
                line,
                vram,
                palette,
                oam,
                &mut line_buffer,
                &mut priority_buffer,
            );
        }

        self.write_line_buffer(line, &line_buffer);
    }

    /// mode 3: single 240x160 bitmap, 15-bit direct color
    fn render_mode3(&mut self, line: usize, vram: &[u8]) {
        let offset = line * DISPLAY_WIDTH * 3;
        for x in 0..DISPLAY_WIDTH {
            let vram_offset = (line * DISPLAY_WIDTH + x) * 2;
            if vram_offset + 1 < vram.len() {
                let color = (vram[vram_offset] as u16) | ((vram[vram_offset + 1] as u16) << 8);
                let (r, g, b) = bgr555_to_rgb888(color);
                self.frame_buffer[offset + x * 3] = r;
                self.frame_buffer[offset + x * 3 + 1] = g;
                self.frame_buffer[offset + x * 3 + 2] = b;
            }
        }
    }

    /// mode 4: single 240x160 bitmap, 8-bit palette indexed, double buffered
    fn render_mode4(&mut self, line: usize, vram: &[u8], palette: &[u8]) {
        let page_offset = if self.dispcnt & (1 << 4) != 0 {
            0xA000
        } else {
            0
        };
        let offset = line * DISPLAY_WIDTH * 3;
        for x in 0..DISPLAY_WIDTH {
            let vram_offset = page_offset + line * DISPLAY_WIDTH + x;
            if vram_offset < vram.len() {
                let index = vram[vram_offset] as usize;
                let color = self.read_palette_color(palette, index);
                let (r, g, b) = bgr555_to_rgb888(color);
                self.frame_buffer[offset + x * 3] = r;
                self.frame_buffer[offset + x * 3 + 1] = g;
                self.frame_buffer[offset + x * 3 + 2] = b;
            }
        }
    }

    /// mode 5: 160x128 bitmap, 15-bit direct color, double buffered
    fn render_mode5(&mut self, line: usize, vram: &[u8]) {
        let page_offset: usize = if self.dispcnt & (1 << 4) != 0 {
            0xA000
        } else {
            0
        };
        let offset = line * DISPLAY_WIDTH * 3;

        if line >= 128 {
            for i in 0..DISPLAY_WIDTH * 3 {
                self.frame_buffer[offset + i] = 0;
            }
            return;
        }

        for x in 0..DISPLAY_WIDTH {
            if x >= 160 {
                self.frame_buffer[offset + x * 3] = 0;
                self.frame_buffer[offset + x * 3 + 1] = 0;
                self.frame_buffer[offset + x * 3 + 2] = 0;
                continue;
            }
            let vram_offset = page_offset + (line * 160 + x) * 2;
            if vram_offset + 1 < vram.len() {
                let color = (vram[vram_offset] as u16) | ((vram[vram_offset + 1] as u16) << 8);
                let (r, g, b) = bgr555_to_rgb888(color);
                self.frame_buffer[offset + x * 3] = r;
                self.frame_buffer[offset + x * 3 + 1] = g;
                self.frame_buffer[offset + x * 3 + 2] = b;
            }
        }
    }

    /// renders a text background layer into the line buffer
    #[allow(clippy::too_many_arguments)]
    fn render_text_bg(
        &self,
        bg_index: usize,
        line: usize,
        vram: &[u8],
        palette: &[u8],
        line_buffer: &mut [u16; DISPLAY_WIDTH],
        priority_buffer: &mut [u8; DISPLAY_WIDTH],
        bg_priority: u8,
    ) {
        let cnt = self.bgcnt[bg_index];
        let char_base = ((cnt >> 2) & 0x03) as usize * 0x4000;
        let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
        let is_8bpp = cnt & (1 << 7) != 0;
        let screen_size = (cnt >> 14) & 0x03;

        let (map_width, map_height) = match screen_size {
            0 => (256u32, 256u32),
            1 => (512, 256),
            2 => (256, 512),
            3 => (512, 512),
            _ => (256, 256),
        };

        let scroll_x = self.bg_hofs[bg_index] as u32;
        let scroll_y = self.bg_vofs[bg_index] as u32;
        let y = (line as u32 + scroll_y) % map_height;

        for x_screen in 0..DISPLAY_WIDTH {
            let x = (x_screen as u32 + scroll_x) % map_width;

            // determine which screen block
            let screen_x = x / 256;
            let screen_y = y / 256;
            let screen_offset = match screen_size {
                0 => 0,
                1 => screen_x as usize,
                2 => screen_y as usize,
                3 => (screen_x + screen_y * 2) as usize,
                _ => 0,
            };

            let tile_x = (x % 256) / 8;
            let tile_y = (y % 256) / 8;
            let map_entry_offset =
                screen_base + screen_offset * 0x800 + (tile_y * 32 + tile_x) as usize * 2;

            if map_entry_offset + 1 >= vram.len() {
                continue;
            }

            let map_entry =
                (vram[map_entry_offset] as u16) | ((vram[map_entry_offset + 1] as u16) << 8);
            let tile_number = (map_entry & 0x03FF) as usize;
            let h_flip = map_entry & (1 << 10) != 0;
            let v_flip = map_entry & (1 << 11) != 0;
            let palette_bank = ((map_entry >> 12) & 0x0F) as usize;

            let pixel_y = if v_flip {
                7 - (y % 8) as usize
            } else {
                (y % 8) as usize
            };
            let pixel_x = if h_flip {
                7 - (x % 8) as usize
            } else {
                (x % 8) as usize
            };

            let color_index = if is_8bpp {
                let tile_offset = char_base + tile_number * 64 + pixel_y * 8 + pixel_x;
                if tile_offset < vram.len() {
                    vram[tile_offset] as usize
                } else {
                    0
                }
            } else {
                let tile_offset = char_base + tile_number * 32 + pixel_y * 4 + pixel_x / 2;
                if tile_offset < vram.len() {
                    let byte = vram[tile_offset];
                    if pixel_x & 1 == 0 {
                        (byte & 0x0F) as usize
                    } else {
                        ((byte >> 4) & 0x0F) as usize
                    }
                } else {
                    0
                }
            };

            // skip transparent pixels (color index 0)
            if color_index == 0 {
                continue;
            }

            // only draw if this pixel has equal or higher priority
            if bg_priority <= priority_buffer[x_screen] {
                let color = if is_8bpp {
                    self.read_palette_color(palette, color_index)
                } else {
                    self.read_palette_color(palette, palette_bank * 16 + color_index)
                };
                line_buffer[x_screen] = color;
                priority_buffer[x_screen] = bg_priority;
            }
        }
    }

    /// renders an affine background layer into the line buffer
    #[allow(clippy::too_many_arguments)]
    fn render_affine_bg(
        &self,
        affine_index: usize,
        bg_index: usize,
        _line: usize,
        vram: &[u8],
        palette: &[u8],
        line_buffer: &mut [u16; DISPLAY_WIDTH],
        priority_buffer: &mut [u8; DISPLAY_WIDTH],
        bg_priority: u8,
    ) {
        let cnt = self.bgcnt[bg_index];
        let char_base = ((cnt >> 2) & 0x03) as usize * 0x4000;
        let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
        let wraparound = cnt & (1 << 13) != 0;
        let screen_size = (cnt >> 14) & 0x03;

        let map_size = match screen_size {
            0 => 128,
            1 => 256,
            2 => 512,
            3 => 1024,
            _ => 128,
        };
        let tiles_per_row = map_size / 8;

        let mut ref_x = self.bg_ref_x[affine_index];
        let mut ref_y = self.bg_ref_y[affine_index];
        let pa = self.bg_pa[affine_index] as i32;
        let pc = self.bg_pc[affine_index] as i32;

        for x_screen in 0..DISPLAY_WIDTH {
            // convert fixed-point (8.8) to pixel coordinates
            let tex_x = ref_x >> 8;
            let tex_y = ref_y >> 8;

            ref_x += pa;
            ref_y += pc;

            let (tx, ty) = if wraparound {
                (
                    ((tex_x % map_size as i32) + map_size as i32) as usize % map_size,
                    ((tex_y % map_size as i32) + map_size as i32) as usize % map_size,
                )
            } else {
                if tex_x < 0 || tex_y < 0 || tex_x >= map_size as i32 || tex_y >= map_size as i32 {
                    continue;
                }
                (tex_x as usize, tex_y as usize)
            };

            let tile_x = tx / 8;
            let tile_y = ty / 8;
            let map_offset = screen_base + tile_y * tiles_per_row + tile_x;

            if map_offset >= vram.len() {
                continue;
            }

            let tile_number = vram[map_offset] as usize;
            let pixel_x = tx % 8;
            let pixel_y = ty % 8;

            // affine BGs are always 8bpp
            let tile_offset = char_base + tile_number * 64 + pixel_y * 8 + pixel_x;
            if tile_offset >= vram.len() {
                continue;
            }

            let color_index = vram[tile_offset] as usize;
            if color_index == 0 {
                continue;
            }

            if bg_priority <= priority_buffer[x_screen] {
                let color = self.read_palette_color(palette, color_index);
                line_buffer[x_screen] = color;
                priority_buffer[x_screen] = bg_priority;
            }
        }
    }

    /// renders sprites (OBJ) into the line buffer
    fn render_sprites(
        &self,
        line: usize,
        vram: &[u8],
        palette: &[u8],
        oam: &[u8],
        line_buffer: &mut [u16; DISPLAY_WIDTH],
        priority_buffer: &mut [u8; DISPLAY_WIDTH],
    ) {
        // iterate sprites in reverse order (sprite 0 has highest priority)
        for obj_index in (0..OBJ_COUNT).rev() {
            let oam_offset = obj_index * 8;
            if oam_offset + 7 >= oam.len() {
                continue;
            }

            let attr0 = (oam[oam_offset] as u16) | ((oam[oam_offset + 1] as u16) << 8);
            let attr1 = (oam[oam_offset + 2] as u16) | ((oam[oam_offset + 3] as u16) << 8);
            let attr2 = (oam[oam_offset + 4] as u16) | ((oam[oam_offset + 5] as u16) << 8);

            // check if sprite is disabled (OBJ disable or affine double-size with no affine)
            let obj_mode = (attr0 >> 8) & 0x03;
            if obj_mode == 2 {
                continue; // disabled
            }

            let is_affine = obj_mode == 1 || obj_mode == 3;
            let _is_double = obj_mode == 3;

            // skip affine sprites for now (complex, implement later)
            if is_affine {
                continue;
            }

            let shape = (attr0 >> 14) & 0x03;
            let size = (attr1 >> 14) & 0x03;
            let (width, height) = obj_dimensions(shape as u8, size as u8);

            let y = (attr0 & 0xFF) as i32;
            let x = (attr1 & 0x01FF) as i32;
            let x = if x >= 240 { x - 512 } else { x };
            let y = if y >= 160 { y - 256 } else { y };

            // check if this sprite is on the current scanline
            let rel_y = line as i32 - y;
            if rel_y < 0 || rel_y >= height as i32 {
                continue;
            }

            let h_flip = attr1 & (1 << 12) != 0;
            let v_flip = attr1 & (1 << 13) != 0;
            let tile_number = (attr2 & 0x03FF) as usize;
            let priority = ((attr2 >> 10) & 0x03) as u8;
            let palette_bank = ((attr2 >> 12) & 0x0F) as usize;
            let is_8bpp = attr0 & (1 << 13) != 0;

            let pixel_y = if v_flip {
                height - 1 - rel_y as usize
            } else {
                rel_y as usize
            };

            // OBJ tile base is 0x10000 in VRAM
            let obj_base = 0x10000usize;
            let mapping_1d = self.dispcnt & (1 << 6) != 0;

            for pixel_x_offset in 0..width {
                let screen_x = x + pixel_x_offset as i32;
                if screen_x < 0 || screen_x >= DISPLAY_WIDTH as i32 {
                    continue;
                }
                let screen_x = screen_x as usize;

                let px = if h_flip {
                    width - 1 - pixel_x_offset
                } else {
                    pixel_x_offset
                };

                let color_index = if is_8bpp {
                    let tile_x = px / 8;
                    let tile_y_offset = pixel_y / 8;
                    let tile_offset = if mapping_1d {
                        tile_number + tile_y_offset * (width / 8) * 2 + tile_x * 2
                    } else {
                        tile_number + tile_y_offset * 32 + tile_x * 2
                    };
                    let byte_offset = obj_base + tile_offset * 32 + (pixel_y % 8) * 8 + (px % 8);
                    if byte_offset < vram.len() {
                        vram[byte_offset] as usize
                    } else {
                        0
                    }
                } else {
                    let tile_x = px / 8;
                    let tile_y_offset = pixel_y / 8;
                    let tile_offset = if mapping_1d {
                        tile_number + tile_y_offset * (width / 8) + tile_x
                    } else {
                        tile_number + tile_y_offset * 32 + tile_x
                    };
                    let byte_offset =
                        obj_base + tile_offset * 32 + (pixel_y % 8) * 4 + (px % 8) / 2;
                    if byte_offset < vram.len() {
                        let byte = vram[byte_offset];
                        if (px % 8) & 1 == 0 {
                            (byte & 0x0F) as usize
                        } else {
                            ((byte >> 4) & 0x0F) as usize
                        }
                    } else {
                        0
                    }
                };

                if color_index == 0 {
                    continue;
                }

                if priority <= priority_buffer[screen_x] {
                    // OBJ palette starts at offset 0x200 in palette RAM
                    let color = if is_8bpp {
                        self.read_palette_color_obj(palette, color_index)
                    } else {
                        self.read_palette_color_obj(palette, palette_bank * 16 + color_index)
                    };
                    line_buffer[screen_x] = color;
                    priority_buffer[screen_x] = priority;
                }
            }
        }
    }

    /// reads a 15-bit BGR555 color from BG palette RAM
    fn read_palette_color(&self, palette: &[u8], index: usize) -> u16 {
        let offset = index * 2;
        if offset + 1 < palette.len() {
            (palette[offset] as u16) | ((palette[offset + 1] as u16) << 8)
        } else {
            0
        }
    }

    /// reads a 15-bit BGR555 color from OBJ palette RAM (starts at 0x200)
    fn read_palette_color_obj(&self, palette: &[u8], index: usize) -> u16 {
        let offset = 0x200 + index * 2;
        if offset + 1 < palette.len() {
            (palette[offset] as u16) | ((palette[offset + 1] as u16) << 8)
        } else {
            0
        }
    }

    /// converts the line buffer (BGR555) to RGB888 and writes to frame buffer
    fn write_line_buffer(&mut self, line: usize, line_buffer: &[u16; DISPLAY_WIDTH]) {
        let offset = line * DISPLAY_WIDTH * 3;
        for (x, &color) in line_buffer.iter().enumerate() {
            let (r, g, b) = bgr555_to_rgb888(color);
            self.frame_buffer[offset + x * 3] = r;
            self.frame_buffer[offset + x * 3 + 1] = g;
            self.frame_buffer[offset + x * 3 + 2] = b;
        }
    }

    pub fn reset(&mut self) {
        self.vcount = 0;
        self.dot = 0;
        self.dispcnt = 0;
        self.dispstat = 0;
        self.bgcnt = [0; BG_COUNT];
        self.bg_hofs = [0; BG_COUNT];
        self.bg_vofs = [0; BG_COUNT];
        self.bg_pa = [0x100, 0x100];
        self.bg_pb = [0; 2];
        self.bg_pc = [0; 2];
        self.bg_pd = [0x100, 0x100];
        self.bg_ref_x = [0; 2];
        self.bg_ref_y = [0; 2];
        self.bg_ref_x_write = [0; 2];
        self.bg_ref_y_write = [0; 2];
        self.int_vblank = false;
        self.int_hblank = false;
        self.int_vcount = false;
        self.frame = 0;
        self.frame_buffer.fill(0);
    }
}

impl Default for GbaPpu {
    fn default() -> Self {
        Self::new()
    }
}

/// returns (width, height) in pixels for a sprite given shape and size
fn obj_dimensions(shape: u8, size: u8) -> (usize, usize) {
    match (shape, size) {
        // square
        (0, 0) => (8, 8),
        (0, 1) => (16, 16),
        (0, 2) => (32, 32),
        (0, 3) => (64, 64),
        // horizontal
        (1, 0) => (16, 8),
        (1, 1) => (32, 8),
        (1, 2) => (32, 16),
        (1, 3) => (64, 32),
        // vertical
        (2, 0) => (8, 16),
        (2, 1) => (8, 32),
        (2, 2) => (16, 32),
        (2, 3) => (32, 64),
        _ => (8, 8),
    }
}

/// converts a 15-bit BGR555 color to RGB888
#[inline(always)]
fn bgr555_to_rgb888(color: u16) -> (u8, u8, u8) {
    let r = ((color & 0x1F) << 3) as u8;
    let g = (((color >> 5) & 0x1F) << 3) as u8;
    let b = (((color >> 10) & 0x1F) << 3) as u8;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::{bgr555_to_rgb888, obj_dimensions, GbaPpu, VideoMode};
    use crate::gba::consts::{CYCLES_PER_SCANLINE, DISPLAY_HEIGHT, DISPLAY_WIDTH, VISIBLE_DOTS};

    #[test]
    fn test_new() {
        let ppu = GbaPpu::new();
        assert_eq!(ppu.vcount(), 0);
        assert_eq!(ppu.dispcnt(), 0);
        assert_eq!(ppu.frame(), 0);
        assert_eq!(ppu.frame_buffer().len(), DISPLAY_WIDTH * DISPLAY_HEIGHT * 3);
        assert!(!ppu.int_vblank());
        assert!(!ppu.int_hblank());
        assert!(!ppu.int_vcount());
    }

    #[test]
    fn test_dispcnt() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(0x0403);
        assert_eq!(ppu.dispcnt(), 0x0403);
        assert_eq!(ppu.video_mode(), VideoMode::Mode3);
    }

    #[test]
    fn test_dispstat_readonly_bits() {
        let mut ppu = GbaPpu::new();
        // bits 0-2 are read-only; writing them should not change stored bits
        ppu.set_dispstat(0x0007);
        // bit 2 (vcount match) is computed dynamically: vcount=0 matches lyc=0
        let status = ppu.dispstat();
        assert_eq!(status & 0x03, 0); // vblank/hblank flags not set
        assert_eq!(status & 0x04, 0x04); // vcount match: vcount=0 == lyc=0
    }

    #[test]
    fn test_dispstat_vblank_flag() {
        let mut ppu = GbaPpu::new();
        // simulate being in vblank (vcount >= 160)
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];
        // clock through visible lines
        for _ in 0..DISPLAY_HEIGHT {
            ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        assert!(ppu.dispstat() & 1 != 0); // vblank flag
    }

    #[test]
    fn test_dispstat_hblank_flag() {
        let mut ppu = GbaPpu::new();
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];
        // clock past visible dots but before end of scanline
        ppu.clock(VISIBLE_DOTS, &vram, &palette, &oam);
        assert!(ppu.dispstat() & 2 != 0); // hblank flag
    }

    #[test]
    fn test_bgcnt() {
        let mut ppu = GbaPpu::new();
        ppu.set_bgcnt(0, 0x1234);
        ppu.set_bgcnt(3, 0x5678);
        assert_eq!(ppu.bgcnt(0), 0x1234);
        assert_eq!(ppu.bgcnt(3), 0x5678);
    }

    #[test]
    fn test_bg_scroll() {
        let mut ppu = GbaPpu::new();
        ppu.set_bg_hofs(0, 0x1FF);
        ppu.set_bg_vofs(0, 0x200); // masked to 9 bits
                                   // value is masked to 0x01FF
        assert_eq!(ppu.bg_hofs[0], 0x1FF);
        assert_eq!(ppu.bg_vofs[0], 0x0000);
    }

    #[test]
    fn test_bg_affine_params() {
        let mut ppu = GbaPpu::new();
        ppu.set_bg_pa(0, 0x0100);
        ppu.set_bg_pb(0, 0x0200);
        ppu.set_bg_pc(0, 0x0300);
        ppu.set_bg_pd(0, 0x0400);
        assert_eq!(ppu.bg_pa[0], 0x0100);
        assert_eq!(ppu.bg_pb[0], 0x0200);
        assert_eq!(ppu.bg_pc[0], 0x0300);
        assert_eq!(ppu.bg_pd[0], 0x0400);
    }

    #[test]
    fn test_bg_ref_point() {
        let mut ppu = GbaPpu::new();
        ppu.set_bg_ref_x(0, 0x1000);
        ppu.set_bg_ref_y(0, 0x2000);
        assert_eq!(ppu.bg_ref_x[0], 0x1000);
        assert_eq!(ppu.bg_ref_y[0], 0x2000);
    }

    #[test]
    fn test_window_registers() {
        let mut ppu = GbaPpu::new();
        ppu.set_winh(0, 0xA050);
        ppu.set_winv(0, 0xC030);
        ppu.set_winin(0x1234);
        ppu.set_winout(0x5678);
        assert_eq!(ppu.winh[0], 0xA050);
        assert_eq!(ppu.winv[0], 0xC030);
        assert_eq!(ppu.winin, 0x1234);
        assert_eq!(ppu.winout, 0x5678);
    }

    #[test]
    fn test_blend_registers() {
        let mut ppu = GbaPpu::new();
        ppu.set_bldcnt(0x00FF);
        ppu.set_bldalpha(0x1010);
        ppu.set_bldy(0x0010);
        assert_eq!(ppu.bldcnt, 0x00FF);
        assert_eq!(ppu.bldalpha, 0x1010);
        assert_eq!(ppu.bldy, 0x0010);
    }

    #[test]
    fn test_mosaic() {
        let mut ppu = GbaPpu::new();
        ppu.set_mosaic(0x0303);
        assert_eq!(ppu.mosaic, 0x0303);
    }

    #[test]
    fn test_interrupt_ack() {
        let mut ppu = GbaPpu::new();
        ppu.int_vblank = true;
        ppu.int_hblank = true;
        ppu.int_vcount = true;

        assert!(ppu.int_vblank());
        ppu.ack_vblank();
        assert!(!ppu.int_vblank());

        assert!(ppu.int_hblank());
        ppu.ack_hblank();
        assert!(!ppu.int_hblank());

        assert!(ppu.int_vcount());
        ppu.ack_vcount();
        assert!(!ppu.int_vcount());
    }

    #[test]
    fn test_clock_frame_completion() {
        let mut ppu = GbaPpu::new();
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        assert_eq!(ppu.frame(), 0);
        // clock through a full frame (228 scanlines)
        for _ in 0..228 {
            ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        assert_eq!(ppu.frame(), 1);
    }

    #[test]
    fn test_clock_vblank_event() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispstat(1 << 3); // enable vblank IRQ
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        // clock to vblank (line 160)
        for _ in 0..DISPLAY_HEIGHT {
            ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        assert!(ppu.int_vblank());
    }

    #[test]
    fn test_clock_hblank_event() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispstat(1 << 4); // enable hblank IRQ
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        let events = ppu.clock(VISIBLE_DOTS, &vram, &palette, &oam);
        assert!(events & 1 != 0); // hblank event
        assert!(ppu.int_hblank());
    }

    #[test]
    fn test_reset() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(0x1234);
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];
        for _ in 0..228 {
            ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        ppu.reset();
        assert_eq!(ppu.dispcnt(), 0);
        assert_eq!(ppu.frame(), 0);
        assert_eq!(ppu.vcount(), 0);
    }

    #[test]
    fn test_bgr555_to_rgb888() {
        let (r, g, b) = bgr555_to_rgb888(0x7FFF);
        assert_eq!(r, 0xF8);
        assert_eq!(g, 0xF8);
        assert_eq!(b, 0xF8);

        let (r, g, b) = bgr555_to_rgb888(0x0000);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b) = bgr555_to_rgb888(0x001F);
        assert_eq!(r, 0xF8);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_bgr555_pure_green() {
        let (r, g, b) = bgr555_to_rgb888(0x03E0);
        assert_eq!(r, 0);
        assert_eq!(g, 0xF8);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_bgr555_pure_blue() {
        let (r, g, b) = bgr555_to_rgb888(0x7C00);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0xF8);
    }

    #[test]
    fn test_obj_dimensions() {
        assert_eq!(obj_dimensions(0, 0), (8, 8));
        assert_eq!(obj_dimensions(0, 3), (64, 64));
        assert_eq!(obj_dimensions(1, 3), (64, 32));
        assert_eq!(obj_dimensions(2, 3), (32, 64));
    }

    #[test]
    fn test_obj_dimensions_all_square() {
        assert_eq!(obj_dimensions(0, 0), (8, 8));
        assert_eq!(obj_dimensions(0, 1), (16, 16));
        assert_eq!(obj_dimensions(0, 2), (32, 32));
        assert_eq!(obj_dimensions(0, 3), (64, 64));
    }

    #[test]
    fn test_video_mode() {
        assert_eq!(VideoMode::from_u16(0), VideoMode::Mode0);
        assert_eq!(VideoMode::from_u16(3), VideoMode::Mode3);
        assert_eq!(VideoMode::from_u16(0x0403), VideoMode::Mode3);
    }

    #[test]
    fn test_video_mode_all() {
        assert_eq!(VideoMode::from_u16(0), VideoMode::Mode0);
        assert_eq!(VideoMode::from_u16(1), VideoMode::Mode1);
        assert_eq!(VideoMode::from_u16(2), VideoMode::Mode2);
        assert_eq!(VideoMode::from_u16(3), VideoMode::Mode3);
        assert_eq!(VideoMode::from_u16(4), VideoMode::Mode4);
        assert_eq!(VideoMode::from_u16(5), VideoMode::Mode5);
    }
}
