use boytacean::gb::GameBoy;

fn main() {
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_default();

    for i in 0..37000 {
        // runs the CPU clock and determines the number of
        // cycles that have advanced for that clock tick
        let cycles = game_boy.clock();

        // calls the clock in the PPU to update its own
        // execution lifecycle by one set of ticks
        game_boy.ppu_clock(cycles);

        if game_boy.cpu().pc() >= 0x6032 {
            println!("{}", i);
            break;
        }
    }
}
