//! Benchmark utility for GBA emulation speed
//!
//! Runs a GBA ROM for a fixed number of frames and reports MHz,
//! speedup over realtime, and per-frame timing.
//!
//! # Usage
//! gba-bench <rom.gba> \[num_frames\] \[--warmup <frames>\] \[--runs <n>\] \[--cpu-only\]

use std::{env::args, error::Error, time::Instant};

use boytacean::gba::{
    consts::{CPU_FREQ, CYCLES_PER_FRAME, VISUAL_FREQ},
    GameBoyAdvance,
};
use boytacean_common::util::read_file;

const DEFAULT_FRAMES: u32 = 300;
const DEFAULT_WARMUP: u32 = 60;
const DEFAULT_RUNS: u32 = 3;

/// The options of a run of the utility, as provided in its arguments.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Options {
    rom: String,
    frames: u32,
    warmup: u32,
    runs: u32,
    cpu_only: bool,
}

fn print_usage() {
    println!("Usage: gba-bench <rom.gba> [num_frames] [options]");
    println!("If num_frames is not specified, each run measures {DEFAULT_FRAMES} frames");
    println!("Options:");
    println!("  --warmup <frames>   Frames run before measuring ({DEFAULT_WARMUP})");
    println!("  --runs <n>          Number of measured runs ({DEFAULT_RUNS})");
    println!(
        "  --cpu-only          Clocks the CPU alone, with the PPU, APU, DMA and timers disabled"
    );
}

/// Parses the arguments of the utility, `None` when they are not valid.
fn parse_args(args: &[String]) -> Option<Options> {
    let mut options = Options {
        rom: args.first().filter(|arg| !arg.starts_with("--"))?.clone(),
        frames: DEFAULT_FRAMES,
        warmup: DEFAULT_WARMUP,
        runs: DEFAULT_RUNS,
        cpu_only: false,
    };
    let mut frames = None;
    let mut args = args[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmup" => options.warmup = args.next()?.parse().ok()?,
            "--runs" => options.runs = parse_count(args.next()?)?,
            "--cpu-only" => options.cpu_only = true,
            arg if arg.starts_with("--") || frames.is_some() => return None,
            arg => frames = Some(parse_count(arg)?),
        }
    }
    options.frames = frames.unwrap_or(DEFAULT_FRAMES);
    Some(options)
}

/// Parses a count that must be positive, as no speed can be measured
/// from zero frames or zero runs.
fn parse_count(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().filter(|&count| count > 0)
}

fn clock_frame(gba: &mut GameBoyAdvance, cpu_only: bool) -> u64 {
    if cpu_only {
        gba.clocks_cycles(CYCLES_PER_FRAME as usize)
    } else {
        gba.next_frame()
    }
}

