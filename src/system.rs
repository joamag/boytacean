//! Unified system abstraction for running either a Game Boy
//! or a Game Boy Advance emulator through a single interface.
//!
//! The [`System`] enum wraps both [`GameBoy`] and [`GameBoyAdvance`]
//! behind a common API, allowing frontends (SDL, web, console) to
//! drive either emulator without branching on the system type.
//!
//! ROM loading returns a [`RomInfo`] value that carries both the
//! cartridge title (for window titles) and a multi-line description
//! (for startup banner printing).

use std::{
    collections::VecDeque,
    fmt::{self, Display, Formatter},
};

use boytacean_common::error::Error;

use crate::{
    gb::{AudioProvider, GameBoy},
    gba::{rom::is_gba_rom, GameBoyAdvance},
    pad::PadKey,
};

/// ROM/Cartridge information returned by [`System::load_rom_file`].
///
/// Provides both the title (for window titles, etc.) and the full
/// description (for banner printing), mirroring the `Cartridge`
/// type used by the Game Boy path.
pub struct RomInfo {
    title: String,
    description: String,
}

impl RomInfo {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Display for RomInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

/// Unified system enum wrapping either a Game Boy or
/// a Game Boy Advance emulator instance.
#[allow(clippy::large_enum_variant)]
pub enum System {
    Gb(GameBoy),
    Gba(GameBoyAdvance),
}

impl System {
    /// detects the system type from the ROM data and creates
    /// the appropriate emulator instance
    pub fn from_rom(data: &[u8]) -> Result<Self, Error> {
        if is_gba_rom(data) {
            let mut gba = GameBoyAdvance::new();
            gba.load_rom(data)?;
            Ok(System::Gba(gba))
        } else {
            Ok(System::Gb(GameBoy::new(None)))
        }
    }

    pub fn is_gba(&self) -> bool {
        matches!(self, System::Gba(_))
    }

    pub fn is_gb(&self) -> bool {
        matches!(self, System::Gb(_))
    }

    pub fn display_width(&self) -> usize {
        match self {
            System::Gb(_) => 160,
            System::Gba(_) => 240,
        }
    }

    pub fn display_height(&self) -> usize {
        match self {
            System::Gb(_) => 144,
            System::Gba(_) => 160,
        }
    }

    pub fn cpu_freq(&self) -> u32 {
        match self {
            System::Gb(_) => GameBoy::CPU_FREQ,
            System::Gba(gba) => gba.cpu_freq(),
        }
    }

    pub fn visual_freq(&self) -> f32 {
        match self {
            System::Gb(_) => GameBoy::VISUAL_FREQ,
            System::Gba(gba) => gba.visual_freq(),
        }
    }

    pub fn clock(&mut self) -> u32 {
        match self {
            System::Gb(gb) => gb.clock() as u32,
            System::Gba(gba) => gba.clock(),
        }
    }

    pub fn next_frame(&mut self) -> u64 {
        match self {
            System::Gb(gb) => gb.next_frame() as u64,
            System::Gba(gba) => gba.next_frame(),
        }
    }

    pub fn clocks_cycles(&mut self, limit: usize) -> u64 {
        match self {
            System::Gb(gb) => gb.clocks_cycles(limit),
            System::Gba(gba) => gba.clocks_cycles(limit),
        }
    }

    pub fn frame_buffer(&mut self) -> &[u8] {
        match self {
            System::Gb(gb) => gb.frame_buffer(),
            System::Gba(gba) => gba.frame_buffer(),
        }
    }

    pub fn ppu_frame(&mut self) -> u64 {
        match self {
            System::Gb(gb) => gb.ppu_frame() as u64,
            System::Gba(gba) => gba.ppu_frame(),
        }
    }

    pub fn audio_buffer(&mut self) -> &VecDeque<i16> {
        match self {
            System::Gb(gb) => gb.audio_buffer(),
            System::Gba(gba) => gba.audio_buffer(),
        }
    }

    pub fn clear_audio_buffer(&mut self) {
        match self {
            System::Gb(gb) => gb.clear_audio_buffer(),
            System::Gba(gba) => gba.clear_audio_buffer(),
        }
    }

    pub fn key_press(&mut self, key: PadKey) {
        match self {
            System::Gb(gb) => gb.key_press(key),
            System::Gba(gba) => gba.key_press(key),
        }
    }

    pub fn key_lift(&mut self, key: PadKey) {
        match self {
            System::Gb(gb) => gb.key_lift(key),
            System::Gba(gba) => gba.key_lift(key),
        }
    }

    pub fn load_rom_file(&mut self, path: &str, ram_path: Option<&str>) -> Result<RomInfo, Error> {
        match self {
            System::Gb(gb) => {
                let rom = gb.load_rom_file(path, ram_path)?;
                Ok(RomInfo {
                    title: rom.title().to_string(),
                    description: rom.description(9),
                })
            }
            System::Gba(gba) => {
                let data = boytacean_common::util::read_file(path)?;
                let info = gba.load_rom(&data)?;
                Ok(RomInfo {
                    title: info.title(),
                    description: info.description(9),
                })
            }
        }
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_ppu_enabled(value),
            System::Gba(gba) => gba.set_ppu_enabled(value),
        }
    }

