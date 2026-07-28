# boytacean-react

The React component of [Boytacean](https://github.com/joamag/boytacean), a Game Boy (DMG) and Game Boy Color (CGB) emulator written in Rust 🦀 and compiled to WebAssembly.

Drop a `<Boytacean />` element into your application and you get a running emulator, with the display, the physical keyboard binding and the on screen game pad included.

## Installation

```bash
npm install boytacean-react boytacean-core boytacean
```

Both [`boytacean-core`](https://www.npmjs.com/package/boytacean-core) (the headless emulator logic) and [`boytacean`](https://www.npmjs.com/package/boytacean) (the WASM binary) are peer dependencies, alongside `react` 18.

## Build a working example

The following block goes from nothing to an emulator running in the browser, copy and paste it as it is.

```bash
npm create vite@latest my-emulator -- --template react-ts
cd my-emulator

# the component targets React 18, while recent versions of the Vite
# template scaffold React 19, so the version is pinned before install
npm install react@^18 react-dom@^18 @types/react@^18 @types/react-dom@^18
npm install boytacean-react boytacean-core boytacean

# any Game Boy ROM works, this one is the demo shipped by the project
mkdir -p public/roms
curl -L -o public/roms/pocket.gb \
    https://github.com/joamag/boytacean/raw/master/res/roms/demo/pocket.gb

rm -f src/App.tsx src/App.css src/index.css
cat > src/main.tsx <<'EOF'
import React from "react";
import { createRoot } from "react-dom/client";
import { Boytacean } from "boytacean-react";

import "boytacean-react/styles.css";

const App = () => <Boytacean rom="/roms/pocket.gb" palette="christmas" />;

createRoot(document.getElementById("root")!).render(<App />);
EOF

npx vite
```

Open the URL printed by Vite and the Nintendo boot logo shows up, followed by the demo ROM.

To produce a production build use `npx vite build` and serve the resulting `dist` directory with any static HTTP server.

Notice that `npx vite` is used instead of `npm run dev`, as the `build` script of the template also runs `tsc -b` over the sources of [EmuKit](https://github.com/joamag/emukit), which are published as TypeScript and do not survive the strict compiler options of the template. Building through Vite directly skips that type check, which is not needed to run the emulator. Add `"skipLibCheck": true` and drop `erasableSyntaxOnly` from `tsconfig.app.json` in case the template scripts are preferred.

Up to (and including) version `0.13.2` the emulator still boots but logs a `404` for `lib/boytacean_bg.wasm` on startup, as the default location of the WASM binary was only valid inside the repository, the binary is then loaded from the fallback of `wasm-bindgen`. Pass an explicit `wasmPath` to silence it:

```tsx
import wasmUrl from "boytacean/boytacean_bg.wasm?url";

<Boytacean rom="/roms/pocket.gb" wasmPath={wasmUrl} />;
```

## Quick start

```tsx
import React from "react";
import { Boytacean } from "boytacean-react";

import "boytacean-react/styles.css";

export const App = () => (
    <Boytacean rom="/roms/pocket.gb" palette="christmas" />
);
```

That single element boots the ROM, renders the display, binds the arrow keys plus `Enter`, `Space`, `A` and `S`, and shows the on screen game pad for touch devices.

## A complete emulation page

A minimal page that lets the visitor play a bundled ROM and load one from their own machine.

```tsx
import React, { ChangeEvent } from "react";
import { createRoot } from "react-dom/client";
import {
    Boytacean,
    useBoytacean,
    useBoytaceanStats,
    useBoytaceanStatus
} from "boytacean-react";

import "boytacean-react/styles.css";

const Controls = () => {
    const { play, pause, reset, loadRom } = useBoytacean();
    const { romName } = useBoytaceanStatus();
    const { framerate } = useBoytaceanStats();

    const onFile = async (event: ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file) return;
        const data = new Uint8Array(await file.arrayBuffer());
        await loadRom(file.name, data);
    };

    return (
        <div>
            <button onClick={() => play()}>Play</button>
            <button onClick={() => pause()}>Pause</button>
            <button onClick={() => reset()}>Reset</button>
            <input type="file" accept=".gb,.gbc" onChange={onFile} />
            <span>
                {romName ?? "no ROM"} · {framerate} FPS
            </span>
        </div>
    );
};

const App = () => (
    <Boytacean.Provider rom="/roms/pocket.gb" palette="christmas">
        <Boytacean.Screen scale={3} />
        <Controls />
        <Boytacean.Keyboard />
        <Boytacean.Gamepad />
    </Boytacean.Provider>
);

createRoot(document.getElementById("app")!).render(<App />);
```

Serve the page with any bundler that can emit the WASM binary as an asset (Parcel, Vite, webpack, ...) and put your `.gb` files under the public directory.

## Components

Passing `children` to `<Boytacean />` switches it into provider only mode, so none of the default presentation is rendered and the layout is entirely yours. The pieces are also available as `Boytacean.Provider`, `Boytacean.Screen`, `Boytacean.Gamepad` and `Boytacean.Keyboard`.

| Component               | Description                                                                                    |
| ----------------------- | ---------------------------------------------------------------------------------------------- |
| `<Boytacean />`         | Everything at once, or just the provider whenever `children` are given.                        |
| `<BoytaceanProvider />` | Owns the emulator lifecycle, booting it on mount and stopping it on unmount.                   |
| `<BoytaceanScreen />`   | Draws the frame buffer into a canvas, imperatively and without triggering any React re-render. |
| `<BoytaceanGamepad />`  | On screen game pad, the out of the box input method for touch devices.                         |
| `<BoytaceanKeyboard />` | Binds the physical keyboard to the emulated game pad, renders nothing.                         |

### Props

`<Boytacean />` accepts `rom`, `wasmPath`, `storage`, `palette`, `system`, `scale`, `keyboard`, `gamepad`, `className`, `style` and `children`. The provider takes the first five.

| Prop       | Type              | Default  | Description                                                                  |
| ---------- | ----------------- | -------- | ---------------------------------------------------------------------------- |
| `rom`      | `string`          | —        | URL of the ROM to load at boot, the machine stays idle when omitted.         |
| `wasmPath` | `string`          | —        | Where to load the WASM binary from, defaults to the wasm-bindgen resolution. |
| `storage`  | `StorageAdapter`  | —        | Persistence for the battery backed RAM and the settings.                     |
| `palette`  | `string`          | —        | Palette applied at startup, only used by the Game Boy.                       |
| `system`   | `BoytaceanSystem` | `"auto"` | System to emulate, inferred from the ROM extension in the automatic mode.    |
| `scale`    | `number`          | `3`      | Scale factor of the display, ignored when custom children are provided.      |
| `keyboard` | `boolean`         | `true`   | Whether the physical keyboard is bound, ignored with custom children.        |
| `gamepad`  | `boolean`         | `true`   | Whether the on screen game pad is shown, ignored with custom children.       |

Built-in palettes: `basic`, `hogwards`, `christmas`, `goldsilver`, `pacman`, `mariobros` and `pokemon`.

## Game Boy Advance

The same component runs Game Boy Advance ROMs, building a `GbaCore` instead of a `GameBoyCore` whenever the system resolves to the GBA. A `.gba` ROM is detected out of the box:

```tsx
<Boytacean rom="/roms/pocket.gba" />
```

Use the `system` property to select it explicitly, which is required whenever the ROM is not loaded from a URL that carries the extension:

```tsx
import { Boytacean, BoytaceanSystem } from "boytacean-react";

<Boytacean rom="/roms/pocket" system={BoytaceanSystem.GameBoyAdvance} />;
```

The display is sized from the core, so the screen switches between the 160x144 of the Game Boy and the 240x160 of the GBA on its own. Notice that the palette is ignored by the GBA, as the colors come from the ROM itself, and that save states are not yet supported by it.

## Hooks

* `useBoytacean()` — the context of the closest provider: the `core` instance and the `system` it emulates, plus the `play()`, `pause()`, `reset()`, `loadRom()`, `press()` and `release()` actions. Its identity is stable for the lifetime of the provider, so consumers are never re-rendered by emulation activity.
* `useBoytaceanStatus()` — `booted` and `romName`, updated only when a ROM is booted.
* `useBoytaceanStats()` — `framerate`, `cyclerate` and `emulationSpeed`, sampled every 500 ms by default (`useBoytaceanStats(interval)` to change it).

High frequency data such as the frame buffer is deliberately kept out of React state, bind to the events of `core` directly when you need it.

## Keys

| Key        | Action           |
| ---------- | ---------------- |
| Arrow keys | D-Pad            |
| Enter      | Start            |
| Space      | Select           |
| A / S      | A / B            |
| Q / W      | L / R (GBA only) |

Remap them by passing a `keys` record to `<Boytacean.Keyboard />`:

```tsx
<Boytacean.Keyboard keys={{ ...KEYS_MAP, z: "A", x: "B" }} />
```

Or skip the component and drive `press()` / `release()` from the context yourself, using the key names `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Start`, `Select`, `A`, `B`, `L` and `R`.

## Persistence

By default the core persists the battery backed RAM and its settings through the Web Storage API. Provide your own `StorageAdapter` (a synchronous `getItem` / `setItem` pair) to change that, or use `NullStorageAdapter` from `boytacean-core` to disable persistence entirely:

```tsx
import { NullStorageAdapter } from "boytacean-core";

<Boytacean rom="/roms/pocket.gb" storage={new NullStorageAdapter()} />;
```

## Notes

Only one emulator instance receives the WASM callbacks (speed switch, serial, printer and rumble) at any given time, as they are bound to the `window` namespace. Multiple instances still render and handle input independently.

A runnable version of these examples lives in [`frontends/web/example`](https://github.com/joamag/boytacean/tree/master/frontends/web/example).

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
