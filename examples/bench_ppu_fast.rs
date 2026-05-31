//! Microbenchmark comparing [`Ppu::clock`] and [`FastPpu::clock`] dispatch cost.
//!
//! Replays the same cycle stream captured from a real CPU run
//! through both drivers and reports the relative cost of the
//! per-instruction PPU dispatch. The current PPU is also run with
//! no rendering (LCD off) so only the state-machine overhead is
//! measured — that's the part we'd actually save by switching to
//! a clock_target driver.
//!
//! # Usage
//! cargo run --release --example bench_ppu_fast -- <rom.gb> [frames]

use std::{env::args, error::Error, time::Instant};

use boytacean::{
    consts::LCDC_ADDR,
    gb::{GameBoy, GameBoyMode},
    ppu::Ppu,
    ppu_fast::FastPpu,
};

/// Default number of frames to capture when no second argument
/// is provided on the command line.
const DEFAULT_FRAMES: usize = 6000;

/// Number of frames to discard after `load()` before starting the
/// capture, so the boot ROM hand-off doesn't skew the sample.
const WARMUP_FRAMES: usize = 60;

/// LCDC value with the LCD on plus background bit, matching the
/// realistic cost a Tetris-like frame pays.
const LCDC_RENDER_ON: u8 = 0x91;

/// LCDC value with the LCD on and BG/window/obj all off, so the
/// state machine runs but `render_line` short-circuits — isolates
/// the pure dispatch cost.
const LCDC_NO_DRAW: u8 = 0x80;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        println!("Usage: bench_ppu_fast <rom.gb> [frames]");
        return Ok(());
    }
    let rom_path = &args[1];
    let frames = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_FRAMES);

    let cycles = capture_cycles(rom_path, frames)?;

    let total_cycles: u64 = cycles.iter().map(|&c| c as u64).sum();
    println!();
    println!("=== Boytacean PPU driver microbenchmark ===");
    println!(" rom:    {}", rom_path);
    println!(" frames: {}", frames);
    println!(" total cycles captured:        {}", total_cycles);
    println!(" total clock() calls captured: {}", cycles.len());
    println!(
        " avg cycles per call:          {:.2}",
        total_cycles as f64 / cycles.len() as f64,
    );
    println!();

    // FastPpu, prototype clock_target driver from ppu_fast.rs; pure
    // mode/LY state machine, serves as the dispatch-cost baseline
    let mut fast_ppu = FastPpu::new();
    let fast_ns = bench(&cycles, |c| {
        fast_ppu.clock(c);
    });
    print_result(
        "FastPpu",
        fast_ns,
        cycles.len(),
        fast_ppu.frame_index as u16,
    );

    // existing Ppu, full state machine with rendering enabled (LCD on
    // plus BG bit), captures the realistic cost a Tetris-like frame pays
    let mut ppu_render = Ppu::new(GameBoyMode::Dmg, Default::default());
    ppu_render.write(LCDC_ADDR, LCDC_RENDER_ON);
    let render_ns = bench(&cycles, |c| {
        ppu_render.clock(c);
    });
    print_result(
        "Ppu::clock (render on)",
        render_ns,
        cycles.len(),
        ppu_render.frame_index(),
    );

    // existing Ppu with LCD on but BG/window/obj cleared, so the state
    // machine runs but `render_line` short-circuits - isolates the
    // pure dispatch cost vs FastPpu
    let mut ppu_off = Ppu::new(GameBoyMode::Dmg, Default::default());
    ppu_off.write(LCDC_ADDR, LCDC_NO_DRAW);
    let no_draw_ns = bench(&cycles, |c| {
        ppu_off.clock(c);
    });
    print_result(
        "Ppu::clock (no draw)",
        no_draw_ns,
        cycles.len(),
        ppu_off.frame_index(),
    );

    println!();
    let speedup = render_ns as f64 / fast_ns as f64;
    println!(" driver-only speedup over real Ppu: {:.2}x", speedup);
    println!(
        " saved time per frame:              {:.1} us",
        (render_ns as i128 - fast_ns as i128) as f64 / 1000.0 / frames as f64,
    );
    println!();
    println!(" note: real PPU also does scanline rendering when LCDC has BG on;");
    println!("       to read this as 'driver-only cost', subtract render_line work");

    Ok(())
}

/// Captures a representative per-instruction cycle stream by booting
/// the ROM, warming up the boot sequence and stepping the CPU one
/// instruction at a time until `frames` frames have completed.
fn capture_cycles(rom_path: &str, frames: usize) -> Result<Vec<u16>, Box<dyn Error>> {
    let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
    gb.load(true)?;
    gb.load_rom_file(rom_path, None)?;
    gb.set_apu_enabled(false);
    for _ in 0..WARMUP_FRAMES {
        gb.next_frame();
    }

    let mut cycles = Vec::with_capacity(frames * 17_500);
    let target_frames = frames as u32;
    let mut completed: u32 = 0;
    while completed < target_frames {
        let prev_frame = gb.ppu_frame();
        let c = gb.cpu_clock() as u16;
        cycles.push(c);
        let cycles_n = c / gb.multiplier() as u16;
        gb.ppu_clock(cycles_n);
        gb.dma_clock(c);
        gb.timer_clock(c);
        gb.serial_clock(c);
        if gb.ppu_frame() != prev_frame {
            completed += 1;
        }
    }
    Ok(cycles)
}

/// Replays `cycles` through the supplied driver and returns the
/// elapsed wall-clock time in nanoseconds.
fn bench(cycles: &[u16], mut driver: impl FnMut(u16)) -> u128 {
    let t0 = Instant::now();
    for &c in cycles {
        driver(c);
    }
    t0.elapsed().as_nanos()
}

/// Prints a single bench row with consistent column alignment.
fn print_result(label: &str, ns: u128, calls: usize, frame_index: u16) {
    println!(
        " {:<24} {:>7.1} ms  ({:.2} ns/call, frame_index={})",
        format!("{label}:"),
        ns as f64 / 1e6,
        ns as f64 / calls as f64,
        frame_index,
    );
}
