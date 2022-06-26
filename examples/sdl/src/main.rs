use boytacean::gb::GameBoy;

fn main() {
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_default();

    game_boy.clock();
    game_boy.clock();
    game_boy.clock();
    game_boy.clock();
    game_boy.clock();
}
