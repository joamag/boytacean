# Boytacean Libretro

## Build

```bash
cargo build
```

### Cross Compilation

#### Arm64 Linux

Download the linux toolchain from [Arm GNU Toolchain Downloads](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads).

Set the env variable `ARM64_TOOLCHAIN` to the path of the toolchain directory.

Create a toolchain symbolic link using the following command in Unix:

```bash
ln -s $ARM64_TOOLCHAIN aarch64-linux-gnu
```

... and the following command in Windows (cmd vs powershell):

```bash
mklink /D aarch64-linux-gnu %ARM64_TOOLCHAIN%
New-Item -ItemType SymbolicLink -Path aarch64-linux-gnu -Target $env:ARM64_TOOLCHAIN
```

To install the Rust targets for Arm64 Linux using rustup run:

```bash
rustup target add aarch64-unknown-linux-gnu
```

Then you're ready to build Boytacean's libretro core:

```bash
cargo build --target=aarch64-unknown-linux-gnu --release
```

#### Android

Configure `NDK_HOME` environment variable to point to your Android NDK directory and then create local toolchain replicas in the root project directory using:

```bash
mkdir -p ndk
${NDK_HOME}/build/tools/make_standalone_toolchain.py --api 26 --arch arm64 --install-dir ndk/arm64
${NDK_HOME}/build/tools/make_standalone_toolchain.py --api 26 --arch arm --install-dir ndk/arm
${NDK_HOME}/build/tools/make_standalone_toolchain.py --api 26 --arch x86 --install-dir ndk/x86
```

To install the Rust targets for Android using rustup run:

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
```

Then you're ready to build Boytacean's libretro core using the following commands (for release builds):

```bash
cargo build --target=aarch64-linux-android --release
cargo build --target=armv7-linux-androideabi --release
cargo build --target=i686-linux-android --release
```

## Run

### Mac OS

```bash
cargo build --release
cp -p ../../target/release/libboytacean_libretro.dylib ~/Library/Application\ Support/RetroArch/cores/boytacean_libretro.dylib
cp -p res/boytacean_libretro.info ~/Library/Application\ Support/RetroArch/info/boytacean_libretro.info
```

Then you should be able to see the Core available in RetroArch.

If there's a new for debugging information to be display in RetroArch console then use:

```bash
export RUST_BACKTRACE=1
cargo build --features debug
cp -p ../../target/debug/libboytacean_libretro.dylib ~/Library/Application\ Support/RetroArch/cores/boytacean_libretro.dylib
```
