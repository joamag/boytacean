//! Diagnostic utility for GBA emulation
//!
//! Runs a GBA ROM for a given number of frames and dumps detailed
//! emulator state including CPU registers, PPU configuration, IRQ
//! status, timer state, and per-frame PPM snapshots.
//!
//! # Usage
//! gba-diag <rom.gba> \[num_frames\] \[--bios <bios.bin>\] \[--audio\]

use std::env;

use boytacean::gba::{
    diag::{run_audio_diagnostics, run_diagnostics},
    GameBoyAdvance,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gba-diag <rom.gba> [num_frames] [--bios <bios.bin>] [--audio]");
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let mut num_frames = 30u32;
    let mut bios_path: Option<String> = None;
    let mut audio_mode = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--bios" => {
                if i + 1 < args.len() {
                    bios_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--bios requires a path argument");
                    std::process::exit(1);
                }
            }
            "--audio" => {
                audio_mode = true;
                i += 1;
            }
            _ => {
                num_frames = args[i].parse::<u32>().unwrap_or(30);
                i += 1;
            }
        }
    }

    let data = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&data).expect("Failed to load ROM");
    println!("Loaded: {} ({})", info.title(), info.game_code());

    if let Some(path) = &bios_path {
        let bios_data = std::fs::read(path).expect("Failed to read BIOS file");
        gba.load_bios(&bios_data);
        println!("BIOS loaded from {path}");
    }

    if audio_mode {
        run_audio_diagnostics(&mut gba, num_frames);
    } else {
        run_diagnostics(&mut gba, num_frames);
    }
}
