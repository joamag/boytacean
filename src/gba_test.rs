//! GBA interactive testing building blocks.
//!
//! Provides functions to run GBA test ROMs in headless mode,
//! mirroring the Game Boy test infrastructure in [`crate::test`].

use boytacean_common::error::Error;

use crate::gba::GameBoyAdvance;

#[derive(Default)]
pub struct GbaTestOptions {
    pub ppu_enabled: Option<bool>,
    pub apu_enabled: Option<bool>,
    pub dma_enabled: Option<bool>,
    pub timer_enabled: Option<bool>,
}

pub fn build_gba_test(options: GbaTestOptions) -> Box<GameBoyAdvance> {
    let mut gba = Box::new(GameBoyAdvance::new());
    gba.set_ppu_enabled(options.ppu_enabled.unwrap_or(true));
    gba.set_apu_enabled(options.apu_enabled.unwrap_or(true));
    gba.set_dma_enabled(options.dma_enabled.unwrap_or(true));
    gba.set_timer_enabled(options.timer_enabled.unwrap_or(true));
    gba
}

pub fn run_gba_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: GbaTestOptions,
) -> Result<Box<GameBoyAdvance>, Error> {
    let max_cycles = max_cycles.unwrap_or(u64::MAX);
    let mut gba = build_gba_test(options);
    let data = std::fs::read(rom_path)
        .map_err(|e| Error::CustomError(format!("Failed to read ROM: {e}")))?;
    gba.load_rom(&data)?;
    gba.clocks_cycles(max_cycles as usize);
    Ok(gba)
}

pub fn run_gba_image_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: GbaTestOptions,
) -> Result<(Vec<u8>, Box<GameBoyAdvance>), Error> {
    let gba = run_gba_test(rom_path, max_cycles, options)?;
    let fb = gba.frame_buffer().to_vec();
    Ok((fb, gba))
}

#[cfg(test)]
mod tests {
    use super::{run_gba_test, GbaTestOptions};

    #[test]
    fn test_jsmolka_arm() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/arm.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_memory() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/memory.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_bios() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/bios.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_sram() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/sram.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_flash64() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/flash64.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_nes() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/nes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_thumb() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/thumb.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_flash128() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/flash128.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_none() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/none.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_unsafe() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/unsafe.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_ppu_hello() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_hello.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_ppu_shades() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_shades.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_jsmolka_ppu_stripes() {
        let gba = run_gba_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_alyosha_dma_rom_fixed() {
        let gba = run_gba_test(
            "res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }

    #[test]
    fn test_alyosha_dma_mode_change() {
        let gba = run_gba_test(
            "res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert!(gba.ppu_frame() > 0);
    }
}
