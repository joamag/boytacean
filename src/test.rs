use crate::{devices::buffer::BufferDevice, gb::GameBoy};

#[derive(Default)]
pub struct TestOptions {
    ppu_enabled: Option<bool>,
    apu_enabled: Option<bool>,
    dma_enabled: Option<bool>,
    timer_enabled: Option<bool>,
}

pub fn build_test(options: TestOptions) -> GameBoy {
    let device = Box::<BufferDevice>::default();
    let mut game_boy = GameBoy::new(None);
    game_boy.set_ppu_enabled(options.ppu_enabled.unwrap_or(true));
    game_boy.set_apu_enabled(options.apu_enabled.unwrap_or(true));
    game_boy.set_dma_enabled(options.dma_enabled.unwrap_or(true));
    game_boy.set_timer_enabled(options.timer_enabled.unwrap_or(true));
    game_boy.attach_serial(device);
    game_boy.load(true);
    game_boy
}

pub fn run_test(rom_path: &str, max_cycles: Option<u64>, options: TestOptions) -> String {
    let mut cycles = 0u64;
    let max_cycles = max_cycles.unwrap_or(u64::MAX);

    let mut game_boy = build_test(options);
    game_boy.load_rom_file(rom_path);

    loop {
        cycles += game_boy.clock() as u64;
        if cycles >= max_cycles {
            break;
        }
    }

    game_boy.serial().device().state()
}

#[cfg(test)]
mod tests {
    use super::{run_test, TestOptions};

    #[test]
    fn test_blargg_cpu_instrs() {
        let result = run_test(
            "res/roms/test/blargg/cpu/cpu_instrs.gb",
            Some(300000000),
            TestOptions::default(),
        );
        assert_eq!(result, "cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests\n");
    }
}
