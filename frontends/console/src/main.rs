use boytacean::{
    gb::{GameBoy, GameBoyMode},
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE},
};
use clap::Parser;
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    rom_path: String,

    #[clap(short, long, default_value_t = 10000000)]
    cycles: u64,
}

fn main() {
    let parsed_args = Args::parse();

    // Construct an absolute path to the ROM file.
    // Assumes that when run with `cargo run --package boytacean-console`,
    // the current directory is `<workspace_root>/frontends/console`.
    // The ROM path is relative to the workspace root.
    let mut base_path = std::env::current_dir().unwrap(); // Should be /app/frontends/console
    base_path.pop(); // Now /app/frontends
    base_path.pop(); // Now /app (workspace root)
    let rom_path = base_path.join(&parsed_args.rom_path);

    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
    game_boy.load(true).unwrap();
    game_boy
        .load_rom_file(rom_path.to_str().unwrap(), None)
        .unwrap();
    game_boy.attach_stdout_serial();

    println!("Running ROM: {}", rom_path.display());
    println!("Running for {} cycles...", parsed_args.cycles);

    let start = Instant::now();
    let mut current_cycles = 0;
    while current_cycles < parsed_args.cycles {
        current_cycles += game_boy.clock() as u64;
    }
    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs_f64();
    let frequency_mhz = current_cycles as f64 / elapsed_s / 1000.0 / 1000.0;

    println!(
        "Ran {} cycles in {:?} ({:.2} Mhz)",
        current_cycles, elapsed, frequency_mhz
    );

    print_framebuffer(&game_boy.frame_buffer_raw());
}

fn print_framebuffer(frame_buffer: &[u8; FRAME_BUFFER_SIZE]) {
    println!("Framebuffer output (ASCII):");
    for y in 0..DISPLAY_HEIGHT {
        for x in 0..DISPLAY_WIDTH {
            let base_idx = (y * DISPLAY_WIDTH + x) * 3; // 3 bytes per pixel (RGB)
            let r = frame_buffer[base_idx];
            let g = frame_buffer[base_idx + 1];
            let b = frame_buffer[base_idx + 2];

            // Simple ASCII representation based on luminance
            let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
            let char_to_print = match luminance {
                0..=50 => '#',
                51..=100 => '@',
                101..=150 => '%',
                151..=200 => '.',
                _ => ' ',
            };
            print!("{}", char_to_print);
        }
        println!();
    }
}
