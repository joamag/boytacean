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
    rom::Cartridge,
};

/// Base audio volume to be used as the basis of the
/// amplification level of the volume for the Game Boy.
const VOLUME: f32 = 64.0;

/// Audio normalization factor for the GBA, scaled proportionally
/// to the GB volume (web uses 100/32768, SDL uses 64/20972).
const VOLUME_GBA: f32 = 20972.0;

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
    /// Detects the system type from the ROM data and creates
    /// the appropriate emulator instance, ready to be run.
    ///
    /// The Game Boy runs in the mode preferred by the cartridge
    /// (CGB for CGB compatible cartridges) and starts with the
    /// default boot ROM of that mode.
    pub fn from_rom(data: &[u8]) -> Result<Self, Error> {
        if is_gba_rom(data) {
            let mut gba = GameBoyAdvance::new();
            gba.load_rom(data)?;
            Ok(System::Gba(gba))
        } else {
            let rom = Cartridge::from_data(data)?;
            let mut gb = GameBoy::new(Some(rom.gb_mode()));
            gb.load(true)?;
            gb.load_cartridge(rom)?;
            Ok(System::Gb(gb))
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
            System::Gb(gb) => gb.display_width(),
            System::Gba(gba) => gba.display_width(),
        }
    }

    pub fn display_height(&self) -> usize {
        match self {
            System::Gb(gb) => gb.display_height(),
            System::Gba(gba) => gba.display_height(),
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
                    title: rom.title(),
                    description: rom.description(9),
                })
            }
            System::Gba(gba) => {
                let data = boytacean_common::util::read_file(path)?;
                let info = gba.load_rom(&data)?;
                if let Some(ram_path) = ram_path {
                    gba.set_ram_data(boytacean_common::util::read_file(ram_path)?);
                }
                Ok(RomInfo {
                    title: info.title(),
                    description: info.description(9),
                })
            }
        }
    }

    /// If the cartridge of the loaded game has a battery, keeping
    /// its save RAM, which should then be persisted (eg: in a file).
    pub fn has_battery(&self) -> bool {
        match self {
            System::Gb(gb) => gb.rom_i().has_battery(),
            System::Gba(gba) => gba.has_battery(),
        }
    }

    /// Obtains the save RAM of the cartridge of the loaded game.
    pub fn ram_data(&self) -> &[u8] {
        match self {
            System::Gb(gb) => gb.rom_i().ram_data(),
            System::Gba(gba) => &gba.cpu.bus.save.data,
        }
    }

    /// Obtains the save RAM of the cartridge of the loaded game to be
    /// written, as a slice, so that its buffer (and the pointers to it,
    /// eg: the one of a libretro frontend) remains the same.
    pub fn ram_data_mut(&mut self) -> &mut [u8] {
        match self {
            System::Gb(gb) => gb.rom().ram_data_mut(),
            System::Gba(gba) => &mut gba.cpu.bus.save.data,
        }
    }

    pub fn save_ram_file(&self, path: &str) -> Result<(), Error> {
        if self.has_battery() {
            boytacean_common::util::write_file(path, self.ram_data(), None)?;
        }
        Ok(())
    }

    pub fn set_ppu_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_ppu_enabled(value),
            System::Gba(gba) => gba.set_ppu_enabled(value),
        }
    }

    pub fn ppu_enabled(&self) -> bool {
        match self {
            System::Gb(gb) => gb.ppu_enabled(),
            System::Gba(gba) => gba.ppu_enabled(),
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

    pub fn dma_enabled(&self) -> bool {
        match self {
            System::Gb(gb) => gb.dma_enabled(),
            System::Gba(gba) => gba.dma_enabled(),
        }
    }

    pub fn set_timer_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_timer_enabled(value),
            System::Gba(gba) => gba.set_timer_enabled(value),
        }
    }

    pub fn timer_enabled(&self) -> bool {
        match self {
            System::Gb(gb) => gb.timer_enabled(),
            System::Gba(gba) => gba.timer_enabled(),
        }
    }

    pub fn set_all_enabled(&mut self, value: bool) {
        match self {
            System::Gb(gb) => gb.set_all_enabled(value),
            System::Gba(gba) => gba.set_all_enabled(value),
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

    /// Obtains the audio volume of the system, the level of its samples,
    /// used by the frontends to normalize them, so that both systems
    /// play at the same level.
    pub fn audio_volume(&self) -> f32 {
        match self {
            System::Gb(_) => VOLUME,
            System::Gba(_) => VOLUME_GBA,
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
            System::Gb(gb) => gb.rom_i().title(),
            System::Gba(gba) => gba.rom_title(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            System::Gb(gb) => gb.reset(),
            System::Gba(gba) => gba.reset(),
        }
    }

    /// Restarts the loaded game, as the reset of the console does,
    /// keeping its ROM, its save RAM (in the same buffer, so that the
    /// pointers to it remain valid) and the held keys.
    pub fn reload(&mut self) -> Result<(), Error> {
        match self {
            System::Gb(gb) => gb.reload(),
            System::Gba(gba) => {
                gba.reset();
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{read, remove_file, write},
    };

    use super::System;
    use crate::{
        gb::{GameBoy, GameBoyMode},
        gba::GameBoyAdvance,
        pad::PadKey,
    };

    #[test]
    fn test_rom_info() {
        for (mut system, path, title) in [
            (
                System::Gb(GameBoy::new(None)),
                "res/roms/demo/pocket.gb",
                "POCKET-DEMO",
            ),
            (
                System::Gba(GameBoyAdvance::new()),
                "res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba",
                "GBA Tests",
            ),
        ] {
            let info = system.load_rom_file(path, None).unwrap();
            assert_eq!(info.title(), title);
            assert!(info.description().contains(title));
            assert_eq!(format!("{info}"), info.description());
        }
    }

    #[test]
    fn test_from_rom_gb() {
        let mut data = include_bytes!("../res/roms/demo/pocket.gb").to_vec();
        data[0xB2] = 0x96;
        let system = System::from_rom(&data).unwrap();
        if let System::Gb(gb) = system {
            assert_eq!(gb.rom_i().title(), "POCKET-DEMO");
            assert_eq!(gb.rom_i().read(0xB2), 0x96);
            assert_eq!(gb.mode(), GameBoyMode::Dmg);
        } else {
            panic!("Expected Game Boy system");
        }
    }

    #[test]
    fn test_from_rom_gb_cgb() {
        let data = include_bytes!("../res/roms/test/blargg/cpu/cpu_instrs.gb");
        let mut system = System::from_rom(data).unwrap();
        if let System::Gb(gb) = &system {
            assert_eq!(gb.mode(), GameBoyMode::Cgb);
        } else {
            panic!("Expected Game Boy system");
        }

        // the system is ready to run, starting with the boot ROM
        assert!(system.next_frame() > 0);
        assert_eq!(system.ppu_frame(), 1);
    }

    #[test]
    fn test_from_rom_gba() {
        let data = include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba");
        let system = System::from_rom(data).unwrap();
        assert!(system.is_gba());
    }

    #[test]
    fn test_from_rom_invalid() {
        assert!(System::from_rom(&[]).is_err());
    }

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
    fn test_visual_freq_gb() {
        let system = System::Gb(GameBoy::new(None));
        assert_eq!(system.visual_freq(), GameBoy::VISUAL_FREQ);
    }

    #[test]
    fn test_visual_freq_gba() {
        let system = System::Gba(GameBoyAdvance::new());
        assert!(system.visual_freq() > 59.0);
    }

    #[test]
    fn test_clock_gb() {
        let mut system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        let cycles = system.clock();
        assert!(cycles >= 4);
    }

    #[test]
    fn test_clock_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        let cycles = system.clock();
        assert!(cycles >= 1);
    }

    #[test]
    fn test_next_frame() {
        for data in [
            include_bytes!("../res/roms/demo/pocket.gb").as_slice(),
            include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba"),
        ] {
            let mut system = System::from_rom(data).unwrap();
            assert_eq!(system.ppu_frame(), 0);
            assert!(system.next_frame() > 0);
            assert_eq!(system.ppu_frame(), 1);
        }
    }

    #[test]
    fn test_clocks_cycles_gb() {
        let mut system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        let elapsed = system.clocks_cycles(100);
        assert!(elapsed >= 100);
    }

    #[test]
    fn test_clocks_cycles_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        let elapsed = system.clocks_cycles(100);
        assert!(elapsed >= 100);
    }

    #[test]
    fn test_frame_buffer_gb() {
        let mut system = System::Gb(GameBoy::new(None));
        assert_eq!(system.frame_buffer().len(), 160 * 144 * 3);
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
    fn test_clear_audio_buffer_gb() {
        let mut system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        system.next_frame();
        assert!(!system.audio_buffer().is_empty());
        system.clear_audio_buffer();
        assert!(system.audio_buffer().is_empty());
    }

    #[test]
    fn test_clear_audio_buffer_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.clear_audio_buffer();
        assert!(system.audio_buffer().is_empty());
    }

    #[test]
    fn test_key_press_lift_gb() {
        let mut system = System::Gb(GameBoy::new(None));
        if let System::Gb(gb) = &mut system {
            gb.pad().write(0xff00, 0x10); // selects the action keys
        }
        system.key_press(PadKey::A);
        if let System::Gb(gb) = &system {
            assert_eq!(gb.pad_i().read(0xff00) & 0x01, 0x00);
        }
        system.key_lift(PadKey::A);
        if let System::Gb(gb) = &system {
            assert_eq!(gb.pad_i().read(0xff00) & 0x01, 0x01);
        }
    }

    #[test]
    fn test_key_press_lift_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.key_press(PadKey::A);
        if let System::Gba(gba) = &system {
            assert_eq!(gba.cpu.bus.pad.keyinput() & 0x01, 0x00);
        }
        system.key_lift(PadKey::A);
        if let System::Gba(gba) = &system {
            assert_eq!(gba.cpu.bus.pad.keyinput() & 0x01, 0x01);
        }
    }

    #[test]
    fn test_load_rom_file_save_data() {
        let rom_path = temp_dir().join("boytacean_system_save_test.rom");
        let ram_path = temp_dir().join("boytacean_system_save_test.sav");
        for signature in [
            b"SRAM_V".as_slice(),
            b"FLASH_V",
            b"FLASH1M_V",
            b"EEPROM_V",
            b"GB",
        ] {
            let gb = signature == b"GB";
            let mut data = if gb {
                include_bytes!("../res/roms/demo/pocket.gb").to_vec()
            } else {
                let mut data = vec![0u8; 512];
                data[0xB2] = 0x96;
                data[0x100..0x100 + signature.len()].copy_from_slice(signature);
                data
            };
            if gb {
                data[0x147] = 0x1B; // mbc5+ram+battery
                data[0x149] = 0x03; // 32kb ram
            }
            write(&rom_path, data).unwrap();
            let mut system = if gb {
                System::Gb(GameBoy::new(None))
            } else {
                System::Gba(GameBoyAdvance::new())
            };
            let info = system
                .load_rom_file(rom_path.to_str().unwrap(), None)
                .unwrap();
            assert!(!info.description().is_empty());
            let mut ram = match &mut system {
                System::Gb(gb) => gb.ram_data_eager(),
                System::Gba(gba) => gba.ram_data_eager(),
            };
            ram[0] = 0x42;
            let last = ram.len() - 1;
            ram[last] = 0x24;
            match &mut system {
                System::Gb(gb) => gb.set_ram_data(ram.clone()),
                System::Gba(gba) => gba.set_ram_data(ram.clone()),
            }
            system.save_ram_file(ram_path.to_str().unwrap()).unwrap();
            assert_eq!(read(&ram_path).unwrap(), ram);
            system
                .load_rom_file(rom_path.to_str().unwrap(), Some(ram_path.to_str().unwrap()))
                .unwrap();
            let restored = match &mut system {
                System::Gb(gb) => gb.ram_data_eager(),
                System::Gba(gba) => gba.ram_data_eager(),
            };
            assert_eq!(restored, ram);
            assert!(system.save_ram_file(temp_dir().to_str().unwrap()).is_err());
            assert!(system
                .load_rom_file(
                    rom_path.to_str().unwrap(),
                    Some(temp_dir().to_str().unwrap())
                )
                .is_err());
        }
        remove_file(rom_path).unwrap();
        remove_file(ram_path).unwrap();
    }

    #[test]
    fn test_load_rom_file_errors() {
        let mut system = System::Gba(GameBoyAdvance::new());
        assert!(system
            .load_rom_file(temp_dir().to_str().unwrap(), None)
            .is_err());
        let path = temp_dir().join("boytacean_system_invalid_rom_test.gba");
        write(&path, [0]).unwrap();
        assert!(system.load_rom_file(path.to_str().unwrap(), None).is_err());
        remove_file(path).unwrap();
    }

    #[test]
    fn test_has_battery() {
        for (data, battery) in [
            (
                include_bytes!("../res/roms/game/shocklobster.gb").as_slice(),
                true,
            ),
            (
                include_bytes!("../res/roms/test/blargg/cgb_sound/cgb_sound.gb"),
                false,
            ),
            (
                include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/sram.gba"),
                true,
            ),
            (
                include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/none.gba"),
                false,
            ),
        ] {
            let system = System::from_rom(data).unwrap();
            assert_eq!(system.has_battery(), battery);
        }
    }

    #[test]
    fn test_ram_data() {
        let system = System::from_rom(include_bytes!("../res/roms/game/shocklobster.gb")).unwrap();
        if let System::Gb(gb) = &system {
            assert_eq!(system.ram_data().as_ptr(), gb.rom_i().ram_data().as_ptr());
        }
        assert_eq!(system.ram_data().len(), 8192);
        let system = System::from_rom(include_bytes!(
            "../res/roms.gba/test/jsmolka_gba-tests/sram.gba"
        ))
        .unwrap();
        if let System::Gba(gba) = &system {
            assert_eq!(system.ram_data().as_ptr(), gba.cpu.bus.save.data.as_ptr());
        }
        assert_eq!(system.ram_data().len(), 0x8000);
    }

    #[test]
    fn test_ram_data_mut() {
        // the writes to the save RAM are read by the game, in the same
        // buffer as the one of the save RAM to be read
        let mut system =
            System::from_rom(include_bytes!("../res/roms/game/shocklobster.gb")).unwrap();
        let buffer = system.ram_data().as_ptr();
        system.ram_data_mut()[1] = 0x42;
        assert_eq!(system.ram_data_mut().as_ptr(), buffer);
        if let System::Gb(gb) = &mut system {
            gb.write_memory(0x0000, 0x0a);
            assert_eq!(gb.read_memory(0xa001), 0x42);
        }
        let mut system = System::from_rom(include_bytes!(
            "../res/roms.gba/test/jsmolka_gba-tests/sram.gba"
        ))
        .unwrap();
        let buffer = system.ram_data().as_ptr();
        system.ram_data_mut()[1] = 0x42;
        assert_eq!(system.ram_data_mut().as_ptr(), buffer);
        if let System::Gba(gba) = &mut system {
            assert_eq!(gba.cpu.bus.read8(0x0e000001), 0x42);
        }
    }

    #[test]
    fn test_save_ram_file_without_battery() {
        for system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            assert!(system.save_ram_file(temp_dir().to_str().unwrap()).is_ok());
        }
    }

    #[test]
    fn test_set_ppu_enabled() {
        for mut system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            for enabled in [false, true] {
                system.set_ppu_enabled(enabled);
                assert_eq!(system.ppu_enabled(), enabled);
            }
        }
    }

    #[test]
    fn test_set_apu_enabled() {
        for mut system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            for enabled in [false, true] {
                system.set_apu_enabled(enabled);
                assert_eq!(system.apu_enabled(), enabled);
            }
        }
    }

    #[test]
    fn test_set_dma_enabled() {
        for mut system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            for enabled in [false, true] {
                system.set_dma_enabled(enabled);
                assert_eq!(system.dma_enabled(), enabled);
            }
        }
    }

    #[test]
    fn test_set_timer_enabled() {
        for mut system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            for enabled in [false, true] {
                system.set_timer_enabled(enabled);
                assert_eq!(system.timer_enabled(), enabled);
            }
        }
    }

    #[test]
    fn test_set_all_enabled() {
        for mut system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            for enabled in [false, true] {
                system.set_all_enabled(enabled);
                assert_eq!(system.ppu_enabled(), enabled);
                assert_eq!(system.apu_enabled(), enabled);
                assert_eq!(system.dma_enabled(), enabled);
                assert_eq!(system.timer_enabled(), enabled);
                match &system {
                    System::Gb(gb) => {
                        assert_eq!(gb.ppu_enabled(), enabled);
                        assert_eq!(gb.dma_enabled(), enabled);
                        assert_eq!(gb.timer_enabled(), enabled);
                    }
                    System::Gba(gba) => assert_eq!(gba.ppu_enabled(), enabled),
                }
            }
        }
    }

    #[test]
    fn test_multiplier() {
        for data in [
            include_bytes!("../res/roms/demo/pocket.gb").as_slice(),
            include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba"),
        ] {
            let system = System::from_rom(data).unwrap();
            assert_eq!(system.multiplier(), 1);
        }
    }

    #[test]
    fn test_audio_sampling_rate() {
        let system = System::Gb(GameBoy::new(None));
        assert_eq!(system.audio_sampling_rate(), 44100);
        let system = System::Gba(GameBoyAdvance::new());
        assert_eq!(system.audio_sampling_rate(), 32768);
    }

    #[test]
    fn test_audio_channels() {
        for system in [
            System::Gb(GameBoy::new(None)),
            System::Gba(GameBoyAdvance::new()),
        ] {
            assert_eq!(system.audio_channels(), 2);
        }
    }

    #[test]
    fn test_audio_volume() {
        let system = System::Gb(GameBoy::new(None));
        assert_eq!(system.audio_volume(), 64.0);
        let system = System::Gba(GameBoyAdvance::new());
        assert_eq!(system.audio_volume(), 20972.0);
    }

    #[test]
    fn test_description_debug() {
        let system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        if let System::Gb(gb) = &system {
            assert_eq!(system.description_debug(), gb.description_debug());
        }
        let system = System::from_rom(include_bytes!(
            "../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba"
        ))
        .unwrap();
        assert_eq!(
            system.description_debug(),
            "GBA [GBA Tests] CPU: 16777216 Hz, PPU: 59.73 Hz"
        );
    }

    #[test]
    fn test_rom_title() {
        let system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        assert_eq!(system.rom_title(), "POCKET-DEMO");
        let system = System::from_rom(include_bytes!(
            "../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba"
        ))
        .unwrap();
        assert_eq!(system.rom_title(), "GBA Tests");
    }

    #[test]
    fn test_reset_gb() {
        let mut system = System::from_rom(include_bytes!("../res/roms/demo/pocket.gb")).unwrap();
        system.next_frame();
        system.reset();
        assert_eq!(system.ppu_frame(), 0);
    }

    #[test]
    fn test_reset_gba() {
        let mut system = System::Gba(GameBoyAdvance::new());
        system.clock();
        system.reset();
        assert_eq!(system.ppu_frame(), 0);
    }

    #[test]
    fn test_reload_gb() {
        let mut system =
            System::from_rom(include_bytes!("../res/roms/game/shocklobster.gb")).unwrap();
        let buffer = if let System::Gb(gb) = &mut system {
            gb.rom().ram_data_mut()[0] = 0x42;
            gb.rom_i().ram_data().as_ptr()
        } else {
            panic!("Expected Game Boy system");
        };
        let title = system.rom_title();
        system.next_frame();
        system.key_press(PadKey::A);
        system.reload().unwrap();

        // the game restarts with its ROM, its save RAM (in the same
        // buffer) and the held keys
        assert_eq!(system.ppu_frame(), 0);
        assert_eq!(system.rom_title(), title);
        if let System::Gb(gb) = &mut system {
            assert_eq!(gb.rom_i().ram_data().as_ptr(), buffer);
            assert_eq!(gb.rom_i().ram_data()[0], 0x42);
            gb.pad().write(0xff00, 0x10); // selects the action keys
            assert_eq!(gb.pad_i().read(0xff00) & 0x01, 0x00);
        }
        assert!(system.next_frame() > 0);
    }

    #[test]
    fn test_reload_gba() {
        let mut system = System::from_rom(include_bytes!(
            "../res/roms.gba/test/jsmolka_gba-tests/sram.gba"
        ))
        .unwrap();
        let buffer = if let System::Gba(gba) = &mut system {
            gba.cpu.bus.save.data[0] = 0x42;
            gba.cpu.bus.save.data.as_ptr()
        } else {
            panic!("Expected GBA system");
        };
        system.next_frame();
        system.key_press(PadKey::A);
        system.reload().unwrap();

        // the game restarts with its ROM, its save RAM (in the same
        // buffer) and the held keys
        assert_eq!(system.ppu_frame(), 0);
        assert_eq!(system.rom_title(), "GBA Tests");
        if let System::Gba(gba) = &system {
            assert_eq!(gba.cpu.bus.save.data.as_ptr(), buffer);
            assert_eq!(gba.cpu.bus.save.data[0], 0x42);
            assert_eq!(gba.cpu.bus.pad.keyinput() & 0x01, 0x00);
        }
        assert!(system.next_frame() > 0);
    }
}
