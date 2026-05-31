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

## New Release

To create a new release follow the following steps:

- Make sure that both the tests pass and the code formatting are valid.
- Increment (look at `CHANGELOG.md` for semver changes) the `version` value in all the `Cargo.toml` files across the repo, the `version` value in `setup.py`, the `version` value in `frontends/web/package.json` and the `display_version` value in `frontends/libretro/res/boytacean_libretro.info`.
- Update the dependencies in the multiple `Cargo.toml` files that are associated with boytacean crates and back references.
- Move all the `CHANGELOG.md` Unreleased items that have at least one non empty item the into a new section with the new version number and date, and then create new empty sub-sections (Added, Changed and Fixed) for the Unreleased section with a single empty item.
- Create a commit with the following message `version: $VERSION_NUMBER`.
- Push the commit.
- Create a new tag with the value fo the new version number `$VERSION_NUMBER`.
- Create a new release on the GitHub repo using the Markdown from the corresponding version entry in `CHANGELOG.md` as the description of the release and the version number as the title. Do not include the title of the release (version and date) in the description.
- Promote the release to the stable channel by merging `master` into the `stable` branch and pushing it (this triggers the stable and production deployments).
- Sync the tag and the affected branches (`master` and `stable`) to both the GitLab and GitHub remotes, since the deployment and publishing pipelines run across both.

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
