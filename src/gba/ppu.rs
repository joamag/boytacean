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

/// Layer ID for blending target identification
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Layer {
    Bg0 = 0,
    Bg1 = 1,
    Bg2 = 2,
    Bg3 = 3,
    Obj = 4,
    Backdrop = 5,
}

/// per-pixel compositing entry used during scanline rendering
#[derive(Clone, Copy)]
struct PixelEntry {
    color: u16,
    priority: u8,
    layer: Layer,
    is_semi_transparent: bool,
}

impl Default for PixelEntry {
    fn default() -> Self {
        Self {
            color: 0,
            priority: 5,
            layer: Layer::Backdrop,
            is_semi_transparent: false,
        }
    }
}

/// blend mode from BLDCNT bits 6-7
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlendMode {
    None = 0,
    Alpha = 1,
    BrightnessInc = 2,
    BrightnessDec = 3,
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

    /// Raw 32-bit shadow registers for partial 16-bit writes to BG ref points
    bg_ref_x_raw: [u32; 2],
    bg_ref_y_raw: [u32; 2],

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
            bg_ref_x_raw: [0; 2],
            bg_ref_y_raw: [0; 2],
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

    pub fn bg_hofs(&self, index: usize) -> u16 {
        self.bg_hofs[index]
    }

    pub fn bg_vofs(&self, index: usize) -> u16 {
        self.bg_vofs[index]
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

    pub fn set_bg_ref_x_lo(&mut self, index: usize, value: u16) {
        self.bg_ref_x_raw[index] = (self.bg_ref_x_raw[index] & 0xFFFF0000) | value as u32;
        self.apply_bg_ref_x(index);
    }

    pub fn set_bg_ref_x_hi(&mut self, index: usize, value: u16) {
        self.bg_ref_x_raw[index] = (self.bg_ref_x_raw[index] & 0x0000FFFF) | ((value as u32) << 16);
        self.apply_bg_ref_x(index);
    }

    pub fn set_bg_ref_y_lo(&mut self, index: usize, value: u16) {
        self.bg_ref_y_raw[index] = (self.bg_ref_y_raw[index] & 0xFFFF0000) | value as u32;
        self.apply_bg_ref_y(index);
    }

    pub fn set_bg_ref_y_hi(&mut self, index: usize, value: u16) {
        self.bg_ref_y_raw[index] = (self.bg_ref_y_raw[index] & 0x0000FFFF) | ((value as u32) << 16);
        self.apply_bg_ref_y(index);
    }

    fn apply_bg_ref_x(&mut self, index: usize) {
        // 28-bit signed, sign-extend from bit 27
        let signed = ((self.bg_ref_x_raw[index] as i32) << 4) >> 4;
        self.bg_ref_x_write[index] = signed;
        self.bg_ref_x[index] = signed;
    }

    fn apply_bg_ref_y(&mut self, index: usize) {
        let signed = ((self.bg_ref_y_raw[index] as i32) << 4) >> 4;
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

    /// returns true if any window is enabled in DISPCNT (bits 13-15)
    fn windows_enabled(&self) -> bool {
        self.dispcnt & 0xE000 != 0
    }

    /// evaluates window mask including OBJ window support
    fn window_mask_with_obj(&self, x: usize, line: usize, obj_window: bool) -> u8 {
        if !self.windows_enabled() {
            return 0x3F;
        }

        if self.dispcnt & (1 << 13) != 0 && self.pixel_in_window(x, line, 0) {
            return (self.winin & 0x3F) as u8;
        }

        if self.dispcnt & (1 << 14) != 0 && self.pixel_in_window(x, line, 1) {
            return ((self.winin >> 8) & 0x3F) as u8;
        }

        if self.dispcnt & (1 << 15) != 0 && obj_window {
            return ((self.winout >> 8) & 0x3F) as u8;
        }

        (self.winout & 0x3F) as u8
    }

    /// checks if a pixel (x, line) is inside the given window (0 or 1)
    fn pixel_in_window(&self, x: usize, line: usize, win: usize) -> bool {
        let h = self.winh[win];
        let v = self.winv[win];
        let x1 = (h >> 8) as usize;
        let x2 = (h & 0xFF) as usize;
        let y1 = (v >> 8) as usize;
        let y2 = (v & 0xFF) as usize;

        let in_h = if x1 <= x2 {
            x >= x1 && x < x2
        } else {
            x >= x1 || x < x2
        };
        let in_v = if y1 <= y2 {
            line >= y1 && line < y2
        } else {
            line >= y1 || line < y2
        };
        in_h && in_v
    }

    /// returns the blend mode from BLDCNT
    fn blend_mode(&self) -> BlendMode {
        match (self.bldcnt >> 6) & 0x03 {
            0 => BlendMode::None,
            1 => BlendMode::Alpha,
            2 => BlendMode::BrightnessInc,
            3 => BlendMode::BrightnessDec,
            _ => BlendMode::None,
        }
    }

    /// checks if a layer is a first target in BLDCNT (bits 0-5)
    fn is_blend_target1(&self, layer: Layer) -> bool {
        self.bldcnt & (1 << layer as u16) != 0
    }

    /// checks if a layer is a second target in BLDCNT (bits 8-13)
    fn is_blend_target2(&self, layer: Layer) -> bool {
        self.bldcnt & (1 << (8 + layer as u16)) != 0
    }

    /// applies alpha blending between two BGR555 colors
    fn blend_alpha(&self, color1: u16, color2: u16) -> u16 {
        let eva = ((self.bldalpha & 0x1F) as u32).min(16);
        let evb = (((self.bldalpha >> 8) & 0x1F) as u32).min(16);

        let r1 = (color1 & 0x1F) as u32;
        let g1 = ((color1 >> 5) & 0x1F) as u32;
        let b1 = ((color1 >> 10) & 0x1F) as u32;

        let r2 = (color2 & 0x1F) as u32;
        let g2 = ((color2 >> 5) & 0x1F) as u32;
        let b2 = ((color2 >> 10) & 0x1F) as u32;

        let r = ((r1 * eva + r2 * evb) >> 4).min(31);
        let g = ((g1 * eva + g2 * evb) >> 4).min(31);
        let b = ((b1 * eva + b2 * evb) >> 4).min(31);

        (r | (g << 5) | (b << 10)) as u16
    }

    /// applies brightness increase to a BGR555 color
    fn blend_brighten(&self, color: u16) -> u16 {
        let evy = ((self.bldy & 0x1F) as u32).min(16);

        let r = (color & 0x1F) as u32;
        let g = ((color >> 5) & 0x1F) as u32;
        let b = ((color >> 10) & 0x1F) as u32;

        let r = r + (((31 - r) * evy) >> 4);
        let g = g + (((31 - g) * evy) >> 4);
        let b = b + (((31 - b) * evy) >> 4);

        (r | (g << 5) | (b << 10)) as u16
    }

    /// applies brightness decrease to a BGR555 color
    fn blend_darken(&self, color: u16) -> u16 {
        let evy = ((self.bldy & 0x1F) as u32).min(16);

        let r = (color & 0x1F) as u32;
        let g = ((color >> 5) & 0x1F) as u32;
        let b = ((color >> 10) & 0x1F) as u32;

        let r = r - ((r * evy) >> 4);
        let g = g - ((g * evy) >> 4);
        let b = b - ((b * evy) >> 4);

        (r | (g << 5) | (b << 10)) as u16
    }

    /// applies mosaic effect to OBJ x coordinate
    fn apply_obj_mosaic_x(&self, x: usize) -> usize {
        let h_size = ((self.mosaic >> 8) & 0x0F) as usize + 1;
        x / h_size * h_size
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

            // bit 0: hblank DMA trigger (always fires)
            events |= 1;

            // bit 2: hblank IRQ (only when DISPSTAT bit 4 is set)
            if self.dispstat & (1 << 4) != 0 {
                self.int_hblank = true;
                events |= 4;
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

                // bit 1: vblank DMA trigger (always fires)
                events |= 2;

                // bit 3: vblank IRQ (only when DISPSTAT bit 3 is set)
                if self.dispstat & (1 << 3) != 0 {
                    self.int_vblank = true;
                    events |= 8;
                }
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

    /// composites BG and OBJ layers with window masking and blending.
    /// `bg_layers` contains (bg_index, is_affine, affine_index) for each enabled BG.
    fn render_composited(
        &mut self,
        line: usize,
        bg_layers: &[(usize, bool, usize)],
        vram: &[u8],
        palette: &[u8],
        oam: &[u8],
    ) {
        let backdrop = self.read_palette_color(palette, 0);

        // collect per-pixel BG colors: [bg_index] -> Option<(color, priority)>
        let mut bg_pixels: [[(u16, u8); DISPLAY_WIDTH]; 4] = [[(0, 255); DISPLAY_WIDTH]; 4];
        let mut bg_has_pixel: [[bool; DISPLAY_WIDTH]; 4] = [[false; DISPLAY_WIDTH]; 4];

        for &(bg_index, is_affine, affine_index) in bg_layers {
            let bg_priority = (self.bgcnt[bg_index] & 0x03) as u8;
            if is_affine {
                self.collect_affine_bg(
                    affine_index,
                    bg_index,
                    vram,
                    palette,
                    &mut bg_pixels[bg_index],
                    &mut bg_has_pixel[bg_index],
                    bg_priority,
                );
            } else {
                self.collect_text_bg(
                    bg_index,
                    line,
                    vram,
                    palette,
                    &mut bg_pixels[bg_index],
                    &mut bg_has_pixel[bg_index],
                    bg_priority,
                );
            }
        }

        // collect OBJ pixels
        let mut obj_pixels: [(u16, u8, bool); DISPLAY_WIDTH] = [(0, 255, false); DISPLAY_WIDTH];
        let mut obj_has_pixel = [false; DISPLAY_WIDTH];
        let mut obj_window_mask = [false; DISPLAY_WIDTH];

        if self.dispcnt & (1 << 12) != 0 {
            self.collect_sprites(
                line,
                vram,
                palette,
                oam,
                &mut obj_pixels,
                &mut obj_has_pixel,
                &mut obj_window_mask,
            );
        }

        let use_windows = self.windows_enabled();
        let blend_mode = self.blend_mode();
        let bg_layer_ids = [Layer::Bg0, Layer::Bg1, Layer::Bg2, Layer::Bg3];

        let mut line_buffer = [backdrop; DISPLAY_WIDTH];

        for x in 0..DISPLAY_WIDTH {
            let win_mask = if use_windows {
                self.window_mask_with_obj(x, line, obj_window_mask[x])
            } else {
                0x3F
            };

            let blend_enabled = win_mask & 0x20 != 0;

            // find top two visible pixels (for blending)
            let mut top = PixelEntry {
                color: backdrop,
                priority: 5,
                layer: Layer::Backdrop,
                is_semi_transparent: false,
            };
            let mut bot = PixelEntry {
                color: backdrop,
                priority: 5,
                layer: Layer::Backdrop,
                is_semi_transparent: false,
            };

            // check OBJ (can interleave with BG by priority)
            let obj_visible = obj_has_pixel[x] && (win_mask & 0x10 != 0);
            let obj_entry = if obj_visible {
                let (color, pri, semi) = obj_pixels[x];
                Some(PixelEntry {
                    color,
                    priority: pri,
                    layer: Layer::Obj,
                    is_semi_transparent: semi,
                })
            } else {
                None
            };

            // iterate BGs by priority (lower value = higher priority),
            // then by BG index as tiebreaker (lower index = higher priority)
            for bg_index in 0..4 {
                if !bg_has_pixel[bg_index][x] {
                    continue;
                }
                if win_mask & (1 << bg_index) == 0 {
                    continue;
                }
                let (color, pri) = bg_pixels[bg_index][x];
                let entry = PixelEntry {
                    color,
                    priority: pri,
                    layer: bg_layer_ids[bg_index],
                    is_semi_transparent: false,
                };

                // insert OBJ if it has higher priority than this BG
                // and we haven't inserted it yet
                if let Some(ref obj) = obj_entry {
                    if obj.priority <= entry.priority && top.layer == Layer::Backdrop {
                        // OBJ goes before this BG
                        if top.layer == Layer::Backdrop {
                            bot = top;
                            top = *obj;
                        }
                    }
                }

                if (entry.priority < top.priority)
                    || (entry.priority == top.priority && top.layer == Layer::Backdrop)
                {
                    bot = top;
                    top = entry;
                } else if (entry.priority < bot.priority)
                    || (entry.priority == bot.priority && bot.layer == Layer::Backdrop)
                {
                    bot = entry;
                }
            }

            // insert OBJ if it hasn't been inserted yet
            if let Some(obj) = obj_entry {
                if (obj.priority < top.priority)
                    || (obj.priority == top.priority && top.layer == Layer::Backdrop)
                {
                    bot = top;
                    top = obj;
                } else if (obj.priority < bot.priority)
                    || (obj.priority == bot.priority && bot.layer == Layer::Backdrop)
                {
                    bot = obj;
                }
            }

            // apply blending
            let final_color = if blend_enabled {
                if top.is_semi_transparent && self.is_blend_target2(bot.layer) {
                    // semi-transparent OBJ always alpha-blends regardless of BLDCNT mode
                    self.blend_alpha(top.color, bot.color)
                } else {
                    match blend_mode {
                        BlendMode::Alpha => {
                            if self.is_blend_target1(top.layer) && self.is_blend_target2(bot.layer)
                            {
                                self.blend_alpha(top.color, bot.color)
                            } else {
                                top.color
                            }
                        }
                        BlendMode::BrightnessInc => {
                            if self.is_blend_target1(top.layer) {
                                self.blend_brighten(top.color)
                            } else {
                                top.color
                            }
                        }
                        BlendMode::BrightnessDec => {
                            if self.is_blend_target1(top.layer) {
                                self.blend_darken(top.color)
                            } else {
                                top.color
                            }
                        }
                        BlendMode::None => top.color,
                    }
                }
            } else {
                top.color
            };

            line_buffer[x] = final_color;
        }

        self.write_line_buffer(line, &line_buffer);
    }

    /// mode 0: 4 text BG layers
    fn render_mode0(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut layers = Vec::new();
        for bg_index in 0..4 {
            if self.dispcnt & (1 << (8 + bg_index)) != 0 {
                layers.push((bg_index, false, 0));
            }
        }
        self.render_composited(line, &layers, vram, palette, oam);
    }

    /// mode 1: 2 text BGs + 1 affine BG (BG2)
    fn render_mode1(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut layers = Vec::new();
        if self.dispcnt & (1 << 8) != 0 {
            layers.push((0, false, 0));
        }
        if self.dispcnt & (1 << 9) != 0 {
            layers.push((1, false, 0));
        }
        if self.dispcnt & (1 << 10) != 0 {
            layers.push((2, true, 0));
        }
        self.render_composited(line, &layers, vram, palette, oam);
    }

    /// mode 2: 2 affine BGs (BG2, BG3)
    fn render_mode2(&mut self, line: usize, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let mut layers = Vec::new();
        if self.dispcnt & (1 << 10) != 0 {
            layers.push((2, true, 0));
        }
        if self.dispcnt & (1 << 11) != 0 {
            layers.push((3, true, 1));
        }
        self.render_composited(line, &layers, vram, palette, oam);
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

    /// collects text background pixels into per-pixel arrays
    #[allow(clippy::too_many_arguments)]
    fn collect_text_bg(
        &self,
        bg_index: usize,
        line: usize,
        vram: &[u8],
        palette: &[u8],
        pixels: &mut [(u16, u8); DISPLAY_WIDTH],
        has_pixel: &mut [bool; DISPLAY_WIDTH],
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

        let mosaic_on = cnt & (1 << 6) != 0;
        let (mos_h, mos_v) = if mosaic_on {
            (
                (self.mosaic & 0x0F) as u32 + 1,
                ((self.mosaic >> 4) & 0x0F) as u32 + 1,
            )
        } else {
            (1, 1)
        };

        let mosaic_line = if mosaic_on {
            (line as u32 / mos_v) * mos_v
        } else {
            line as u32
        };
        let y = (mosaic_line + scroll_y) % map_height;

        for x_screen in 0..DISPLAY_WIDTH {
            let eff_x = if mosaic_on {
                (x_screen as u32 / mos_h) * mos_h
            } else {
                x_screen as u32
            };
            let x = (eff_x + scroll_x) % map_width;

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

            if color_index == 0 {
                continue;
            }

            let color = if is_8bpp {
                self.read_palette_color(palette, color_index)
            } else {
                self.read_palette_color(palette, palette_bank * 16 + color_index)
            };
            pixels[x_screen] = (color, bg_priority);
            has_pixel[x_screen] = true;
        }
    }

    /// collects affine background pixels into per-pixel arrays
    #[allow(clippy::too_many_arguments)]
    fn collect_affine_bg(
        &self,
        affine_index: usize,
        bg_index: usize,
        vram: &[u8],
        palette: &[u8],
        pixels: &mut [(u16, u8); DISPLAY_WIDTH],
        has_pixel: &mut [bool; DISPLAY_WIDTH],
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

            let color = self.read_palette_color(palette, color_index);
            pixels[x_screen] = (color, bg_priority);
            has_pixel[x_screen] = true;
        }

        // apply horizontal mosaic as a post-pass
        if cnt & (1 << 6) != 0 {
            let mos_h = (self.mosaic & 0x0F) as usize + 1;
            if mos_h > 1 {
                for x in 0..DISPLAY_WIDTH {
                    let src = x / mos_h * mos_h;
                    if src != x {
                        pixels[x] = pixels[src];
                        has_pixel[x] = has_pixel[src];
                    }
                }
            }
        }
    }

    /// collects sprite pixels with support for affine and OBJ window sprites
    #[allow(clippy::too_many_arguments)]
    fn collect_sprites(
        &self,
        line: usize,
        vram: &[u8],
        palette: &[u8],
        oam: &[u8],
        obj_pixels: &mut [(u16, u8, bool); DISPLAY_WIDTH],
        obj_has_pixel: &mut [bool; DISPLAY_WIDTH],
        obj_window_mask: &mut [bool; DISPLAY_WIDTH],
    ) {
        let obj_base = 0x10000usize;
        let mapping_1d = self.dispcnt & (1 << 6) != 0;

        // iterate sprites in reverse order (sprite 0 has highest priority)
        for obj_index in (0..OBJ_COUNT).rev() {
            let oam_offset = obj_index * 8;
            if oam_offset + 7 >= oam.len() {
                continue;
            }

            let attr0 = (oam[oam_offset] as u16) | ((oam[oam_offset + 1] as u16) << 8);
            let attr1 = (oam[oam_offset + 2] as u16) | ((oam[oam_offset + 3] as u16) << 8);
            let attr2 = (oam[oam_offset + 4] as u16) | ((oam[oam_offset + 5] as u16) << 8);

            let is_affine = attr0 & (1 << 8) != 0;
            let is_double = is_affine && attr0 & (1 << 9) != 0;

            // non-affine: bit 9 = 1 means disabled
            if !is_affine && attr0 & (1 << 9) != 0 {
                continue;
            }

            let obj_mode = (attr0 >> 10) & 0x03;
            // obj_mode 3 is forbidden
            if obj_mode == 3 {
                continue;
            }

            let shape = (attr0 >> 14) & 0x03;
            let size = (attr1 >> 14) & 0x03;
            let (width, height) = obj_dimensions(shape as u8, size as u8);

            // double-size affine sprites have double the bounding box
            let bound_w = if is_double { width * 2 } else { width };
            let bound_h = if is_double { height * 2 } else { height };

            let y = (attr0 & 0xFF) as i32;
            let x = (attr1 & 0x01FF) as i32;
            let x = if x >= 240 { x - 512 } else { x };
            let y = if y >= 160 { y - 256 } else { y };

            // check if this sprite is on the current scanline
            let rel_y = line as i32 - y;
            if rel_y < 0 || rel_y >= bound_h as i32 {
                continue;
            }

            let tile_number = (attr2 & 0x03FF) as usize;
            let priority = ((attr2 >> 10) & 0x03) as u8;
            let palette_bank = ((attr2 >> 12) & 0x0F) as usize;
            let is_8bpp = attr0 & (1 << 13) != 0;
            let is_semi_transparent = obj_mode == 1;
            let is_obj_window = obj_mode == 2;
            let use_mosaic = attr0 & (1 << 12) != 0;

            if is_affine {
                // affine sprite rendering
                let affine_index = ((attr1 >> 9) & 0x1F) as usize;
                let affine_oam = affine_index * 32;

                // read affine parameters from OAM (PA, PB, PC, PD at +6, +14, +22, +30)
                let pa = if affine_oam + 7 < oam.len() {
                    (oam[affine_oam + 6] as i16) | ((oam[affine_oam + 7] as i16) << 8)
                } else {
                    0x100
                };
                let pb = if affine_oam + 15 < oam.len() {
                    (oam[affine_oam + 14] as i16) | ((oam[affine_oam + 15] as i16) << 8)
                } else {
                    0
                };
                let pc = if affine_oam + 23 < oam.len() {
                    (oam[affine_oam + 22] as i16) | ((oam[affine_oam + 23] as i16) << 8)
                } else {
                    0
                };
                let pd = if affine_oam + 31 < oam.len() {
                    (oam[affine_oam + 30] as i16) | ((oam[affine_oam + 31] as i16) << 8)
                } else {
                    0x100
                };

                let half_w = width as i32 / 2;
                let half_h = height as i32 / 2;
                let iy = rel_y - bound_h as i32 / 2;

                for screen_dx in 0..bound_w {
                    let screen_x = x + screen_dx as i32;
                    if screen_x < 0 || screen_x >= DISPLAY_WIDTH as i32 {
                        continue;
                    }
                    let screen_x = screen_x as usize;

                    let ix = screen_dx as i32 - bound_w as i32 / 2;

                    // inverse affine transform
                    let tex_x = ((pa as i32 * ix + pb as i32 * iy) >> 8) + half_w;
                    let tex_y = ((pc as i32 * ix + pd as i32 * iy) >> 8) + half_h;

                    if tex_x < 0 || tex_x >= width as i32 || tex_y < 0 || tex_y >= height as i32 {
                        continue;
                    }

                    let px = tex_x as usize;
                    let py = tex_y as usize;

                    let color_index = self.read_obj_pixel(
                        vram,
                        tile_number,
                        px,
                        py,
                        width,
                        is_8bpp,
                        mapping_1d,
                        obj_base,
                    );

                    if color_index == 0 {
                        continue;
                    }

                    let sx = if use_mosaic {
                        self.apply_obj_mosaic_x(screen_x)
                    } else {
                        screen_x
                    };
                    let _ = sx; // mosaic would re-sample, but for now use screen_x

                    if is_obj_window {
                        obj_window_mask[screen_x] = true;
                        continue;
                    }

                    if priority <= obj_pixels[screen_x].1 || !obj_has_pixel[screen_x] {
                        let color = if is_8bpp {
                            self.read_palette_color_obj(palette, color_index)
                        } else {
                            self.read_palette_color_obj(palette, palette_bank * 16 + color_index)
                        };
                        obj_pixels[screen_x] = (color, priority, is_semi_transparent);
                        obj_has_pixel[screen_x] = true;
                    }
                }
            } else {
                // regular (non-affine) sprite rendering
                let h_flip = attr1 & (1 << 12) != 0;
                let v_flip = attr1 & (1 << 13) != 0;

                let pixel_y = if v_flip {
                    height - 1 - rel_y as usize
                } else {
                    rel_y as usize
                };

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

                    let color_index = self.read_obj_pixel(
                        vram,
                        tile_number,
                        px,
                        pixel_y,
                        width,
                        is_8bpp,
                        mapping_1d,
                        obj_base,
                    );

                    if color_index == 0 {
                        continue;
                    }

                    if is_obj_window {
                        obj_window_mask[screen_x] = true;
                        continue;
                    }

                    if priority <= obj_pixels[screen_x].1 || !obj_has_pixel[screen_x] {
                        let color = if is_8bpp {
                            self.read_palette_color_obj(palette, color_index)
                        } else {
                            self.read_palette_color_obj(palette, palette_bank * 16 + color_index)
                        };
                        obj_pixels[screen_x] = (color, priority, is_semi_transparent);
                        obj_has_pixel[screen_x] = true;
                    }
                }
            }
        }
    }

    /// reads a single pixel from an OBJ tile, returning the palette color index
    #[allow(clippy::too_many_arguments)]
    fn read_obj_pixel(
        &self,
        vram: &[u8],
        tile_number: usize,
        px: usize,
        py: usize,
        width: usize,
        is_8bpp: bool,
        mapping_1d: bool,
        obj_base: usize,
    ) -> usize {
        if is_8bpp {
            let tile_x = px / 8;
            let tile_y_offset = py / 8;
            let tile_offset = if mapping_1d {
                tile_number + tile_y_offset * (width / 8) * 2 + tile_x * 2
            } else {
                tile_number + tile_y_offset * 32 + tile_x * 2
            };
            let byte_offset = obj_base + tile_offset * 32 + (py % 8) * 8 + (px % 8);
            if byte_offset < vram.len() {
                vram[byte_offset] as usize
            } else {
                0
            }
        } else {
            let tile_x = px / 8;
            let tile_y_offset = py / 8;
            let tile_offset = if mapping_1d {
                tile_number + tile_y_offset * (width / 8) + tile_x
            } else {
                tile_number + tile_y_offset * 32 + tile_x
            };
            let byte_offset = obj_base + tile_offset * 32 + (py % 8) * 4 + (px % 8) / 2;
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
        self.bg_ref_x_raw = [0; 2];
        self.bg_ref_y_raw = [0; 2];
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
    use super::{bgr555_to_rgb888, obj_dimensions, BlendMode, GbaPpu, Layer, VideoMode};
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
        ppu.set_bg_ref_x_lo(0, 0x1000);
        ppu.set_bg_ref_x_hi(0, 0x0000);
        ppu.set_bg_ref_y_lo(0, 0x2000);
        ppu.set_bg_ref_y_hi(0, 0x0000);
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
        let mut last_events = 0u8;
        for _ in 0..DISPLAY_HEIGHT {
            last_events = ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        assert!(last_events & 2 != 0); // vblank DMA event
        assert!(last_events & 8 != 0); // vblank IRQ event
        assert!(ppu.int_vblank());
    }

    #[test]
    fn test_clock_vblank_dma_without_irq() {
        let mut ppu = GbaPpu::new();
        // DISPSTAT bit 3 NOT set: vblank IRQ disabled
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        let mut last_events = 0u8;
        for _ in 0..DISPLAY_HEIGHT {
            last_events = ppu.clock(CYCLES_PER_SCANLINE, &vram, &palette, &oam);
        }
        assert!(last_events & 2 != 0); // vblank DMA event still fires
        assert!(last_events & 8 == 0); // vblank IRQ does NOT fire
        assert!(!ppu.int_vblank());
    }

    #[test]
    fn test_clock_hblank_event() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispstat(1 << 4); // enable hblank IRQ
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        let events = ppu.clock(VISIBLE_DOTS, &vram, &palette, &oam);
        assert!(events & 1 != 0); // hblank DMA event
        assert!(events & 4 != 0); // hblank IRQ event
        assert!(ppu.int_hblank());
    }

    #[test]
    fn test_clock_hblank_dma_without_irq() {
        let mut ppu = GbaPpu::new();
        // DISPSTAT bit 4 NOT set: hblank IRQ disabled
        let vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let oam = vec![0u8; 0x400];

        let events = ppu.clock(VISIBLE_DOTS, &vram, &palette, &oam);
        assert!(events & 1 != 0); // hblank DMA event still fires
        assert!(events & 4 == 0); // hblank IRQ does NOT fire
        assert!(!ppu.int_hblank());
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

    #[test]
    fn test_windows_enabled() {
        let mut ppu = GbaPpu::new();
        assert!(!ppu.windows_enabled());
        ppu.set_dispcnt(1 << 13); // WIN0
        assert!(ppu.windows_enabled());
        ppu.set_dispcnt(1 << 14); // WIN1
        assert!(ppu.windows_enabled());
        ppu.set_dispcnt(1 << 15); // OBJ window
        assert!(ppu.windows_enabled());
    }

    #[test]
    fn test_pixel_in_window() {
        let mut ppu = GbaPpu::new();
        // WIN0: x=[10,50), y=[20,40)
        ppu.set_winh(0, (10 << 8) | 50);
        ppu.set_winv(0, (20 << 8) | 40);

        assert!(ppu.pixel_in_window(10, 20, 0));
        assert!(ppu.pixel_in_window(49, 39, 0));
        assert!(!ppu.pixel_in_window(50, 20, 0)); // x out of range
        assert!(!ppu.pixel_in_window(10, 40, 0)); // y out of range
        assert!(!ppu.pixel_in_window(9, 20, 0)); // x before start
    }

    #[test]
    fn test_pixel_in_window_wrap() {
        let mut ppu = GbaPpu::new();
        // wrapping window: x2 < x1
        ppu.set_winh(0, (200 << 8) | 50);
        ppu.set_winv(0, 160);

        assert!(ppu.pixel_in_window(210, 80, 0)); // past x1
        assert!(ppu.pixel_in_window(30, 80, 0)); // before x2
        assert!(!ppu.pixel_in_window(100, 80, 0)); // between x2 and x1
    }

    #[test]
    fn test_window_mask_no_windows() {
        let ppu = GbaPpu::new();
        // no windows enabled: all bits set
        assert_eq!(ppu.window_mask_with_obj(0, 0, false), 0x3F);
    }

    #[test]
    fn test_window_mask_win0_inside() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(1 << 13); // enable WIN0
        ppu.set_winh(0, (10 << 8) | 50);
        ppu.set_winv(0, 160);
        ppu.set_winin(0x0015); // WIN0: BG0 + BG2 + OBJ
        ppu.set_winout(0x003F);

        assert_eq!(ppu.window_mask_with_obj(20, 10, false), 0x15);
    }

    #[test]
    fn test_window_mask_outside() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(1 << 13); // enable WIN0
        ppu.set_winh(0, (10 << 8) | 50);
        ppu.set_winv(0, 160);
        ppu.set_winin(0x0015);
        ppu.set_winout(0x002A); // WINOUT: BG1 + BG3 + blend

        assert_eq!(ppu.window_mask_with_obj(100, 10, false), 0x2A);
    }

    #[test]
    fn test_window_mask_obj_window() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt((1 << 15) | (1 << 13)); // OBJ WIN + WIN0
        ppu.set_winh(0, (10 << 8) | 20);
        ppu.set_winv(0, 160);
        ppu.set_winin(0x001F);
        ppu.set_winout(0x0A00 | 0x0001); // OBJ WIN: bits 8-13, WINOUT: BG0 only

        // outside WIN0, in OBJ window
        assert_eq!(ppu.window_mask_with_obj(100, 10, true), 0x0A);
    }

    #[test]
    fn test_blend_mode() {
        let mut ppu = GbaPpu::new();
        ppu.set_bldcnt(0 << 6);
        assert_eq!(ppu.blend_mode(), BlendMode::None);
        ppu.set_bldcnt(1 << 6);
        assert_eq!(ppu.blend_mode(), BlendMode::Alpha);
        ppu.set_bldcnt(2 << 6);
        assert_eq!(ppu.blend_mode(), BlendMode::BrightnessInc);
        ppu.set_bldcnt(3 << 6);
        assert_eq!(ppu.blend_mode(), BlendMode::BrightnessDec);
    }

    #[test]
    fn test_is_blend_target1() {
        let mut ppu = GbaPpu::new();
        ppu.set_bldcnt(1 << 0 | 1 << 4); // BG0 + OBJ
        assert!(ppu.is_blend_target1(Layer::Bg0));
        assert!(!ppu.is_blend_target1(Layer::Bg1));
        assert!(ppu.is_blend_target1(Layer::Obj));
        assert!(!ppu.is_blend_target1(Layer::Backdrop));
    }

    #[test]
    fn test_is_blend_target2() {
        let mut ppu = GbaPpu::new();
        ppu.set_bldcnt(1 << 9 | 1 << 13); // BG1 + Backdrop (second target)
        assert!(!ppu.is_blend_target2(Layer::Bg0));
        assert!(ppu.is_blend_target2(Layer::Bg1));
        assert!(ppu.is_blend_target2(Layer::Backdrop));
    }

    #[test]
    fn test_blend_alpha() {
        let mut ppu = GbaPpu::new();
        // EVA=16, EVB=0 -> result = color1
        ppu.set_bldalpha(0x0010);
        let white = 0x7FFF;
        let black = 0x0000;
        assert_eq!(ppu.blend_alpha(white, black), white);

        // EVA=0, EVB=16 -> result = color2
        ppu.set_bldalpha(0x1000);
        assert_eq!(ppu.blend_alpha(white, black), black);

        // EVA=8, EVB=8 -> 50% blend
        ppu.set_bldalpha(0x0808);
        let red = 0x001F; // R=31, G=0, B=0
        let result = ppu.blend_alpha(red, black);
        let r = result & 0x1F;
        assert_eq!(r, 15); // 31 * 8 / 16 = 15
    }

    #[test]
    fn test_blend_brighten() {
        let mut ppu = GbaPpu::new();
        // EVY=16 -> full white
        ppu.set_bldy(16);
        let black = 0x0000;
        assert_eq!(ppu.blend_brighten(black), 0x7FFF);

        // EVY=0 -> no change
        ppu.set_bldy(0);
        let red = 0x001F;
        assert_eq!(ppu.blend_brighten(red), red);
    }

    #[test]
    fn test_blend_darken() {
        let mut ppu = GbaPpu::new();
        // EVY=16 -> full black
        ppu.set_bldy(16);
        let white = 0x7FFF;
        assert_eq!(ppu.blend_darken(white), 0x0000);

        // EVY=0 -> no change
        ppu.set_bldy(0);
        assert_eq!(ppu.blend_darken(white), white);
    }

    #[test]
    fn test_read_obj_pixel_4bpp() {
        let ppu = GbaPpu::new();
        let mut vram = vec![0u8; 0x18000];
        // place a 4bpp pixel at tile 0, px=(0,0): low nibble = 5
        let obj_base = 0x10000;
        vram[obj_base] = 0x05;

        let idx = ppu.read_obj_pixel(&vram, 0, 0, 0, 8, false, true, obj_base);
        assert_eq!(idx, 5);

        // high nibble at px=(1,0) = 0xA
        vram[obj_base] = 0xA5;
        let idx = ppu.read_obj_pixel(&vram, 0, 1, 0, 8, false, true, obj_base);
        assert_eq!(idx, 0x0A);
    }

    #[test]
    fn test_read_obj_pixel_8bpp() {
        let ppu = GbaPpu::new();
        let mut vram = vec![0u8; 0x18000];
        let obj_base = 0x10000;
        vram[obj_base + 3] = 42;

        let idx = ppu.read_obj_pixel(&vram, 0, 3, 0, 8, true, true, obj_base);
        assert_eq!(idx, 42);
    }

    #[test]
    fn test_collect_sprites_regular() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(1 << 6); // 1D mapping

        let mut vram = vec![0u8; 0x18000];
        let mut palette = vec![0u8; 0x400];
        let mut oam = vec![0u8; 0x400];

        // set up sprite 0: 8x8, at (10, 0), 4bpp, priority 0
        let attr0: u16 = 0; // y=0, no affine, normal mode, 4bpp
        let attr1: u16 = 10; // x=10, size=0 (8x8)
        let attr2: u16 = 0; // tile=0, priority=0, palette=0
        oam[0] = (attr0 & 0xFF) as u8;
        oam[1] = ((attr0 >> 8) & 0xFF) as u8;
        oam[2] = (attr1 & 0xFF) as u8;
        oam[3] = ((attr1 >> 8) & 0xFF) as u8;
        oam[4] = (attr2 & 0xFF) as u8;
        oam[5] = ((attr2 >> 8) & 0xFF) as u8;

        // put pixel data at obj tile 0, line 0: pixel 0 = color 1
        let obj_base = 0x10000;
        vram[obj_base] = 0x01; // low nibble = 1

        // set OBJ palette color 1 (at palette offset 0x200 + 1*2)
        palette[0x202] = 0xFF;
        palette[0x203] = 0x7F; // white

        let mut obj_pixels = [(0u16, 255u8, false); DISPLAY_WIDTH];
        let mut obj_has_pixel = [false; DISPLAY_WIDTH];
        let mut obj_window_mask = [false; DISPLAY_WIDTH];

        ppu.collect_sprites(
            0,
            &vram,
            &palette,
            &oam,
            &mut obj_pixels,
            &mut obj_has_pixel,
            &mut obj_window_mask,
        );

        assert!(obj_has_pixel[10]);
        assert_eq!(obj_pixels[10].0, 0x7FFF); // white
        assert_eq!(obj_pixels[10].1, 0); // priority 0
        assert!(!obj_pixels[10].2); // not semi-transparent
        assert!(!obj_has_pixel[9]); // pixel before sprite
    }

    #[test]
    fn test_collect_sprites_obj_window() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(1 << 6); // 1D mapping

        let mut vram = vec![0u8; 0x18000];
        let palette = vec![0u8; 0x400];
        let mut oam = vec![0u8; 0x400];

        // sprite with obj_mode=2 (OBJ window), 8x8 at (5, 0)
        let attr0: u16 = 2 << 10; // obj_mode=2
        let attr1: u16 = 5;
        let attr2: u16 = 0;
        oam[0] = (attr0 & 0xFF) as u8;
        oam[1] = ((attr0 >> 8) & 0xFF) as u8;
        oam[2] = (attr1 & 0xFF) as u8;
        oam[3] = ((attr1 >> 8) & 0xFF) as u8;
        oam[4] = (attr2 & 0xFF) as u8;
        oam[5] = ((attr2 >> 8) & 0xFF) as u8;

        // pixel data
        vram[0x10000] = 0x01;

        let mut obj_pixels = [(0u16, 255u8, false); DISPLAY_WIDTH];
        let mut obj_has_pixel = [false; DISPLAY_WIDTH];
        let mut obj_window_mask = [false; DISPLAY_WIDTH];

        ppu.collect_sprites(
            0,
            &vram,
            &palette,
            &oam,
            &mut obj_pixels,
            &mut obj_has_pixel,
            &mut obj_window_mask,
        );

        // OBJ window sprites set mask but don't render
        assert!(obj_window_mask[5]);
        assert!(!obj_has_pixel[5]);
    }

    #[test]
    fn test_collect_sprites_semi_transparent() {
        let mut ppu = GbaPpu::new();
        ppu.set_dispcnt(1 << 6);

        let mut vram = vec![0u8; 0x18000];
        let mut palette = vec![0u8; 0x400];
        let mut oam = vec![0u8; 0x400];

        // sprite with obj_mode=1 (semi-transparent), 8x8 at (0, 0)
        let attr0: u16 = 1 << 10; // obj_mode=1
        let attr1: u16 = 0;
        let attr2: u16 = 0;
        oam[0] = (attr0 & 0xFF) as u8;
        oam[1] = ((attr0 >> 8) & 0xFF) as u8;
        oam[2] = (attr1 & 0xFF) as u8;
        oam[3] = ((attr1 >> 8) & 0xFF) as u8;
        oam[4] = (attr2 & 0xFF) as u8;
        oam[5] = ((attr2 >> 8) & 0xFF) as u8;

        vram[0x10000] = 0x01;
        palette[0x202] = 0xFF;
        palette[0x203] = 0x7F;

        let mut obj_pixels = [(0u16, 255u8, false); DISPLAY_WIDTH];
        let mut obj_has_pixel = [false; DISPLAY_WIDTH];
        let mut obj_window_mask = [false; DISPLAY_WIDTH];

        ppu.collect_sprites(
            0,
            &vram,
            &palette,
            &oam,
            &mut obj_pixels,
            &mut obj_has_pixel,
            &mut obj_window_mask,
        );

        assert!(obj_has_pixel[0]);
        assert!(obj_pixels[0].2); // semi-transparent flag set
    }

    #[test]
    fn test_obj_mosaic_x() {
        let mut ppu = GbaPpu::new();
        ppu.set_mosaic(0x0300); // OBJ mosaic H=4 (bits 8-11 = 3 -> size 4)
        assert_eq!(ppu.apply_obj_mosaic_x(0), 0);
        assert_eq!(ppu.apply_obj_mosaic_x(3), 0);
        assert_eq!(ppu.apply_obj_mosaic_x(4), 4);
        assert_eq!(ppu.apply_obj_mosaic_x(7), 4);
    }
}
