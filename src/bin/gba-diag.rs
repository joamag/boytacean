use std::env;

use boytacean::gba::{diag::run_diagnostics, GameBoyAdvance};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gba-diag <rom.gba> [num_frames]");
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let num_frames = if args.len() > 2 {
        args[2].parse::<u32>().unwrap_or(30)
    } else {
        30
    };

    let data = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&data).expect("Failed to load ROM");
    println!("Loaded: {} ({})", info.title(), info.game_code());

    run_diagnostics(&mut gba, num_frames);
}
