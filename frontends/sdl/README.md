# Boytacean SDL

## Build

### Build SDL using [vcpkg](https://vcpkg.io/)

To be able to run the `cargo build` one must first install a local version of `vcpkg` using:

```bash
cargo install cargo-vcpkg
cargo vcpkg build
cargo build
```

### Build SDL in Linux

For Linux you may need to install additional dependencies, they should be installed before running the `cargo vcpkg build` command.

Here's a listing with the bash command for Ubuntu:

```bash
sudo apt install libsdl2-dev libasound2-dev libpulse-dev
```

#### Build SDL in Windows

For Windows the vcpkg based SDL building is simpler:

```bash
cargo install cargo-vcpkg
cargo build
```

Make sure that the current Rust in use is MSVC based using:

```bash
rustup default stable-msvc
```

For more information check [GitHub - Rust-SDL2/rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2).

### Build binaries

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
cargo build --features debug
```

To obtain more information about the issues.

### Troubleshooting

#### I'm facing issues with the vcpkg binaries

If there're issues with the `cargo vcpkg build` build process you may need to remove the `~/.vcpkg-root` and re-run the process to re-build the whole set of packages.
It's also important to completely delete the `target` directory.

### I'm having difficulties building SDL for arm64 in Mac OS

Try the above strategy and also try to remove `~/.cache/vcpkg`.
If that does not work try to remove the `Cargo.lock` file to flush dependencies.

A quick shortcut to the complete set of operations would be:

```bash
rm -rf ~/.vcpkg-root
rm -rf ~/.cache/vcpkg
cargo vcpkg -v build
```

If you're still facing issues try to remove the `installed` directory in the `vcpkg` directory:

```bash
rm -rf ~/vcpkg/installed
```

If by the end there are still issues, you can release the "atomic bomb" by removing the whole `.vcpkg` directory:

```bash
rm -rf ~/.vcpkg
```

## Execution

### Headless

It's possible to run the emulator in headless mode using the `--headless` parameter:

```bash
cargo run -- --rom-path ../../res/roms/test/blargg/cpu/cpu_instrs.gb --cycles 100000000 --headless --device stdout --unlimited
```

## Features

| Provider   | Description                                                                                                                                |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `debug`    | Activates the base `debug` feature from Boytacean.                                                                                         |
| `pedantic` | Additional safety instructions are executed to make sure the machine does no run "out of tracks", making sure to run many `panic()` calls. |
| `slow`     | Runs the emulator at a very slow page 60x slower to allow visual debugging.                                                                |
| `cpulog`   | Prints a log of the CPU instruction executed - will fill the stdout quickly.                                                               |

### Shaders

Use `--shader <SHADER>` to load a fragment shader. Example:

```bash
cargo run -- --rom-path path/to/game.gb --shader crt
```
