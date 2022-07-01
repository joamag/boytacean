use boytacean::gb::GameBoy;

fn main() {
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_default();

    for i in 0..37000 {
        // runs the Game Boy clock, this operations should
        // include the advance of both the CPU and the PPU
        game_boy.clock();

        if game_boy.cpu().pc() >= 0x6032 {
            println!("{}", i);
            break;
        }
    }
}
