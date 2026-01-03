# CLAUDE.md

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
- `src/serial.rs` - Serial transfer (Link Cable) implementation
- `src/devices/network.rs` - Network-based serial device for netplay
- `src/netplay/` - Netplay protocol, session, and connection management

## Netplay Architecture

The netplay system enables multiplayer gaming over network by emulating the Game Boy Link Cable:

### Key Concepts

- **Master/Slave**: Game Boy serial uses master (internal clock) and slave (external clock) modes
- **SB Register (0xFF01)**: Serial transfer data register
- **SC Register (0xFF02)**: Serial transfer control register
- **Sync Mechanism**: Master must know slave's SB value before starting transfer

### Sync Protocol

1. **Proactive Sync**: Slave sends its SB value immediately when ready (writes to SC)
2. **SyncRequest Fallback**: If master starts before proactive sync arrives, it sends a SyncRequest
3. **Transfer Blocking**: Master waits (`awaiting_sync`) until slave's SB value is received

### Key Files

- `src/serial.rs` - SerialDevice trait with `sync()` and `awaiting_sync()` methods
- `src/devices/network.rs` - NetworkDevice with `peer_sp`, `saved_sb`, sync buffers
- `src/netplay/protocol.rs` - Message types (SerialByte, SyncByte, SyncRequest)
- `src/netplay/session.rs` - NetplaySession manages connection and message handling
- `frontends/sdl/src/main.rs` - SDL frontend netplay integration

## Build Commands

- **Rust**: `cargo build --release`
- **Rust (SDL)**: `cargo build --release -p boytacean-sdl`
- **Python**: `pip install -e .` or `python setup.py develop`
- **Web**: `cd frontends/web && yarn install && yarn build`

## After Completing Code

Always run these steps after making code changes:

### 1. Format Code

```bash
cargo fmt --all
```

### 2. Run Linter

```bash
cargo clippy --all-targets --all-features
```

### 3. Run Tests

```bash
# Run all library tests
cargo test --lib

# Run specific test module
cargo test --lib network
cargo test --lib serial
cargo test --lib netplay

# Run with output
cargo test --lib -- --nocapture
```

### 4. Build All Targets

```bash
# Build core library
cargo build

# Build SDL frontend
cargo build -p boytacean-sdl

# Build release versions
cargo build --release
cargo build --release -p boytacean-sdl
```

### 5. Verify No Warnings

```bash
cargo build 2>&1 | grep -i warning
```

## Code Style

### Rust Conventions

- Follows Rust conventions with `rustfmt.toml`
- Run `cargo fmt --all` before committing

### Inline Comments

- Use lowercase for inline comments (not doc comments)
- Use third-person descriptive style ("creates", "sends", "returns") not imperative ("create", "send", "return")
- No period at end of inline comments
- Examples:

```rust
// sends sync request if master needs slave's SB value
// clears peer_sp after transfer completes so the next transfer
// requires a fresh sync byte from the slave
// master mode: requests sync from slave if peer_sp is not available
```

### Doc Comments

- Doc comments (`///`) use proper capitalization
- First line should be a complete sentence describing what the function does
- Example:

```rust
/// Requests a sync from the peer by emitting a SyncRequestNeeded event.
/// Sets sync_pending to true to block further transfers until sync arrives.
pub fn request_sync(&mut self) {
```

### Other Style

- Python code follows PEP 8
- TypeScript/JavaScript uses standard formatting

## Testing

- Blargg test ROMs for CPU and sound accuracy
- Acid2 tests for PPU accuracy
- Custom test suite in `src/test.rs`
- Python tests in `src/python/boytacean/test/`
- Netplay tests in `src/netplay/` and `src/devices/network.rs`

## Development Notes

- Uses `wasm-pack` for WebAssembly builds
- Supports multiple Game Boy variants (DMG, CGB, SGB)
- Includes extensive test ROMs in `res/roms/test/`
- Boot ROMs available in `res/boot/`
- Custom shader support in SDL frontend
