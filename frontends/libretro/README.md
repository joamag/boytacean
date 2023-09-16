# Boytacean Libretro

## Build

```bash
cargo build
```

### Android

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
