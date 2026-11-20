//! GBA interactive testing building blocks.
//!
//! Provides functions to run GBA test ROMs in headless mode,
//! mirroring the Game Boy test infrastructure in [`crate::test`].

use boytacean_common::{error::Error, util::read_file};

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
    let data = read_file(rom_path)?;
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
    use boytacean_hashing::crc32::crc32;

    use super::{run_gba_image_test, GbaTestOptions};

    /// CRC-32 of the frame buffer with the "All tests passed"
    /// screen drawn by the jsmolka (and alyosha-tas) test ROMs, as
    /// in the reference images of the SDL frontend tests.
    const PASSED_CRC32: u32 = 0x2F3C94D3;

    #[test]
    fn test_jsmolka_arm() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/arm.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_memory() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/memory.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_bios() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/bios.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_sram() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/sram.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_flash64() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/flash64.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_nes() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/nes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_thumb() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/thumb.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_flash128() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/flash128.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_none() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/none.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_unsafe() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/unsafe.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    fn test_jsmolka_ppu_hello() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_hello.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), 0xF035DC2E);
    }

    #[test]
    fn test_jsmolka_ppu_shades() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_shades.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), 0x8C4C63F4);
    }

    #[test]
    fn test_jsmolka_ppu_stripes() {
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), 0x3B3991F4);
    }

    #[test]
    #[ignore]
    fn test_alyosha_dma_rom_fixed() {
        // fails until the DMA and cartridge wait state timings are emulated
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_ROM_Fixed.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }

    #[test]
    #[ignore]
    fn test_alyosha_dma_mode_change() {
        // fails until the DMA and cartridge wait state timings are emulated
        let (frame_buffer, _) = run_gba_image_test(
            "res/roms.gba/test/alyosha-tas_gba-tests/DMA/DMA_Mode_Change.gba",
            Some(100000000),
            GbaTestOptions::default(),
        )
        .unwrap();
        assert_eq!(crc32(&frame_buffer), PASSED_CRC32);
    }
}
