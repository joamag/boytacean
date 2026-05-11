//! Measures the maximum native fps Boytacean achieves in pure Rust.
//!
//! No Python boundary, no per-component instrumentation and no
//! frame-buffer extraction. APU is disabled (matching the typical
//! AI training configuration). Reports best-of-three runs.
//!
//! # Usage
//! cargo run --release --example bench_headless -- <rom.gb> [frames]

use std::{env::args, error::Error, time::Instant};

use boytacean::gb::{GameBoy, GameBoyMode};

const DEFAULT_FRAMES: usize = 30000;
const VISUAL_FREQ: f64 = 59.7275;
const WARMUP: usize = 600;
const RUNS: usize = 3;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        println!("Usage: bench_headless <rom.gb> [frames]");
        return Ok(());
    }
    let rom_path = &args[1];
    let frames = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_FRAMES);

    let mut best_ns: u128 = u128::MAX;
    for run in 1..=RUNS {
        let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
        gb.load(true)?;
        gb.load_rom_file(rom_path, None)?;
        gb.set_apu_enabled(false);

        for _ in 0..WARMUP {
            gb.next_frame();
        }

        let t0 = Instant::now();
        for _ in 0..frames {
            gb.next_frame();
        }
        let elapsed = t0.elapsed().as_nanos();
        let fps = frames as f64 / (elapsed as f64 / 1e9);
        println!(
            " run {}/{}: {:>7.0} fps ({:.1}x realtime, {:.1} us/frame)",
            run,
            RUNS,
            fps,
            fps / VISUAL_FREQ,
            elapsed as f64 / 1000.0 / frames as f64,
        );
        best_ns = best_ns.min(elapsed);
    }

    let best_fps = frames as f64 / (best_ns as f64 / 1e9);
    println!();
    println!(
        " best:    {:>7.0} fps ({:.1}x realtime, {:.1} us/frame)",
        best_fps,
        best_fps / VISUAL_FREQ,
        best_ns as f64 / 1000.0 / frames as f64,
    );

    Ok(())
}
