# Boytacean - Game Boy Emulator

## Project Overview

Boytacean is a Game Boy and Game Boy Color emulator written in Rust. It supports multiple frontends including SDL, web (WebAssembly), console, and libretro. The project also provides Python bindings.

## Architecture

- **Core Library**: `src/` - Main emulation logic (CPU, PPU, APU, MMU, etc.)
- **Frontends**: `frontends/` - Different UI implementations
- **Crates**: `crates/` - Utility crates (common, encoding, hashing)
- **Python Bindings**: `src/python/` and `src/py.rs`

## Key Components

- `src/gb.rs` - Main Game Boy system implementation
- `src/cpu.rs` - CPU emulation (Sharp LR35902)
- `src/ppu.rs` - Picture Processing Unit (graphics)
- `src/apu.rs` - Audio Processing Unit (sound)
- `src/mmu.rs` - Memory Management Unit
- `src/rom.rs` - ROM handling and cartridge types
- `src/boot/` - Boot ROM implementation

## Build Commands

- **Rust**: `cargo build --release`
- **Python**: `pip install -e .` or `python setup.py develop`
- **Web**: `cd frontends/web && yarn install && yarn build`
- **SDL**: `cargo build --release --bin boytacean-sdl`

## Test Commands

- **Rust**: `cargo test`
- **Python**: `pytest`
- **Benchmarks**: `cargo bench`

## Development Notes

- Uses `wasm-pack` for WebAssembly builds
- Supports multiple Game Boy variants (DMG, CGB, SGB)
- Includes extensive test ROMs in `res/roms/test/`
- Boot ROMs available in `res/boot/`
- Custom shader support in SDL frontend

## Code Style

- Follows Rust conventions with `rustfmt.toml`
- Python code follows PEP 8
- TypeScript/JavaScript uses standard formatting

## Testing

- Blargg test ROMs for CPU and sound accuracy
- Acid2 tests for PPU accuracy  
- Custom test suite in `src/test.rs`
- Python tests in `src/python/boytacean/test/`
