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
pub mod flash;
pub mod irq;
pub mod pad;
pub mod ppu;
pub mod rom;
pub mod thumb;
pub mod timer;

use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};

#[cfg(feature = "wasm")]
use std::panic::{set_hook, take_hook, PanicInfo};

use boytacean_common::error::Error;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use self::{
    bus::GbaBus,
    consts::{DISPLAY_HEIGHT, DISPLAY_WIDTH, EWRAM_SIZE, IWRAM_SIZE, VRAM_SIZE},
    cpu::Arm7Tdmi,
    flash::SaveType,
    rom::GbaRomInfo,
};
use crate::{info::Info, pad::PadKey};

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GbaClockFrame {
    pub cycles: u64,
    pub frames: u16,
    frame_buffer: Option<Vec<u8>>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GbaClockFrame {
    pub fn frame_buffer_eager(&mut self) -> Option<Vec<u8>> {
        self.frame_buffer.take()
    }
}

/// Top level structure that abstracts the usage of the
/// Game Boy Advance system under the Boytacean emulator.
///
/// Should serve as the main entry-point API.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameBoyAdvance {
    /// the ARM7TDMI CPU (includes the memory bus)
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
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

    /// ROM information extracted from the header
    rom_info: Option<GbaRomInfo>,
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

    /// Loads a real BIOS ROM from a byte slice.
    ///
    /// When loaded, the CPU will boot from address 0x00000000 (BIOS entry)
    /// and SWI instructions will execute through the real BIOS code.
    pub fn load_bios(&mut self, data: &[u8]) {
        self.cpu.bus.load_bios(data);

        // resets CPU to boot from BIOS start (address 0x00000000)
        // the BIOS will initialize registers, run checksums, and
        // jump to the ROM entry point at 0x08000000.
        self.cpu.reset_for_bios_boot();
    }

    /// Loads a ROM from a byte slice, returning the extracted
    /// ROM information as a result.
    pub fn load_rom(&mut self, data: &[u8]) -> Result<GbaRomInfo, Error> {
        let info = GbaRomInfo::from_data(data)?;
        self.rom_info = Some(info.clone());
        self.cpu.bus.load_rom(data);
        if !self.cpu.bus.use_real_bios {
            self.cpu.bus.postflg = 1; // mark as post-boot
        }
        Ok(info)
    }

    /// Returns the current frame buffer (RGB888)
    pub fn frame_buffer(&self) -> &[u8] {
        self.cpu.bus.ppu.frame_buffer()
    }

    /// Returns a reference to the audio buffer
    pub fn audio_buffer(&self) -> &VecDeque<i16> {
        self.cpu.bus.apu.audio_buffer()
    }

    /// Returns the ROM information if a ROM has been loaded
    pub fn rom_info(&self) -> Option<&GbaRomInfo> {
        self.rom_info.as_ref()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoyAdvance {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
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
            rom_info: None,
        }
    }

    /// Advances the clock by one CPU instruction, clocking all subsystems.
    /// Returns the number of cycles elapsed during this clock.
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
            // debug: no trace
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

        // clock PPU, retrieves the events that occurred during
        // this clock to trigger related behavior
        if self.ppu_enabled {
            let events = self.cpu.bus.ppu.clock(
                cycles,
                &self.cpu.bus.vram,
                &self.cpu.bus.palette,
                &self.cpu.bus.oam,
            );

            if events & 1 != 0 {
                // hblank DMA trigger (always fires at hblank)
                if self.dma_enabled {
                    self.cpu.bus.dma.trigger_hblank();
                }
            }
            if events & 4 != 0 {
                // hblank IRQ (only when DISPSTAT enables it)
                self.cpu.bus.irq.raise_hblank();
            }
            if events & 2 != 0 {
                // vblank DMA trigger (always fires at vblank)
                self.frame = self.cpu.bus.ppu.frame();
                if self.dma_enabled {
                    self.cpu.bus.dma.trigger_vblank();
                }
            }
            if events & 8 != 0 {
                // vblank IRQ (only when DISPSTAT enables it)
                self.cpu.bus.irq.raise_vblank();
            }
            if events & 16 != 0 {
                // delivers VCount match IRQ to the controller, relies on
                // the IntrWait re-halt check in cpu.rs (gated on CPSR_I==0)
                // to prevent premature unhalt of VBlankIntrWait callers.
                self.cpu.bus.irq.raise_vcount();
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

            let word32 = channel.word_size();
            let (src, dst, complete) = channel.step();

            if word32 {
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

    /// returns the current frame number
    pub fn ppu_frame(&self) -> u64 {
        self.frame
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

    pub fn rom_title(&self) -> String {
        self.rom_info
            .as_ref()
            .map(|info| info.title())
            .unwrap_or_default()
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
        self.cpu.bus.reset();
        if self.cpu.bus.use_real_bios {
            self.cpu.reset_for_bios_boot();
        } else {
            self.cpu.reset();
        }
        self.frame = 0;
    }

    pub fn clocks_frame_buffer(&mut self, limit: usize) -> GbaClockFrame {
        let mut cycles = 0_u64;
        let mut frames = 0_u16;
        let mut frame_buffer: Option<Vec<u8>> = None;
        let mut last_frame = self.ppu_frame();
        while cycles < limit as u64 {
            cycles += self.clock() as u64;
            if self.ppu_frame() != last_frame {
                frame_buffer = Some(self.frame_buffer().to_vec());
                last_frame = self.ppu_frame();
                frames += 1;
            }
        }
        GbaClockFrame {
            cycles,
            frames,
            frame_buffer,
        }
    }

    pub fn frame_buffer_eager(&self) -> Vec<u8> {
        self.frame_buffer().to_vec()
    }

    pub fn audio_buffer_eager(&mut self, clear: bool) -> Vec<i16> {
        let buffer = Vec::from(self.audio_buffer().clone());
        if clear {
            self.clear_audio_buffer();
        }
        buffer
    }

    pub fn has_battery(&self) -> bool {
        self.cpu.bus.save.save_type() != SaveType::None
    }

    pub fn ram_data_eager(&self) -> Vec<u8> {
        self.cpu.bus.save.data.clone()
    }

    pub fn set_ram_data(&mut self, data: Vec<u8>) {
        self.cpu.bus.save.data = data;
    }

    pub fn ppu_enabled(&self) -> bool {
        self.ppu_enabled
    }

    pub fn description(&self, column_length: usize) -> String {
        let version_l = format!("{:width$}", "Version", width = column_length);
        let mode_l = format!("{:width$}", "Mode", width = column_length);
        let boot_rom_l = format!("{:width$}", "Boot ROM", width = column_length);
        let clock_l = format!("{:width$}", "Clock", width = column_length);
        let ram_size_l = format!("{:width$}", "RAM Size", width = column_length);
        let vram_size_l = format!("{:width$}", "VRAM Size", width = column_length);
        format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {:.02} Mhz\n{}  {} KB\n{}  {} KB",
            version_l,
            Info::version(),
            mode_l,
            "Game Boy Advance",
            boot_rom_l,
            if self.cpu.bus.use_real_bios {
                "Real BIOS"
            } else {
                "HLE"
            },
            clock_l,
            Self::CPU_FREQ as f64 / 1_000_000.0,
            ram_size_l,
            (EWRAM_SIZE + IWRAM_SIZE) / 1024,
            vram_size_l,
            VRAM_SIZE / 1024,
        )
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GameBoyAdvance {
    pub fn set_panic_hook_wa() {
        let prev = take_hook();
        set_hook(Box::new(move |info| {
            gba_hook_impl(info);
            prev(info);
        }));
    }

    pub fn load_rom_wa(&mut self, data: &[u8]) -> Result<GbaRomInfo, String> {
        self.load_rom(data).map_err(|e| e.to_string())
    }

    pub fn verify_rom_wa(data: &[u8]) -> bool {
        rom::is_gba_rom(data)
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = panic)]
    fn gba_panic(message: &str);
}

#[cfg(feature = "wasm")]
pub fn gba_hook_impl(info: &PanicInfo) {
    let message = info.to_string();
    gba_panic(message.as_str());
}

impl Default for GameBoyAdvance {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for GameBoyAdvance {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description(9))
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
    #[allow(clippy::assertions_on_constants)]
    fn test_constants() {
        assert_eq!(GameBoyAdvance::CPU_FREQ, 16777216);
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

    #[test]
    fn test_load_bios() {
        let mut gba = GameBoyAdvance::new();
        let bios = vec![0u8; 0x4000];
        gba.load_bios(&bios);
        assert!(gba.cpu.bus.use_real_bios);
        // after loading BIOS, CPU boots from 0x00 in SVC mode
        assert_eq!(gba.cpu.pc(), 0x0000_0000);
        assert_eq!(gba.cpu.cpsr() & 0x1F, 0x13); // MODE_SVC
    }

    #[test]
    fn test_load_bios_after_rom() {
        let mut gba = GameBoyAdvance::new();
        // load a minimal ROM first
        let rom = vec![0u8; 256];
        let _ = gba.load_rom(&rom);
        assert_eq!(gba.cpu.pc(), 0x0800_0000);

        // loading BIOS resets PC to 0x00
        let bios = vec![0u8; 0x4000];
        gba.load_bios(&bios);
        assert_eq!(gba.cpu.pc(), 0x0000_0000);
    }

    #[test]
    fn test_has_battery_none() {
        let gba = GameBoyAdvance::new();
        assert!(!gba.has_battery());
    }

    #[test]
    fn test_has_battery_sram() {
        let mut gba = GameBoyAdvance::new();
        let mut rom = vec![0u8; 512];
        rom[0xB2] = 0x96;
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        let _ = gba.load_rom(&rom);
        assert!(gba.has_battery());
    }

    #[test]
    fn test_ram_data_eager() {
        let gba = GameBoyAdvance::new();
        let ram = gba.ram_data_eager();
        assert!(!ram.is_empty());
    }

    #[test]
    fn test_set_ram_data() {
        let mut gba = GameBoyAdvance::new();
        let mut rom = vec![0u8; 512];
        rom[0xB2] = 0x96;
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        let _ = gba.load_rom(&rom);

        let mut ram = gba.ram_data_eager();
        ram[0] = 0x42;
        ram[1] = 0xAB;
        gba.set_ram_data(ram);

        let restored = gba.ram_data_eager();
        assert_eq!(restored[0], 0x42);
        assert_eq!(restored[1], 0xAB);
    }

    #[test]
    fn test_vcount_irq_delivered_on_match() {
        use crate::gba::consts::{IRQ_VCOUNT, REG_DISPSTAT};

        let mut gba = GameBoyAdvance::new();
        // set LYC = 0 and enable VCount IRQ (bit 5) in DISPSTAT
        // DISPSTAT bits [8:15] = LYC, bit 5 = VCount IRQ enable
        gba.cpu.bus.write16(REG_DISPSTAT, (0 << 8) | (1 << 5));
        // enable VCount in IE and set IME
        gba.cpu.bus.irq.set_ie(IRQ_VCOUNT);
        gba.cpu.bus.irq.set_ime(true);

        // PPU starts at vcount=0, so after a full scanline the vcount
        // wraps and eventually matches LYC=0; clock enough cycles
        // to complete at least one full frame (228 scanlines × 1232 dots)
        let total: usize = 228 * 1232;
        gba.clocks_cycles(total);

        // VCount IRQ should have been raised in IF at some point
        assert!(gba.cpu.bus.irq.if_() & IRQ_VCOUNT != 0);
    }
}