/// Runs the benchmark of the options, printing the speed of each of its
/// runs and their summary, and returning the speed of each run in MHz.
fn bench(options: &Options) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&read_file(&options.rom)?)?;

    let title = info.title();
    let code = info.game_code();
    println!("ROM: {title} ({code})");
    println!(
        "Config: {} frames, {} warmup, {} runs{}",
        options.frames,
        options.warmup,
        options.runs,
        if options.cpu_only { ", CPU-only" } else { "" }
    );
    println!();

    if options.cpu_only {
        gba.set_all_enabled(false);
    }

    // warmup: run frames without measuring to get past boot/menu
    for _ in 0..options.warmup {
        clock_frame(&mut gba, options.cpu_only);
    }

    let mut results = Vec::with_capacity(options.runs as usize);

    for run in 1..=options.runs {
        let mut total_cycles = 0u64;
        let start = Instant::now();

        for _ in 0..options.frames {
            total_cycles += clock_frame(&mut gba, options.cpu_only);
        }

        let elapsed = start.elapsed();
        let secs = elapsed.as_secs_f64();
        let mhz = total_cycles as f64 / secs / 1_000_000.0;
        let speedup = total_cycles as f64 / (CPU_FREQ as f64 * secs);
        let fps = speedup * VISUAL_FREQ as f64;
        let avg_frame_us = elapsed.as_micros() as f64 / options.frames as f64;

        println!(
            "Run {run}/{}: {mhz:.2} MHz ({speedup:.2}x, {fps:.1} FPS) \
             [{total_cycles} cycles in {secs:.3}s, {avg_frame_us:.0} µs/frame]",
            options.runs
        );

        results.push(mhz);
    }

    println!();

    let min = results.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = results.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = results.iter().sum::<f64>() / results.len() as f64;
    let speedup = avg / (CPU_FREQ as f64 / 1_000_000.0);

    println!("Summary: avg {avg:.2} MHz, min {min:.2}, max {max:.2} ({speedup:.2}x realtime)");

    Ok(results)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        print_usage();
        return Ok(());
    };
    bench(&options)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use boytacean::gba::GameBoyAdvance;

    use super::{
        bench, clock_frame, parse_args, parse_count, Options, CYCLES_PER_FRAME, DEFAULT_FRAMES,
        DEFAULT_RUNS, DEFAULT_WARMUP,
    };

    /// The test ROM run by the tests, relative to the package root.
    const ROM_PATH: &str = "res/roms.gba/test/jsmolka_gba-tests/arm.gba";

    fn build_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| String::from(*arg)).collect()
    }

    fn build_options(runs: u32, cpu_only: bool) -> Options {
        Options {
            rom: String::from(ROM_PATH),
            frames: 2,
            warmup: 1,
            runs,
            cpu_only,
        }
    }

    #[test]
    fn test_parse_args() {
        assert_eq!(
            parse_args(&build_args(&["game.gba"])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: DEFAULT_FRAMES,
                warmup: DEFAULT_WARMUP,
                runs: DEFAULT_RUNS,
                cpu_only: false,
            })
        );
        assert_eq!(
            parse_args(&build_args(&[
                "game.gba",
                "6000",
                "--warmup",
                "0",
                "--runs",
                "1",
                "--cpu-only"
            ])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: 6000,
                warmup: 0,
                runs: 1,
                cpu_only: true,
            })
        );
    }

    #[test]
    fn test_parse_args_invalid() {
        for args in [
            vec![],
            vec!["--cpu-only"],
            vec!["game.gba", "0"],
            vec!["game.gba", "x"],
            vec!["game.gba", "100", "200"],
            vec!["game.gba", "--runs", "0"],
            vec!["game.gba", "--runs"],
            vec!["game.gba", "--warmup", "-1"],
            vec!["game.gba", "--warmup"],
            vec!["game.gba", "--frames", "100"],
        ] {
            assert_eq!(parse_args(&build_args(&args)), None);
        }
    }

    #[test]
    fn test_parse_count() {
        assert_eq!(parse_count("1"), Some(1));
        assert_eq!(parse_count("4294967295"), Some(u32::MAX));
        assert_eq!(parse_count("0"), None);
        assert_eq!(parse_count("4294967296"), None);
        assert_eq!(parse_count(""), None);
    }

    #[test]
    fn test_clock_frame() {
        let mut gba = GameBoyAdvance::new();
        assert!(clock_frame(&mut gba, false) > 0);
        assert_eq!(gba.ppu_frame(), 1);
    }

    #[test]
    fn test_clock_frame_cpu_only() {
        let mut gba = GameBoyAdvance::new();
        gba.set_all_enabled(false);
        let pc = gba.cpu.pc();
        assert!(clock_frame(&mut gba, true) >= CYCLES_PER_FRAME as u64);
        assert_ne!(gba.cpu.pc(), pc);
        assert_eq!(gba.ppu_frame(), 0);
        assert!(gba.audio_buffer().is_empty());
    }

    #[test]
    fn test_bench() {
        for cpu_only in [false, true] {
            let results = bench(&build_options(2, cpu_only)).unwrap();
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|mhz| mhz.is_finite() && *mhz > 0.0));
        }
    }

    #[test]
    fn test_bench_missing_rom() {
        let mut options = build_options(1, false);
        options.rom = String::from("/dev/boytacean-none.gba");
        assert_eq!(
            bench(&options).unwrap_err().to_string(),
            "Failed to open file: /dev/boytacean-none.gba"
        );
    }
}
