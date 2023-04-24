# Boytacean SDL

## Build

To be able to run the `cargo build` one must first install a local version of `vcpkg` using:

```bash
cargo install cargo-vcpkg
cargo vcpkg build
cargo build
```

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
