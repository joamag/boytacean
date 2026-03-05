//! Game Boy Advance emulation entrypoint and associated structures.
//!
//! Provides the [`GameBoyAdvance`] struct as the main entry-point API
//! for GBA emulation, mirroring the role of [`GameBoy`](crate::gb::GameBoy)
//! for the original Game Boy.

pub mod apu;
pub mod arm;
pub mod bios;
pub mod bus;
pub mod consts;
pub mod cpu;
pub mod diag;
pub mod dma;
pub mod irq;
pub mod pad;
pub mod ppu;
pub mod rom;
pub mod thumb;
pub mod timer;

use std::collections::VecDeque;

use boytacean_common::error::Error;

use self::{
    bus::GbaBus,
    consts::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
    cpu::Arm7Tdmi,
    rom::GbaRomInfo,
};
use crate::pad::PadKey;

/// Top level structure that abstracts the usage of the
/// Game Boy Advance system under the Boytacean emulator.
///
/// Should serve as the main entry-point API.
pub struct GameBoyAdvance {
    /// the ARM7TDMI CPU (includes the memory bus)
    pub cpu: Arm7Tdmi,

    /// frame counter tracking completed frames
    frame: u64,

    /// if the PPU is enabled, it will be clocked
    ppu_enabled: bool,

    /// if the APU is enabled, it will be clocked
    apu_enabled: bool,

    /// if DMA is enabled, it will be processed
    dma_enabled: bool,

    /// if timers are enabled, they will be clocked
    timer_enabled: bool,

    /// ROM title extracted from the header
    rom_title: String,
}

impl GameBoyAdvance {
    /// CPU clock frequency: 16.78 MHz
    pub const CPU_FREQ: u32 = consts::CPU_FREQ;

    /// visual refresh frequency (~59.7275 Hz)
    pub const VISUAL_FREQ: f32 = consts::VISUAL_FREQ;

    /// display width in pixels
    pub const DISPLAY_WIDTH: usize = DISPLAY_WIDTH;

    /// display height in pixels
    pub const DISPLAY_HEIGHT: usize = DISPLAY_HEIGHT;

    pub fn new() -> Self {
        let bus = GbaBus::new();
        let cpu = Arm7Tdmi::new(bus);
        Self {
            cpu,
            frame: 0,
            ppu_enabled: true,
            apu_enabled: true,
            dma_enabled: true,
            timer_enabled: true,
            rom_title: String::new(),
        }
    }

    /// loads a ROM from a byte slice
    pub fn load_rom(&mut self, data: &[u8]) -> Result<GbaRomInfo, Error> {
        let info = GbaRomInfo::from_data(data)?;
        self.rom_title = info.title().to_string();
        self.cpu.bus.load_rom(data);
        self.cpu.bus.postflg = 1; // mark as post-boot
        Ok(info)
    }

    /// advance the clock by one CPU instruction, clocking all subsystems
    pub fn clock(&mut self) -> u32 {
        // handle halt state
        if self.cpu.bus.halt_requested {
            self.cpu.set_halted(true);
            self.cpu.bus.halt_requested = false;
        }

        // execute one CPU instruction (or idle if halted)
        let cycles = self.cpu.step();

        // clock timers
        if self.timer_enabled {
            let overflows = self.cpu.bus.timers.clock(cycles);
            for i in 0..4 {
                if overflows & (1 << i) != 0 {
                    if self.cpu.bus.timers.timers[i].irq_enable() {
                        self.cpu.bus.irq.raise_timer(i);
                    }
                    // timer overflow triggers DirectSound FIFO
                    if self.apu_enabled {
                        self.cpu.bus.apu.timer_overflow(i);
                        // trigger DMA FIFO refill if needed
                        for fifo in 0..2 {
                            if self.cpu.bus.apu.direct_sound[fifo].timer_id == i
                                && self.cpu.bus.apu.direct_sound[fifo].needs_refill()
                            {
                                self.cpu.bus.dma.trigger_sound_fifo(fifo);
                            }
                        }
                    }
                }
            }
        }

        // clock PPU
        if self.ppu_enabled {
            let events = self.cpu.bus.ppu.clock(
                cycles,
                &self.cpu.bus.vram,
                &self.cpu.bus.palette,
                &self.cpu.bus.oam,
            );

            if events & 1 != 0 {
                // hblank
                self.cpu.bus.irq.raise_hblank();
                if self.dma_enabled {
                    self.cpu.bus.dma.trigger_hblank();
                }
            }
            if events & 2 != 0 {
                // vblank
                self.frame = self.cpu.bus.ppu.frame();
                self.cpu.bus.irq.raise_vblank();
                if self.dma_enabled {
                    self.cpu.bus.dma.trigger_vblank();
                }
            }
        }

        // clock APU
        if self.apu_enabled {
            self.cpu.bus.apu.clock(cycles);
        }

        // process DMA transfers
        if self.dma_enabled {
            self.process_dma();
        }

        // check keypad interrupt
        if self.cpu.bus.pad.int_keypad() {
            self.cpu.bus.irq.raise_keypad();
            self.cpu.bus.pad.ack_keypad();
        }

        cycles
    }

    /// processes pending DMA transfers
    fn process_dma(&mut self) {
        while let Some(index) = self.cpu.bus.dma.highest_active() {
            let channel = &mut self.cpu.bus.dma.channels[index];
            if !channel.active() {
                break;
            }

            let (src, dst, complete) = channel.step();

            if channel.word_size() {
                let value = self.cpu.bus.read32(src);
                self.cpu.bus.write32(dst, value);
            } else {
                let value = self.cpu.bus.read16(src);
                self.cpu.bus.write16(dst, value);
            }

            if complete {
                if self.cpu.bus.dma.channels[index].irq_enable() {
                    self.cpu.bus.irq.raise_dma(index);
                }
                break;
            }
        }
    }

