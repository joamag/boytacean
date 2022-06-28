use boytacean::gb::GameBoy;

fn main() {
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_default();

    for i in 0..24612 {
        game_boy.clock();
        if game_boy.cpu().pc() >= 0x3032 {
            println!("{}", i);
            break;
        }
    }
}
