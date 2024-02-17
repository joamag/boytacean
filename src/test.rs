use crate::{
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
}

pub fn build_test(options: TestOptions) -> GameBoy {
    let device = Box::<BufferDevice>::default();
    let mut game_boy = GameBoy::new(options.mode);
    game_boy.set_ppu_enabled(options.ppu_enabled.unwrap_or(true));
    game_boy.set_apu_enabled(options.apu_enabled.unwrap_or(true));
    game_boy.set_dma_enabled(options.dma_enabled.unwrap_or(true));
    game_boy.set_timer_enabled(options.timer_enabled.unwrap_or(true));
    game_boy.attach_serial(device);
    game_boy.load(true);
    game_boy
}

pub fn run_test(rom_path: &str, max_cycles: Option<u64>, options: TestOptions) -> GameBoy {
    let mut cycles = 0u64;
    let max_cycles = max_cycles.unwrap_or(u64::MAX);

    let mut game_boy = build_test(options);
    game_boy.load_rom_file(rom_path, None);

    loop {
        cycles += game_boy.clock() as u64;
        if cycles >= max_cycles {
            break;
        }
    }

    game_boy
}

pub fn run_step_test(rom_path: &str, addr: u16, options: TestOptions) -> GameBoy {
    let mut game_boy = build_test(options);
    game_boy.load_rom_file(rom_path, None);
    game_boy.step_to(addr);
    game_boy
}

pub fn run_serial_test(rom_path: &str, max_cycles: Option<u64>, options: TestOptions) -> String {
    let mut game_boy = run_test(rom_path, max_cycles, options);
    game_boy.serial().device().state()
}

pub fn run_image_test(
    rom_path: &str,
    max_cycles: Option<u64>,
    options: TestOptions,
) -> [u8; FRAME_BUFFER_SIZE] {
    let mut game_boy = run_test(rom_path, max_cycles, options);
    *game_boy.frame_buffer()
}

#[cfg(test)]
mod tests {
    use super::{run_serial_test, run_step_test, TestOptions};

    #[test]
    fn test_boot_state() {
        let result = run_step_test(
            "res/roms/test/blargg/cpu/cpu_instrs.gb",
            0x0100,
            TestOptions::default(),
        );
        assert_eq!(result.cpu_i().pc(), 0x0100);
    }

    #[test]
    fn test_blargg_cpu_instrs() {
        let result = run_serial_test(
            "res/roms/test/blargg/cpu/cpu_instrs.gb",
            Some(300000000),
            TestOptions::default(),
        );
        assert_eq!(result, "cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests\n");
    }

    #[test]
    fn test_blargg_instr_timing() {
        let result = run_serial_test(
            "res/roms/test/blargg/instr_timing/instr_timing.gb",
            Some(50000000),
            TestOptions::default(),
        );
        assert_eq!(result, "instr_timing\n\n\nPassed\n");
    }
}
