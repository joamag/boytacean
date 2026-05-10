//! Microbenchmark: drives [`Ppu::clock`] and [`FastPpuDriver::clock`]
//! with the same cycle stream captured from a real CPU run, and
//! reports the relative cost of the per-instruction PPU dispatch.
//! The current PPU is run with no rendering (LCD off) so only the
//! state-machine overhead is measured — that's the part we'd
//! actually save by switching to a clock_target driver
//!
//! # Usage
//! cargo run --release --example bench_ppu_fast -- <rom.gb> [frames]

use std::{env::args, error::Error, time::Instant};

use boytacean::{
    gb::{GameBoy, GameBoyMode},
    ppu::Ppu,
    ppu_fast::FastPpuDriver,
};

const DEFAULT_FRAMES: usize = 6000;

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

    // capture a representative cycle stream by stepping the CPU one
    // instruction at a time for N frames, recording the cycle count
    // returned by `cpu_clock()` for each step
    let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
    gb.load(true)?;
    gb.load_rom_file(rom_path, None)?;
    gb.set_apu_enabled(false);
    for _ in 0..60 {
        gb.next_frame();
    }

    let mut cycles = Vec::with_capacity(frames * 17_500);
    let initial_frame = gb.ppu_frame() as i32 - frames as i32;
    let start_frame = gb.ppu_frame();
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
    let _ = (initial_frame, start_frame);

    let total_cycles: u64 = cycles.iter().map(|&c| c as u64).sum();
    println!();
    println!("=== Boytacean PPU driver microbenchmark ===");
    println!(" rom:    {}", rom_path);
    println!(" frames: {}", frames);
    println!(" total cycles captured:       {}", total_cycles);
    println!(" total clock() calls captured: {}", cycles.len());
    println!(
        " avg cycles per call:         {:.2}",
        total_cycles as f64 / cycles.len() as f64,
    );
    println!();

    // FastPpuDriver — pure driver, no renderer
    let mut driver = FastPpuDriver::new();
    let t0 = Instant::now();
    for &c in &cycles {
        driver.clock(c);
    }
    let fast_ns = t0.elapsed().as_nanos();
    println!(
        " FastPpuDriver:    {:>7.1} ms  ({:.2} ns/call, frame_index={})",
        fast_ns as f64 / 1e6,
        fast_ns as f64 / cycles.len() as f64,
        driver.frame_index,
    );

    // existing Ppu — full state machine with rendering enabled (LCDC bit 7
    // and BG bit 0 set), captures the realistic cost a Tetris frame pays
    let mut ppu_render = Ppu::new(GameBoyMode::Dmg, Default::default());
    ppu_render.write(0xff40, 0x91);
    let t0 = Instant::now();
    for &c in &cycles {
        ppu_render.clock(c);
    }
    let real_render_ns = t0.elapsed().as_nanos();
    println!(
        " Ppu::clock (render on):  {:>7.1} ms  ({:.2} ns/call, frame_index={})",
        real_render_ns as f64 / 1e6,
        real_render_ns as f64 / cycles.len() as f64,
        ppu_render.frame_index(),
    );

    // existing Ppu with LCD off so render_line is skipped — isolates the
    // pure state-machine dispatch cost vs FastPpuDriver
    let mut ppu_off = Ppu::new(GameBoyMode::Dmg, Default::default());
    // LCDC = 0 means LCD off; the PPU returns immediately, but the Game
    // Boy treats this as "frozen", so to get a fair driver comparison we
    // toggle LCD on for the state machine and clear BG/window/obj bits
    ppu_off.write(0xff40, 0x80); // LCD on, no BG, no window, no obj
    let t0 = Instant::now();
    for &c in &cycles {
        ppu_off.clock(c);
    }
    let real_off_ns = t0.elapsed().as_nanos();
    println!(
        " Ppu::clock (no draw):    {:>7.1} ms  ({:.2} ns/call, frame_index={})",
        real_off_ns as f64 / 1e6,
        real_off_ns as f64 / cycles.len() as f64,
        ppu_off.frame_index(),
    );

    let real_ns = real_render_ns;

    println!();
    let speedup = real_ns as f64 / fast_ns as f64;
    println!(" driver-only speedup over real Ppu: {:.2}x", speedup,);
    println!(
        " saved time per frame:              {:.1} us",
        (real_ns as i128 - fast_ns as i128) as f64 / 1000.0 / frames as f64,
    );
    println!();
    println!(" note: real Ppu also does scanline rendering when LCDC has BG on;");
    println!("       to read this as 'driver-only cost', subtract render_line work");

    Ok(())
}
