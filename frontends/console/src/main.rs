use boytacean::gb::{GameBoy, GameBoyMode};
use std::time::Instant;

const REBOOTS: u32 = 500;

fn main() {
    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
    let mut cycles = 0_u32;
    let start = Instant::now();
    println!("Running {} reboots...", REBOOTS);
    for _ in 0..REBOOTS {
        game_boy.reset();
        game_boy.load(true).unwrap();
        game_boy.load_rom_empty().unwrap();
        cycles += game_boy.step_to(0x0100);
    }
    let elapsed = start.elapsed();
    println!("Ran {} cycles in {:?}", cycles, elapsed);
}
