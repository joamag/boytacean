use std::{env::current_dir, error::Error, time::Instant};

use boytacean::{
    color::RGB_SIZE,
    gb::{GameBoy, GameBoyMode},
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE},
};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short = 'p', long)]
    rom_path: String,

    #[clap(short, long, default_value_t = 10000000)]
    cycles: u64,

    #[clap(short, long, default_value_t = 1)]
    scale: u32,

    #[clap(short, long, default_value_t = 0)]
    reboots: u32,

    #[clap(short, long, default_value_t = true)]
    frame_buffer: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let parsed_args = Args::parse();
    let rom_path = current_dir().unwrap().join(&parsed_args.rom_path);
    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));

    let start = Instant::now();
    let mut cycles = 0;

    if parsed_args.reboots > 0 {
        println!("Running {} reboots...", parsed_args.reboots);
        for _ in 0..parsed_args.reboots {
            game_boy.reset();
            game_boy.load(true).unwrap();
            game_boy.load_rom_empty().unwrap();
            cycles += game_boy.step_to(0x0100) as u64;
        }
    } else {
        game_boy.load(true).unwrap();
        game_boy
            .load_rom_file(rom_path.to_str().unwrap(), None)
            .unwrap();
        game_boy.attach_stdout_serial();

        println!("Running ROM: {}", rom_path.display());
        println!("Running for {} cycles...", parsed_args.cycles);

        while cycles < parsed_args.cycles {
            cycles += game_boy.clock() as u64;
        }
    }

    let elapsed = start.elapsed();
    let elapsed_s = elapsed.as_secs_f64();
    let frequency_mhz = cycles as f64 / elapsed_s / 1000.0 / 1000.0;
    println!(
        "Ran {} cycles in {:?} ({:.2} Mhz)",
        cycles, elapsed, frequency_mhz
    );

    if parsed_args.frame_buffer {
        print_framebuffer(&game_boy.frame_buffer_raw(), parsed_args.scale);
    }

    Ok(())
}

/// Prints the framebuffer contents as ASCII characters to the console.
///
/// This function converts the raw RGB framebuffer data into a human-readable
/// ASCII representation. Each pixel is converted to a character based on its
/// luminance value using the standard luminance formula:
/// `0.299 * R + 0.587 * G + 0.114 * B`
///
/// The ASCII characters used for different luminance levels are:
/// - `#` for very dark pixels (luminance 0-50)
/// - `@` for dark pixels (luminance 51-100)
/// - `%` for medium pixels (luminance 101-150)
/// - `.` for light pixels (luminance 151-200)
/// - ` ` (space) for very light pixels (luminance 201-255)
///
/// The output is formatted as a grid with dimensions `DISPLAY_WIDTH` x `DISPLAY_HEIGHT`,
/// where each character represents one pixel from the Game Boy's display.
fn print_framebuffer(frame_buffer: &[u8; FRAME_BUFFER_SIZE], scale: u32) {
    let scale = scale.max(1);
    let scaled_width = DISPLAY_WIDTH / scale as usize;
    let scaled_height = DISPLAY_HEIGHT / scale as usize;

    println!("Framebuffer output (ASCII) - Scale 1:{}", scale);
    for y in 0..scaled_height {
        for x in 0..scaled_width {
            // calculates the average color for the scaled pixel
            let mut total_r = 0u32;
            let mut total_g = 0u32;
            let mut total_b = 0u32;
            let mut pixel_count = 0u32;

            // samples pixels in the scale x scale block
            for dy in 0..scale {
                for dx in 0..scale {
                    let src_x = x * scale as usize + dx as usize;
                    let src_y = y * scale as usize + dy as usize;

                    if src_x < DISPLAY_WIDTH && src_y < DISPLAY_HEIGHT {
                        let base_idx = (src_y * DISPLAY_WIDTH + src_x) * RGB_SIZE;
                        total_r += frame_buffer[base_idx] as u32;
                        total_g += frame_buffer[base_idx + 1] as u32;
                        total_b += frame_buffer[base_idx + 2] as u32;
                        pixel_count += 1;
                    }
                }
            }

            // calculates average color
            let r = (total_r / pixel_count) as u8;
            let g = (total_g / pixel_count) as u8;
            let b = (total_b / pixel_count) as u8;

            // implements a simple ASCII representation based on luminance
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
