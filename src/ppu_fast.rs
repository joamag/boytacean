//! A "fast PPU" driver inspired by PyBoy's `lcd.py`.
//!
//! This is **not** a working PPU — it has no renderer, no STAT
//! interrupts, no LYC compare and no CGB support. Its only purpose
//! is to validate whether the per-CPU-instruction `clock(cycles)`
//! dispatch in the real PPU is itself a meaningful cost.
//!
//! The model differs from [`crate::ppu::Ppu::clock`] in one way: it
//! never enters the state machine until enough cycles have
//! accumulated to cross a mode boundary, by tracking
//! `clock_target` (a la PyBoy) instead of `mode_clock`. The state
//! transitions and durations match Boytacean's existing PPU
//! (mode 2 = 80, mode 3 = 172, mode 0 = 204, V-blank = 4560).
//!
//! Use [`FastPpu::clock`] in a benchmark harness against
//! [`crate::ppu::Ppu::clock`] to measure the driver-only delta.

/// PPU state-machine mode tracked by the fast driver, matching
/// the discriminants used by the LCD STAT register.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum FastMode {
    OamRead = 2,
    VramRead = 3,
    HBlank = 0,
    VBlank = 1,
}

pub struct FastPpu {
    /// Cumulative cycles since the start of the current scanline,
    /// reset every 456 cycles.
    pub clock: u32,

    /// Target cycle count for the next mode transition; mirrors
    /// PyBoy's `clock_target`.
    pub clock_target: u32,

    /// Current LCD line in 0..154.
    pub ly: u8,

    /// Current PPU mode.
    pub mode: FastMode,

    /// Frame index, advanced once per V-blank exit.
    pub frame_index: u32,

    /// Render-trigger flag: set by the driver every time it crosses
    /// the mode 3 -> HBlank boundary, so a host can call its real
    /// `render_line(ly)` only when needed.
    pub render_pending: bool,

    /// V-blank IRQ pending; mirrors `int_vblank` in the real PPU.
    pub int_vblank: bool,
}

impl Default for FastPpu {
    fn default() -> Self {
        Self::new()
    }
}

impl FastPpu {
    pub fn new() -> Self {
        Self {
            clock: 0,
            clock_target: 80,
            ly: 0,
            mode: FastMode::OamRead,
            frame_index: 0,
            render_pending: false,
            int_vblank: false,
        }
    }

    /// Advances the driver by `cycles` and updates the mode/LY
    /// machine, exactly like [`crate::ppu::Ppu::clock`] but without
    /// any rendering or interrupt-line book-keeping.
    ///
    /// Returns true once per scanline at the mode 3 -> HBlank edge
    /// (the host should call its renderer for `self.ly` then).
    #[inline(always)]
    pub fn clock(&mut self, cycles: u16) -> bool {
        self.clock += cycles as u32;
        let mut rendered = false;
        while self.clock >= self.clock_target {
            self.advance_mode(&mut rendered);
        }
        rendered
    }

    #[inline(always)]
    fn advance_mode(&mut self, rendered: &mut bool) {
        // every transition resets the clock window to "cycles into the
        // current scanline" so that clock/clock_target stay small and
        // wrap-around bookkeeping never drifts; this matches PyBoy's
        // lcd.py invariant where the counters are scanline-local
        match self.mode {
            FastMode::OamRead => {
                self.mode = FastMode::VramRead;
                self.clock = self.clock.saturating_sub(80);
                self.clock_target = 172;
            }
            FastMode::VramRead => {
                *rendered = true;
                self.render_pending = true;
                self.mode = FastMode::HBlank;
                self.clock = self.clock.saturating_sub(172);
                self.clock_target = 204;
            }
            FastMode::HBlank => {
                self.ly += 1;
                self.clock = self.clock.saturating_sub(204);
                if self.ly == 144 {
                    self.int_vblank = true;
                    self.mode = FastMode::VBlank;
                    self.clock_target = 456;
                } else {
                    self.mode = FastMode::OamRead;
                    self.clock_target = 80;
                }
            }
            FastMode::VBlank => {
                self.ly += 1;
                self.clock = self.clock.saturating_sub(456);
                if self.ly == 154 {
                    self.ly = 0;
                    self.mode = FastMode::OamRead;
                    self.frame_index = self.frame_index.wrapping_add(1);
                    self.clock_target = 80;
                } else {
                    self.clock_target = 456;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_frame_is_70224_cycles() {
        let mut d = FastPpu::new();
        let mut cycles = 0u32;
        let start_frame = d.frame_index;
        while d.frame_index == start_frame {
            d.clock(4);
            cycles += 4;
            assert!(cycles < 100_000, "frame did not complete");
        }
        // 154 lines * 456 cycles = 70224 per frame
        assert!(
            (70_220..=70_230).contains(&cycles),
            "frame took {} cycles",
            cycles
        );
    }

    #[test]
    fn renders_once_per_visible_line() {
        // step one cycle at a time so we never cross the V-blank wrap
        // and the first scanline of the next frame in a single call;
        // a 1-cycle granularity guarantees 144 renders are observed
        // before frame_index advances
        let mut fast_ppu = FastPpu::new();
        let mut renders = 0;
        let start_frame = fast_ppu.frame_index;
        while fast_ppu.frame_index == start_frame {
            if fast_ppu.clock(1) {
                renders += 1;
            }
        }
        assert_eq!(renders, 144, "expected 144 renders, got {}", renders);
    }
}
