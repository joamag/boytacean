use boytacean::gb::{GameBoy, GameBoyMode};
use std::time::Instant;

const REBOOTS: u32 = 500;

fn main() {
    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
    let mut cycles = 0_u64;
    let start = Instant::now();
    println!("Running {REBOOTS} reboots...");
    for _ in 0..REBOOTS {
        game_boy.reset();
        game_boy.load(true).unwrap();
        game_boy.load_rom_empty().unwrap();
        cycles += game_boy.step_to(0x0100) as u64;
    }
    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs_f64();
    let frequency_mhz = cycles as f64 / elapsed_s / 1000.0 / 1000.0;
    println!("Ran {cycles} cycles in {elapsed:?} ({frequency_mhz:.2} Mhz)");
}
