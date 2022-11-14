# [Boytacean](https://boytacean.pages.dev)

A Game Boy emulator that is written in Rust 🦀.

**This emulator has been written for educational purposes and shouldn't be taken too seriously.** But yeahh it plays games, which is cool... 🎮

## Features

* Supports Game Boy (DMG) emulation
* Simple navigable source-code
* Web and SDL front-ends
* Fullscreen mode
* Support for multiple MBCs: MBC1, MBC2, MBC3, and MBC5
* Cool bespoke display palettes built by [TheWolfBunny64](https://www.deviantart.com/thewolfbunny).
* Transparent RAM saving using [Web Storage API](https://developer.mozilla.org/docs/Web/API/Window/localStorage)
* GamePad support using [Web Gamepad API](https://developer.mozilla.org/docs/Web/API/Gamepad_API)
* Variable CPU clock speed
* Debug mode: VRAM and registers

## Deployments

| Provider  | Stable  | URL                                                              |
| --------- | ------- | ---------------------------------------------------------------- |
| Cloudfare | `True`  | [boytacean.pages.dev](https://boytacean.pages.dev)               |
| Cloudfare | `True`  | [prod.boytacean.pages.dev](https://prod.boytacean.pages.dev)     |
| Cloudfare | `True`  | [stable.boytacean.pages.dev](https://stable.boytacean.pages.dev) |
| Cloudfare | `False` | [master.boytacean.pages.dev](https://master.boytacean.pages.dev) |

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

## Web version

You can use some GET parameters to control the initial behaviour of the emulator.

| Parameter    | Type    | Description                                                                                    |
| ------------ | ------- | ---------------------------------------------------------------------------------------------- |
| `rom_url`    | String  | The URL from which the initial ROM is going to be loaded, should support CORS.                 |
| `url`        | String  | The same as `url`.                                                                             |
| `fullscreen` | Boolean | If the emulator should start in fullscreen mode.                                               |
| `debug`      | Boolean | If the "debugger" should start visible.                                                        |
| `keyboard`   | Boolean | If the on screen keyboard should start visible.                                                |
| `palette`    | String  | The name of the palette to be set at startup( eg: `christmas`, `hogwards`, `mariobros`, etc.). |

### Palettes

The palettes offered in the web version were provided by [TheWolfBunny64](https://www.deviantart.com/thewolfbunny).

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

* [GitHub - LIJI32/SameBoy (C)](https://github.com/LIJI32/SameBoy)
* [GitHub - binji/binjgb (C)](https://github.com/binji/binjgb)
* [GitHub - Rodrigodd/gameroy (Rust)](https://github.com/Rodrigodd/gameroy)
* [GitHub - simias/gb-rs (Rust)](https://github.com/simias/gb-rs)
* [GitHub - RubenG123/frosty (Rust)](https://github.com/RubenG123/frosty)
* [GitHub - feo-boy/feo-boy (Rust)](https://github.com/feo-boy/feo-boy)
* [GitHub - calvinbaart/gameboy (TypeScript)](https://github.com/calvinbaart/gameboy)

### Videos

* [YouTube - The Ultimate Game Boy Talk (33c3)](https://www.youtube.com/watch?v=HyzD8pNlpwI)

### Other

* [GitHub - gbdev/awesome-gbdev](https://github.com/gbdev/awesome-gbdev)
* [GitHub - Hacktix/Bootix](https://github.com/Hacktix/Bootix)

## License

Boyacian is currently licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