    pub fn set_apu_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_apu_enabled(value),
            System::Gba(gba) => gba.set_apu_enabled(value),
        }
    }

    pub fn apu_enabled(&self) -> bool {
        match self {
            System::Gb(gb) => gb.apu_enabled(),
            System::Gba(gba) => gba.apu_enabled(),
        }
    }

    pub fn set_dma_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_dma_enabled(value),
            System::Gba(gba) => gba.set_dma_enabled(value),
        }
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_timer_enabled(value),
            System::Gba(gba) => gba.set_timer_enabled(value),
        }
    }

    pub fn set_all_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_all_enabled(value),
            System::Gba(gba) => {
                gba.set_ppu_enabled(value);
                gba.set_apu_enabled(value);
                gba.set_dma_enabled(value);
                gba.set_timer_enabled(value);
            }
        }
    }

    pub fn multiplier(&self) -> u32 {
        match self {
            System::Gb(gb) => gb.multiplier() as u32,
            System::Gba(_) => 1,
        }
    }

    pub fn audio_sampling_rate(&self) -> u32 {
        match self {
            System::Gb(gb) => gb.audio_sampling_rate() as u32,
            System::Gba(gba) => gba.audio_sampling_rate(),
        }
    }

    pub fn audio_channels(&self) -> u8 {
        match self {
            System::Gb(gb) => gb.audio_channels(),
            System::Gba(gba) => gba.audio_channels(),
        }
    }

    pub fn description_debug(&self) -> String {
        match self {
            System::Gb(gb) => gb.description_debug(),
            System::Gba(gba) => {
                format!(
                    "GBA [{}] CPU: {} Hz, PPU: {:.2} Hz",
                    gba.rom_title(),
                    gba.cpu_freq(),
                    gba.visual_freq()
                )
            }
        }
    }

    pub fn rom_title(&self) -> String {
        match self {
            System::Gb(gb) => gb.rom_i().title().to_string(),
            System::Gba(gba) => gba.rom_title().to_string(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            System::Gb(gb) => gb.reset(),
            System::Gba(gba) => gba.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::System;
    use crate::{gb::GameBoy, gba::GameBoyAdvance, pad::PadKey};

    #[test]
    fn test_system_gb() {
        let system = System::Gb(GameBoy::new(None));
        assert!(system.is_gb());
        assert!(!system.is_gba());
        assert_eq!(system.display_width(), 160);
        assert_eq!(system.display_height(), 144);
    }

    #[test]
    fn test_system_gba() {
        let system = System::Gba(GameBoyAdvance::new());
        assert!(system.is_gba());
        assert!(!system.is_gb());
        assert_eq!(system.display_width(), 240);
        assert_eq!(system.display_height(), 160);
    }

    #[test]
    fn test_from_rom_gb() {
        let data = vec![0u8; 0x200];
        let system = System::from_rom(&data).unwrap();
        assert!(system.is_gb());
    }

    #[test]
    fn test_from_rom_gba() {
        let mut data = vec![0u8; 0x200];
        data[0xB2] = 0x96;
        let system = System::from_rom(&data).unwrap();
        assert!(system.is_gba());
    }

    #[test]
    fn test_cpu_freq_gb() {
        let system = System::Gb(GameBoy::new(None));
        assert_eq!(system.cpu_freq(), GameBoy::CPU_FREQ);
    }

    #[test]
    fn test_cpu_freq_gba() {
        let system = System::Gba(GameBoyAdvance::new());
        assert_eq!(system.cpu_freq(), 16_777_216);
    }

    #[test]
    fn test_visual_freq_gba() {
        let system = System::Gba(GameBoyAdvance::new());
        assert!(system.visual_freq() > 59.0);
    }

    #[test]
    fn test_clock_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        let cycles = system.clock();
        assert!(cycles >= 1);
    }

    #[test]
    fn test_clocks_cycles_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        let elapsed = system.clocks_cycles(100);
        assert!(elapsed >= 100);
    }

    #[test]
    fn test_frame_buffer_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        assert_eq!(system.frame_buffer().len(), 240 * 160 * 3);
    }

    #[test]
    fn test_ppu_frame_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        assert_eq!(system.ppu_frame(), 0);
    }

    #[test]
    fn test_audio_buffer_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        assert!(system.audio_buffer().is_empty());
    }

    #[test]
    fn test_clear_audio_buffer_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.clear_audio_buffer();
        assert!(system.audio_buffer().is_empty());
    }

    #[test]
    fn test_key_press_lift_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.key_press(PadKey::A);
        system.key_lift(PadKey::A);
    }

    #[test]
    fn test_reset_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.clock();
        system.reset();
        assert_eq!(system.ppu_frame(), 0);
    }
}
