use boytacean_common::error::Error;

use crate::{
    data::BootRom,
    devices::buffer::BufferDevice,
    gb::{GameBoy, GameBoyMode},
    ppu::FRAME_BUFFER_SIZE,
};

#[derive(Default)]
pub struct TestOptions {
    pub mode: Option<GameBoyMode>,
    pub ppu_enabled: Option<bool>,
    pub apu_enabled: Option<bool>,
    pub dma_enabled: Option<bool>,
    pub timer_enabled: Option<bool>,
    pub boot_rom: Option<BootRom>,
}

pub fn build_test(options: TestOptions) -> Box<GameBoy> {
    let device = Box::<BufferDevice>::default();
    let mut game_boy = Box::new(GameBoy::new(options.mode));
    game_boy.set_ppu_enabled(options.ppu_enabled.unwrap_or(true));
    game_boy.set_apu_enabled(options.apu_enabled.unwrap_or(true));
    game_boy.set_dma_enabled(options.dma_enabled.unwrap_or(true));
    game_boy.set_timer_enabled(options.timer_enabled.unwrap_or(true));
    game_boy.attach_serial(device);
    game_boy.load(false).unwrap();
    game_boy.load_boot_smart(options.boot_rom).unwrap();
    game_boy
}

pub fn run_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: TestOptions,
) -> Result<Box<GameBoy>, Error> {
    let max_cycles = max_cycles.unwrap_or(u64::MAX);
    let mut game_boy = build_test(options);
    game_boy.load_rom_file(rom_path, None)?;
    game_boy.clocks_cycles(max_cycles as usize);
    Ok(game_boy)
}

pub fn run_step_test(
    rom_path: &str,
    addr: u16,
    options: TestOptions,
) -> Result<Box<GameBoy>, Error> {
    let mut game_boy = build_test(options);
    game_boy.load_rom_file(rom_path, None)?;
    game_boy.step_to(addr);
    Ok(game_boy)
}

pub fn run_serial_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: TestOptions,
) -> Result<(String, Box<GameBoy>), Error> {
    let mut game_boy = run_test(rom_path, max_cycles, options)?;
    Ok((game_boy.serial().device().state(), game_boy))
}

pub fn run_image_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: TestOptions,
) -> Result<([u8; FRAME_BUFFER_SIZE], Box<GameBoy>), Error> {
    let mut game_boy = run_test(rom_path, max_cycles, options)?;
    Ok((*game_boy.frame_buffer(), game_boy))
}

#[cfg(test)]
mod tests {
    use crate::{
        consts::{
            BGP_ADDR, DIV_ADDR, DMA_ADDR, IF_ADDR, LCDC_ADDR, LYC_ADDR, LY_ADDR, OBP0_ADDR,
            OBP1_ADDR, SCX_ADDR, SCY_ADDR, STAT_ADDR, TAC_ADDR, TIMA_ADDR, TMA_ADDR, WX_ADDR,
            WY_ADDR,
        },
        data::BootRom,
        gb::GameBoyMode,
        licensee::Licensee,
        rom::{RamSize, Region, RomSize},
    };

    use super::{run_serial_test, run_step_test, TestOptions};

    #[test]
    fn test_boot_state() {
        let mut result = run_step_test(
            "res/roms/test/blargg/cpu/cpu_instrs.gb",
            0x0100,
            TestOptions {
                boot_rom: Some(BootRom::Dmg),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(result.cpu_i().pc(), 0x0100);
        assert_eq!(result.cpu_i().sp(), 0xfffe);
        assert_eq!(result.cpu_i().af(), 0x01b0);
        assert_eq!(result.cpu_i().bc(), 0x0013);
        assert_eq!(result.cpu_i().de(), 0x00d8);
        assert_eq!(result.cpu_i().hl(), 0x014d);
        assert!(!result.cpu_i().ime());

        assert_eq!(result.mmu().read(DIV_ADDR), 0xcf);
        assert_eq!(result.mmu().read(TIMA_ADDR), 0x00);
        assert_eq!(result.mmu().read(TMA_ADDR), 0x00);
        assert_eq!(result.mmu().read(TAC_ADDR), 0xf8);
        assert_eq!(result.mmu().read(IF_ADDR), 0xe1);

        assert_eq!(result.ppu().read(LCDC_ADDR), 0x91);
        assert_eq!(result.ppu().read(STAT_ADDR), 0x81);
        assert_eq!(result.ppu().read(SCY_ADDR), 0x00);
        assert_eq!(result.ppu().read(SCX_ADDR), 0x00);
        assert_eq!(result.ppu().read(LY_ADDR), 0x99);
        assert_eq!(result.ppu().read(LYC_ADDR), 0x00);
        assert_eq!(result.ppu().read(BGP_ADDR), 0xfc);
        assert_eq!(result.ppu().read(OBP0_ADDR), 0x00);
        assert_eq!(result.ppu().read(OBP1_ADDR), 0x00);
        assert_eq!(result.ppu().read(WX_ADDR), 0x00);
        assert_eq!(result.ppu().read(WY_ADDR), 0x00);

        assert_eq!(result.ppu().read(DMA_ADDR), 0xff);
    }

    #[test]
    fn test_blargg_cpu_instrs() {
        let (result, game_boy) = run_serial_test(
            "res/roms/test/blargg/cpu/cpu_instrs.gb",
            Some(300000000),
            TestOptions::default(),
        )
        .unwrap();
        assert_eq!(result, "cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests\n");
        assert_eq!(game_boy.rom_i().gb_mode(), GameBoyMode::Cgb);
        assert_eq!(game_boy.rom_i().title().as_str(), "CPU_INSTRS");
        assert_eq!(game_boy.rom_i().licensee(), Licensee::None);
        assert_eq!(game_boy.rom_i().region(), Region::Unknown);
        assert_eq!(game_boy.rom_i().rom_size(), RomSize::Size64K);
        assert_eq!(game_boy.rom_i().ram_size(), RamSize::NoRam);
        assert!(game_boy.rom_i().valid_checksum());
    }

    #[test]
    fn test_blargg_instr_timing() {
        let (result, game_boy) = run_serial_test(
            "res/roms/test/blargg/instr_timing/instr_timing.gb",
            Some(50000000),
            TestOptions::default(),
        )
        .unwrap();
        assert_eq!(result, "instr_timing\n\n\nPassed\n");
        assert_eq!(game_boy.rom_i().gb_mode(), GameBoyMode::Cgb);
        assert_eq!(game_boy.rom_i().title().as_str(), "INSTR_TIMING");
        assert_eq!(game_boy.rom_i().licensee(), Licensee::None);
        assert_eq!(game_boy.rom_i().region(), Region::Unknown);
        assert_eq!(game_boy.rom_i().rom_size(), RomSize::Size32K);
        assert_eq!(game_boy.rom_i().ram_size(), RamSize::NoRam);
        assert!(game_boy.rom_i().valid_checksum());
    }
}
