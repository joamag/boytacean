# Boytacean

A Game Boy emulator that is written in Rust 🦀.

## Build

### WASM for Node.js

```bash
cargo install wasm-pack
wasm-pack build --release --target=nodejs -- --features wasm
```

### WASM for Web

```bash
cargo install wasm-pack
wasm-pack build --release --target=web --out-dir=examples/web/lib -- --features wasm
cd examples/web
npm install && npm run build
cd dist && python3 -m http.server
```

## Inspiration

### Documentation

* [Game Boy Development community](https://gbdev.io/)
* [Game Boy - Pan Docs](https://gbdev.io/pandocs)
* [GameBoy Emulation in JavaScript](http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-The-CPU)
* [POKEGB: a gameboy emulator that only plays Pokémon blue](https://binji.github.io/posts/pokegb)
* [Game Boy CPU Opcodes](https://izik1.github.io/gbops)
* [Codeslinger - Game Boy](http://www.codeslinger.co.uk/pages/projects/gameboy.html)
* [Game Boy hardware database](https://gbhwdb.gekkio.fi)

### ROMs

* [GitHub - c-sp/gameboy-test-roms](https://github.com/c-sp/gameboy-test-roms)

### Emulators

* [GitHub - binji/binjgb (C)](https://github.com/binji/binjgb)
* [GitHub - Rodrigodd/gameroy (Rust)](https://github.com/Rodrigodd/gameroy)
* [GitHub - simias/gb-rs (Rust)](https://github.com/simias/gb-rs)
* [GitHub - RubenG123/frosty (Rust)](https://github.com/RubenG123/frosty)
* [GitHub - calvinbaart/gameboy (TypeScript)](https://github.com/calvinbaart/gameboy)

### Videos

* [YouTube - The Ultimate Game Boy Talk (33c3)](https://www.youtube.com/watch?v=HyzD8pNlpwI)

## License

Boyacian is currently licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
