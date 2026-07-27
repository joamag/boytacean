# Boytacean React Example

Embedding example for the [`boytacean-react`](../component) package, showing the three ways in which the emulator can be integrated in a React application:

* **Out of the box** - a single `<Boytacean />` element renders the display, binds the physical keyboard and shows the on screen game pad.
* **Custom layout** - providing children switches the component into provider only mode, the layout is then built from the exposed components and the `useBoytacean()`, `useBoytaceanStats()` and `useBoytaceanStatus()` hooks, with a remapped keyboard.
* **Custom keyboard** - no input component of the package is used, the keys are driven directly through the `press()` and `release()` actions of the context.

## Build

The example is built from the root of the web front-end, as it relies on both the WASM binary in `lib` and the packages that sit next to it.

```bash
cd frontends/web
npx parcel build example/index.html --dist-dir example/dist --cache-dir .parcel-cache-example --public-url .
```

The WASM binary must have been generated beforehand using:

```bash
wasm-pack build --release --target=web --out-dir=frontends/web/lib -- --features wasm
```

## Run

Serve the resulting `example/dist` directory with any static HTTP server and open it in the browser:

```bash
cd example/dist && python3 -m http.server
```

## Keys

| Key        | Action                             |
| ---------- | ---------------------------------- |
| Arrow keys | D-Pad                              |
| Enter      | Start                              |
| Space      | Select                             |
| A / S      | A / B                              |
| Z / X      | A / B (custom layout section only) |

Notice that only one emulator instance receives the WASM callbacks (speed switch, serial, printer and rumble) at any given time, as they are bound to the `window` namespace. Each instance still renders and handles input independently.
