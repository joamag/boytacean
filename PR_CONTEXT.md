# PR Context: feat/gba-emulation

## Summary

This PR adds full Game Boy Advance (GBA) emulation to Boytacean, which previously only supported Game Boy (DMG) and Game Boy Color (CGB). The GBA runs on the ARM7TDMI CPU, a fundamentally different architecture from the Sharp LR35902 used in DMG/CGB.

## What Changed

### New GBA Core (`src/gba/`)

All new files — ~12,000 lines of Rust implementing the GBA hardware:

| Module | File | Description |
|--------|------|-------------|
| CPU | `cpu.rs` | ARM7TDMI core with mode switching, register banking, pipeline |
| ARM | `arm.rs` | ARM (32-bit) instruction decoder and executor |
| THUMB | `thumb.rs` | THUMB (16-bit) instruction decoder and executor |
| Bus | `bus.rs` | Memory bus, IO registers, WAITCNT wait states |
| PPU | `ppu.rs` | Scanline renderer, modes 0-5, text/affine BGs, sprites, blending |
| APU | `apu.rs` | 4 legacy channels + 2 DirectSound PCM FIFO channels |
| DMA | `dma.rs` | 4-channel DMA controller with priority and timing modes |
| Timer | `timer.rs` | 4 timers with prescaler and cascade |
| IRQ | `irq.rs` | Interrupt controller (IE/IF/IME) |
| BIOS | `bios.rs` | HLE for SWI calls (Div, Sqrt, CpuSet, LZ77, etc.) |
| Flash | `flash.rs` | Flash save media with SRAM/Flash auto-detection |
| ROM | `rom.rs` | ROM header parsing, checksum, auto-detection |
| Pad | `pad.rs` | Keypad input with interrupt support |
| Diag | `diag.rs` | Diagnostic/debug output tooling |
| Top-level | `mod.rs` | `GameBoyAdvance` struct — main API entry point |
| Constants | `consts.rs` | Hardware constants (frequencies, display size, etc.) |

### Unified System Abstraction (`src/system.rs`)

New `System` enum that wraps either `GameBoy` or `GameBoyAdvance`, providing a single interface for frontends. Auto-detects ROM type via a fixed byte at offset `0xB2` in GBA headers.

### SDL Frontend Integration (`frontends/sdl/src/main.rs`)

The SDL frontend now auto-detects GB vs GBA ROMs and creates the appropriate emulator. Display resolution adjusts accordingly (160x144 for GB, 240x160 for GBA).

### Test Infrastructure

- **`src/gba_test.rs`**: Headless GBA test runner (mirrors existing GB test infrastructure)
- **`frontends/sdl/src/test.rs`**: Image comparison tests — runs ROMs and compares frame buffer output against reference PNGs
- **Test ROMs**: jsmolka/gba-tests (13 ROMs) and alyosha-tas DMA tests added under `res/roms.gba/test/`
- **Reference images**: PNG screenshots under `frontends/sdl/res/test/gba/`

### Other Changes

- `src/pad.rs`: Added `PadKey::L` and `PadKey::R` shoulder button variants
- `src/lib.rs`: Exports `gba` and `gba_test` modules
- `src/bin/gba-diag.rs`: CLI diagnostic tool for running GBA ROMs
- Boot ROMs added: `res/boot/agb_boot.bin`, `res/boot/agb_digiretro.bin`
- CHANGELOG.md updated with all new GBA features

## Architecture Decisions

1. **HLE BIOS**: SWI calls are handled via high-level emulation by default; real BIOS can optionally be loaded
2. **Scanline PPU**: Rendering is scanline-based (not pixel-accurate), sufficient for most games
3. **Simplified timing**: CPU uses a 1-cycle-per-instruction model. Wait state calculations exist in `bus.rs` but aren't fully integrated into CPU timing yet
4. **DMA cycle accounting**: Wait state math follows GBATEK (2N + 2(n-1)S + xI) but doesn't advance timers — this is a known limitation (see TODO in `mod.rs:251`)
5. **No BIOS boot sequence by default**: CPU starts in SYS mode at `0x08000000` (post-boot state) unless a real BIOS is loaded

## Test Status

- All 13 jsmolka/gba-tests pass: arm, thumb, memory, bios, sram, flash64, flash128, nes, none, unsafe, ppu_hello, ppu_shades, ppu_stripes
- DMA timing tests (alyosha-tas): smoke tests pass, timing-accuracy tests fail (need cycle-accurate bus)

## Known Limitations

- **DMA cycle timing not applied**: DMA cycles are calculated but discarded (`let _ = dma_cycles` in `mod.rs:257`). Timers don't advance during DMA, so timer-based DMA timing tests fail.
- **No fetch wait states in CPU**: Instruction fetch timing doesn't account for ROM wait states
- **No bus contention modeling**: CPU and DMA don't compete for bus access
- **Save states**: No BESS-format save state support for GBA yet
- **Web frontend**: GBA not yet integrated into the WASM/web frontend

## How to Build and Test

```bash
# Build
cargo build --release

# Run all tests (including GBA)
cargo test --lib -p boytacean

# Run SDL frontend with a GBA ROM (auto-detected)
cargo run --release -p boytacean-sdl -- <rom.gba>

# GBA diagnostic tool
cargo run --release --bin gba-diag -- <rom.gba> [frames]
```
