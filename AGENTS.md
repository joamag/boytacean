# AGENTS.md file

This document describes how to work with the project. Follow these notes when writing code or submitting pull requests.

## Highlights

- Game Boy (DMG) and Game Boy Color (CGB) emulator written in Rust with a focus on performance and safety
- Release builds use LTO, `opt-level = 3`, single codegen unit, and optional SIMD acceleration
- Four frontends: Web (WASM), SDL (desktop with shader support), Libretro (RetroArch), and Console
- Web frontend built with React/TypeScript and powered by [EmuKit](https://github.com/joamag/emukit) for emulator UI infrastructure
- Core compiles to WebAssembly via `wasm-pack`, sharing the same emulation logic across all targets
- Passes dmg-acid2 and cgb-acid2 PPU tests; accurate APU with configurable high-pass audio filters
- Supports MBC1–MBC5, Link Cable, Game Boy Printer, Game Genie/GameShark cheats, and BESS save states
- Python bindings via PyO3; published to crates.io, npm, and PyPI
- Mobile-first web experience with on-screen gamepad, haptic feedback, and Web Gamepad API support
- Uses `cargo` for Rust and `pip` for Python; licensed under Apache 2.0

## Setup

Install Python packages and the Rust toolchain:

```bash
pip install -r requirements.txt
rustup default nightly
rustup component add rustfmt
rustup component add clippy
cargo install cargo-vcpkg
```

## Formatting

Format all code before committing:

```bash
cargo fmt --all
cargo clippy --fix --allow-dirty --allow-staged --all-features --all-targets
black .
```

## Testing

Run the full test suite:

```bash
cargo test --all-targets --features simd,debug,python
```

## SDL Frontend

To build the SDL frontend (if required) use:

```bash
cd frontends/sdl && cargo build
```

## Style Guide

- Always update `CHANGELOG.md` according to semantic versioning, mentioning your changes in the unreleased section.
- Write commit messages using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
- Never bump the internal package version in `Cargo.toml` or `setup.py`. This is handled automatically by the release process.
- Rust files use LF line endings, while Python files use CRLF.
- Inline comments should be in the format `// <comment>` and start with lowercase.

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
