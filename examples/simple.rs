use boytacean::gb::{GameBoy, GameBoyMode};

fn main() {
    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
    game_boy.load(false).unwrap();
    game_boy.load_rom_empty().unwrap();
    let cycles = game_boy.step_to(0x0100);
    println!("Ran {} cycles", cycles);
}
