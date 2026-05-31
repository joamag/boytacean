//! Minimal smoke test that boots a DMG and runs to PC=0x0100.
//!
//! Creates a fresh [`GameBoy`] in DMG mode, loads the bundled boot
//! ROM and an empty cartridge, then single-steps the CPU until it
//! reaches the cartridge entry point (`0x0100`) - the address the
//! boot ROM hands control off to a real game. Prints the number of
//! cycles the boot sequence took, which on the stock DMG boot ROM
//! should be a stable value run-to-run and is a quick way to spot
//! regressions in CPU timing or boot-ROM dispatch.
//!
//! # Usage
//! cargo run --example simple

use std::error::Error;

use boytacean::gb::{GameBoy, GameBoyMode};

fn main() -> Result<(), Box<dyn Error>> {
    let mut game_boy = GameBoy::new(Some(GameBoyMode::Dmg));
    game_boy.load(true)?;
    game_boy.load_rom_empty()?;
    let cycles = game_boy.step_to(0x0100);
    println!("Ran {cycles} cycles");
    Ok(())
}
