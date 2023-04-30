# Boytacean SDL

## Build

### Build SDL using [vcpkg](https://vcpkg.io/)

To be able to run the `cargo build` one must first install a local version of `vcpkg` using:

```bash
cargo install cargo-vcpkg
cargo vcpkg build
cargo build
```

#### Build SDL in Windows

For windows the vcpk based SDL building is simpler:

```bash
cargo install cargo-vcpkg
cargo build
```

Make sure that the current Rust in use is msvc based using:

```bash
rustup default stable-msvc
```

For more information check [GitHub - Rust-SDL2/rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2).

## Build binaries

Then you can use the following command to build and run Boytacean SDL:

```bash
cargo build
cargo run
```

To reload the code continuously use the cargo watch tool:

```bash
cargo install cargo-watch
cargo watch -x run
```

There are some feature flags that control the verbosity of the emulator to run in debug mode use:

```bash
cargo run --features debug
```

To obtain more information about the issues.

## Features

| Provider   | Description                                                                                                                                |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `debug`    | Activates the base `debug` feature from Boytacean.                                                                                         |
| `pedantic` | Additional safety instructions are executed to make sure the machine does no run "out of tracks", making sure to run many `panic()` calls. |
| `slow`     | Runs the emulator at a very slow page 60x slower to allow visual debugging.                                                                |
| `cpulog`   | Prints a log of the CPU instruction executed.                                                                                              |