    /// clocks the emulator until a full frame is completed
    pub fn next_frame(&mut self) -> u64 {
        let mut cycles = 0u64;
        let last_frame = self.frame;
        while self.frame == last_frame {
            cycles += self.clock() as u64;
        }
        cycles
    }

    /// clocks the emulator for the given number of cycles
    pub fn clocks_cycles(&mut self, limit: usize) -> u64 {
        let mut cycles = 0u64;
        while cycles < limit as u64 {
            cycles += self.clock() as u64;
        }
        cycles
    }

    /// returns the current frame buffer (RGB888)
    pub fn frame_buffer(&self) -> &[u8] {
        self.cpu.bus.ppu.frame_buffer()
    }

    /// returns the current frame number
    pub fn ppu_frame(&self) -> u64 {
        self.frame
    }

    /// returns a reference to the audio buffer
    pub fn audio_buffer(&self) -> &VecDeque<i16> {
        self.cpu.bus.apu.audio_buffer()
    }

    /// clears the audio buffer
    pub fn clear_audio_buffer(&mut self) {
        self.cpu.bus.apu.clear_audio_buffer();
    }

    pub fn key_press(&mut self, key: PadKey) {
        self.cpu.bus.pad.key_press(key);
    }

    pub fn key_lift(&mut self, key: PadKey) {
        self.cpu.bus.pad.key_lift(key);
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        self.ppu_enabled = value;
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        self.apu_enabled = value;
    }

    pub fn set_dma_enabled(&mut self, value: bool) {
        self.dma_enabled = value;
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        self.timer_enabled = value;
    }

    pub fn cpu_freq(&self) -> u32 {
        Self::CPU_FREQ
    }

    pub fn visual_freq(&self) -> f32 {
        Self::VISUAL_FREQ
    }

    pub fn display_width(&self) -> usize {
        Self::DISPLAY_WIDTH
    }

    pub fn display_height(&self) -> usize {
        Self::DISPLAY_HEIGHT
    }

    pub fn rom_title(&self) -> &str {
        &self.rom_title
    }

    pub fn apu_enabled(&self) -> bool {
        self.apu_enabled
    }

    pub fn audio_sampling_rate(&self) -> u32 {
        32768
    }

    pub fn audio_channels(&self) -> u8 {
        2
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cpu.bus.reset();
        self.frame = 0;
    }
}

impl Default for GameBoyAdvance {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::GameBoyAdvance;
    use crate::pad::PadKey;

    #[test]
    fn test_new_system() {
        let gba = GameBoyAdvance::new();
        assert_eq!(gba.display_width(), 240);
        assert_eq!(gba.display_height(), 160);
        assert_eq!(gba.cpu_freq(), 16_777_216);
        assert_eq!(gba.ppu_frame(), 0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(GameBoyAdvance::CPU_FREQ, 16_777_216);
        assert_eq!(GameBoyAdvance::DISPLAY_WIDTH, 240);
        assert_eq!(GameBoyAdvance::DISPLAY_HEIGHT, 160);
        assert!(GameBoyAdvance::VISUAL_FREQ > 59.0);
        assert!(GameBoyAdvance::VISUAL_FREQ < 60.0);
    }

    #[test]
    fn test_frame_buffer_size() {
        let gba = GameBoyAdvance::new();
        assert_eq!(gba.frame_buffer().len(), 240 * 160 * 3);
    }

    #[test]
    fn test_clock() {
        let mut gba = GameBoyAdvance::new();
        let cycles = gba.clock();
        assert!(cycles >= 1);
    }

    #[test]
    fn test_clocks_cycles() {
        let mut gba = GameBoyAdvance::new();
        let elapsed = gba.clocks_cycles(100);
        assert!(elapsed >= 100);
    }

    #[test]
    fn test_audio_buffer() {
        let gba = GameBoyAdvance::new();
        assert!(gba.audio_buffer().is_empty());
    }

    #[test]
    fn test_clear_audio_buffer() {
        let mut gba = GameBoyAdvance::new();
        gba.clear_audio_buffer();
        assert!(gba.audio_buffer().is_empty());
    }

    #[test]
    fn test_key_press_and_lift() {
        let mut gba = GameBoyAdvance::new();
        gba.key_press(PadKey::A);
        gba.key_lift(PadKey::A);
    }

    #[test]
    fn test_set_ppu_enabled() {
        let mut gba = GameBoyAdvance::new();
        gba.set_ppu_enabled(false);
        gba.clock();
        gba.set_ppu_enabled(true);
    }

    #[test]
    fn test_set_apu_enabled() {
        let mut gba = GameBoyAdvance::new();
        gba.set_apu_enabled(false);
        gba.clock();
        gba.set_apu_enabled(true);
    }

    #[test]
    fn test_set_dma_enabled() {
        let mut gba = GameBoyAdvance::new();
        gba.set_dma_enabled(false);
        gba.clock();
        gba.set_dma_enabled(true);
    }

    #[test]
    fn test_set_timer_enabled() {
        let mut gba = GameBoyAdvance::new();
        gba.set_timer_enabled(false);
        gba.clock();
        gba.set_timer_enabled(true);
    }

    #[test]
    fn test_visual_freq() {
        let gba = GameBoyAdvance::new();
        assert!(gba.visual_freq() > 59.0);
    }

    #[test]
    fn test_reset() {
        let mut gba = GameBoyAdvance::new();
        gba.clock();
        gba.reset();
        assert_eq!(gba.ppu_frame(), 0);
    }

    #[test]
    fn test_default() {
        let gba = GameBoyAdvance::default();
        assert_eq!(gba.display_width(), 240);
    }
}
