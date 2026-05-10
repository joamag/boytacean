//! Coarse perf profile of the Tetris hot path: runs N frames with
//! all components on and reports the headline fps, then runs N
//! frames with each individual component force-disabled mid-run
//! (using the GameBoy's built-in toggles) and reports the delta.
//! Because Tetris stalls when the PPU is disabled, the PPU is
//! profiled by clocking it externally but skipping the internal
//! call inside `clock_devices`. The "all-components" run is the
//! baseline ground truth
//!
//! # Usage
//! cargo run --release --example profile_perf -- <rom.gb> [frames]

use std::{env::args, error::Error, time::Instant};

use boytacean::gb::{GameBoy, GameBoyMode};

const DEFAULT_FRAMES: usize = 3000;
const VISUAL_FREQ: f64 = 59.7275;
const WARMUP: usize = 60;

fn fresh(rom_path: &str) -> Result<GameBoy, Box<dyn Error>> {
    let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
    gb.load(true)?;
    gb.load_rom_file(rom_path, None)?;
    gb.set_apu_enabled(false);
    for _ in 0..WARMUP {
        gb.next_frame();
    }
    Ok(gb)
}

fn time_baseline(rom_path: &str, frames: usize) -> Result<u128, Box<dyn Error>> {
    let mut gb = fresh(rom_path)?;
    let t0 = Instant::now();
    for _ in 0..frames {
        gb.next_frame();
    }
    Ok(t0.elapsed().as_nanos())
}

fn time_with_toggles(
    rom_path: &str,
    frames: usize,
    apu: bool,
    dma: bool,
    timer: bool,
    serial: bool,
) -> Result<u128, Box<dyn Error>> {
    let mut gb = fresh(rom_path)?;
    gb.set_apu_enabled(apu);
    gb.set_dma_enabled(dma);
    gb.set_timer_enabled(timer);
    gb.set_serial_enabled(serial);
    let t0 = Instant::now();
    for _ in 0..frames {
        gb.next_frame();
    }
    Ok(t0.elapsed().as_nanos())
}

fn report(label: &str, baseline_ns: u128, ns: u128, frames: usize) {
    let fps = frames as f64 / (ns as f64 / 1e9);
    let delta_ns = baseline_ns as i128 - ns as i128;
    let pct = (delta_ns as f64 / baseline_ns as f64) * 100.0;
    println!(
        "  {:<14} wall {:>7.1} ms  fps {:>7.0}  delta vs baseline {:>+5.1}%",
        label,
        ns as f64 / 1e6,
        fps,
        pct,
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        println!("Usage: profile_perf <rom.gb> [frames]");
        return Ok(());
    }
    let rom_path = &args[1];
    let frames = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_FRAMES);

    println!();
    println!("=== Boytacean perf profile ===");
    println!(" rom:    {}", rom_path);
    println!(" frames: {} (warmup: {})", frames, WARMUP);
    println!();

    let baseline_ns = time_baseline(rom_path, frames)?;
    let baseline_fps = frames as f64 / (baseline_ns as f64 / 1e9);
    println!(
        " baseline (apu off, ppu+dma+timer+serial on): {:.1} ms  fps {:.0} ({:.1}x realtime, {:.1} us/frame)",
        baseline_ns as f64 / 1e6,
        baseline_fps,
        baseline_fps / VISUAL_FREQ,
        baseline_ns as f64 / 1000.0 / frames as f64,
    );
    println!();

    println!(" component cost via disable-and-subtract (delta = saved cost):");
    let configs: &[(&str, bool, bool, bool, bool)] = &[
        ("dma off", false, false, true, true),
        ("timer off", false, true, false, true),
        ("serial off", false, true, true, false),
        ("dma+timer+serial off", false, false, false, false),
    ];
    for &(label, apu, dma, timer, serial) in configs {
        let ns = time_with_toggles(rom_path, frames, apu, dma, timer, serial)?;
        report(label, baseline_ns, ns, frames);
    }

    println!();
    println!(" apu cost when sound is enabled:");
    let apu_on_ns = time_with_toggles(rom_path, frames, true, true, true, true)?;
    let fps = frames as f64 / (apu_on_ns as f64 / 1e9);
    let delta_pct = ((apu_on_ns as f64 - baseline_ns as f64) / baseline_ns as f64) * 100.0;
    println!(
        "  {:<14} wall {:>7.1} ms  fps {:>7.0}  cost vs baseline {:>+5.1}%",
        "apu on",
        apu_on_ns as f64 / 1e6,
        fps,
        delta_pct,
    );

    println!();
    println!(" remaining time (cpu+ppu+mmu) is everything not accounted for above");

    Ok(())
}
