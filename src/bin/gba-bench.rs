//! Benchmark utility for GBA emulation speed
//!
//! Runs a GBA ROM for a fixed number of frames and reports MHz,
//! speedup over realtime, and per-frame timing.
//!
//! # Usage
//! gba-bench <rom.gba> \[num_frames\] \[--warmup <frames>\] \[--runs <n>\] \[--cpu-only\]

use std::{env, time::Instant};

use boytacean::gba::GameBoyAdvance;

const DEFAULT_FRAMES: u32 = 300;
const GBA_CPU_FREQ: f64 = 16777216.0;

fn print_usage() {
    eprintln!(
        "Usage: gba-bench <rom.gba> [num_frames] [--warmup <frames>] [--runs <n>] [--cpu-only]"
    );
    eprintln!("If num_frames is not specified, defaults to {DEFAULT_FRAMES}");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let mut num_frames = DEFAULT_FRAMES;
    let mut warmup_frames = 60u32;
    let mut runs = 3u32;
    let mut cpu_only = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--warmup" => {
                warmup_frames = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(60);
                i += 2;
            }
            "--runs" => {
                runs = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
                i += 2;
            }
            "--cpu-only" => {
                cpu_only = true;
                i += 1;
            }
            other => {
                if let Ok(n) = other.parse::<u32>() {
                    num_frames = n;
                }
                i += 1;
            }
        }
    }

    let data = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&data).expect("Failed to load ROM");

    let title = info.title();
    let code = info.game_code();
    println!("ROM: {title} ({code})");
    println!(
        "Config: {num_frames} frames, {warmup_frames} warmup, {runs} runs{}",
        if cpu_only { ", CPU-only" } else { "" }
    );
    println!();

    if cpu_only {
        gba.set_ppu_enabled(false);
        gba.set_apu_enabled(false);
        gba.set_dma_enabled(false);
        gba.set_timer_enabled(false);
    }

    // warmup: run frames without measuring to get past boot/menu
    for _ in 0..warmup_frames {
        gba.next_frame();
    }

    let mut results = Vec::with_capacity(runs as usize);

    for run in 1..=runs {
        let mut total_cycles = 0u64;
        let start = Instant::now();

        for _ in 0..num_frames {
            total_cycles += gba.next_frame();
        }

        let elapsed = start.elapsed();
        let secs = elapsed.as_secs_f64();
        let mhz = total_cycles as f64 / secs / 1_000_000.0;
        let speedup = total_cycles as f64 / (GBA_CPU_FREQ * secs);
        let fps = speedup * 59.7275;
        let avg_frame_us = elapsed.as_micros() as f64 / num_frames as f64;

        println!(
            "Run {run}/{runs}: {mhz:.2} MHz ({speedup:.2}x, {fps:.1} FPS) \
             [{total_cycles} cycles in {secs:.3}s, {avg_frame_us:.0} µs/frame]"
        );

        results.push(mhz);
    }

    println!();

    let min = results.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = results.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = results.iter().sum::<f64>() / results.len() as f64;
    let speedup = avg / (GBA_CPU_FREQ / 1_000_000.0);

    println!("Summary: avg {avg:.2} MHz, min {min:.2}, max {max:.2} ({speedup:.2}x realtime)");
}
