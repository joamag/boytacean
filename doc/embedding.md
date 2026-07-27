# Embedding

Boytacean can be embedded in any website through the [`boytacean-web`](https://www.npmjs.com/package/boytacean-web) package, which ships a self-contained build of the emulator (WebAssembly core and UI included). No build step, no extra runtime dependencies and no server-side component are required.

There are four ways to embed the emulator:

1. A [React component](#react-component), for React applications.
2. A [custom element](#custom-element) driven by HTML attributes, for plain HTML pages.
3. The [`embed()` function](#javascript-api) for programmatic control outside of React.
4. An [`<iframe>`](#iframe) pointing at the hosted front-end, for maximum isolation.

## React component

Install the package and render the `Boytacean` component, it mounts the emulator inside the React tree of the host application:

```bash
npm install boytacean-web
```

```jsx
import { Boytacean } from "boytacean-web/react";

export const Page = () => (
    <Boytacean
        romUrl="https://example.com/game.gb"
        palette="christmas"
        style={{ maxWidth: 760 }}
    />
);
```

`react` and `react-dom` are peer dependencies (React 18), everything else the emulator needs is either bundled or installed automatically.

The stylesheet is imported by the component, so bundlers pick it up without any extra setup. Should your setup strip CSS imports, import it explicitly with `import "boytacean-web/react/react.css"`.

### Props

| Prop            | Type       | Default      | Description                                                                          |
| --------------- | ---------- | ------------ | ------------------------------------------------------------------------------------ |
| `romUrl`        | String     | bundled ROM  | The URL of the ROM to load at startup, the origin must support CORS.                 |
| `playlistUrl`   | String     | —            | The URL of a JSON [playlist](playlists.md), enables the playlist section.            |
| `palette`       | String     | `basic`      | The palette to use at startup.                                                       |
| `background`    | String     | `264653`     | The background color of the emulator area, hexadecimal without the `#` prefix.       |
| `backgrounds`   | String[]   | built-in     | The background colors offered in the UI.                                             |
| `fullscreen`    | Boolean    | `false`      | If the emulator should start in fullscreen mode.                                     |
| `keyboard`      | Boolean    | `false`      | If the on-screen keyboard should start visible.                                      |
| `debug`         | Boolean    | `false`      | If the debug panels should start visible.                                            |
| `verbose`       | Boolean    | `false`      | If information should be logged in verbose mode.                                     |
| `info`          | Boolean    | `false`      | If the informative panels should be shown.                                           |
| `storagePrefix` | String     | `boytacean:` | The prefix for the `localStorage` keys.                                              |
| `contain`       | Boolean    | `true`       | If the page level side effects should be contained, see [Containment](#containment). |
| `className`     | String     | —            | Extra class names for the container element.                                         |
| `style`         | CSSProperties | —         | Inline styles for the container element.                                             |
| `onReady`       | Function   | —            | Called with the emulator instance once it has booted and is running.                 |
| `onError`       | Function   | —            | Called with the error when the boot process fails.                                   |
| `onBackground`  | Function   | —            | Called with the new color whenever the background changes through the UI.            |

The emulator is created once on mount, so the props above are only read at that point. Use the ref handle (or the `onReady` instance) to drive it afterwards.

### Ref handle

The component forwards a ref exposing `emulator`, `pause()`, `resume()` and `loadRom(url)`:

```jsx
import { useRef } from "react";
import { Boytacean } from "boytacean-web/react";

export const Page = () => {
    const emulator = useRef(null);
    return (
        <>
            <button onClick={() => emulator.current.pause()}>Pause</button>
            <button onClick={() => emulator.current.loadRom("/other.gb")}>Swap ROM</button>
            <Boytacean ref={emulator} onReady={(gb) => console.log(gb.romName)} />
        </>
    );
};
```

### Multiple components

Several `Boytacean` components can coexist in the same tree, each with its own ROM, palette and storage. Give them distinct `storagePrefix` values to keep their saved games separate:

```jsx
<Boytacean romUrl="/first.gb" storagePrefix="first:" />
<Boytacean romUrl="/second.gb" storagePrefix="second:" />
```

Every running instance consumes a full emulation loop, so keep the number of simultaneously running emulators low and `pause()` the ones that are not visible.

## Custom element

Load the bundle with a single script tag, then use the `<boytacean-emulator>` element anywhere in the page:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/boytacean-web/embed/embed.js"></script>

<boytacean-emulator rom-url="https://example.com/game.gb"></boytacean-emulator>
```

The element registers itself on import and boots as soon as it is connected to the document. The stylesheet is loaded automatically from next to the bundle.

The element is a normal block level element, so size and frame it with CSS as you would any other:

```html
<boytacean-emulator
    rom-url="https://example.com/game.gb"
    palette="christmas"
    style="display: block; max-width: 760px; border-radius: 8px;"
></boytacean-emulator>
```

### Attributes

| Attribute        | Type    | Description                                                                                     |
| ---------------- | ------- | ----------------------------------------------------------------------------------------------- |
| `rom-url`        | String  | The URL of the ROM to load at startup, the origin must support CORS. Defaults to the bundled ROM. |
| `playlist-url`   | String  | The URL of a JSON [playlist](playlists.md), enables the playlist section.                        |
| `palette`        | String  | The palette to use at startup (eg: `christmas`, `hogwards`, `mariobros`).                        |
| `background`     | String  | The background color of the emulator area, hexadecimal without the `#` prefix.                   |
| `fullscreen`     | Boolean | If the emulator should start in fullscreen mode.                                                 |
| `keyboard`       | Boolean | If the on-screen keyboard should start visible, a sensible default for touch oriented pages.     |
| `debug`          | Boolean | If the debug panels should start visible.                                                        |
| `info`           | Boolean | If the informative panels should be shown.                                                       |
| `storage-prefix` | String  | The prefix for the `localStorage` keys, defaults to `boytacean:`.                                |
| `styles`         | String  | The location of the stylesheet, or `false` to skip loading it.                                   |

Boolean attributes follow the usual HTML convention, so `<boytacean-emulator fullscreen>` and `fullscreen="true"` are equivalent.

### Element API

The element exposes `pause()`, `resume()` and `loadRom(url)` methods, an `emulator` property with the underlying emulator instance, and `ready` / `error` events:

```html
<boytacean-emulator id="emulator"></boytacean-emulator>

<script type="module">
    const element = document.querySelector("#emulator");
    element.addEventListener("ready", () => console.log("emulator is running"));
    element.addEventListener("error", (event) => console.error(event.detail.error));

    await element.loadRom("https://example.com/other.gb");
</script>
```

## JavaScript API

Install the package to mount the emulator programmatically:

```bash
npm install boytacean-web
```

```javascript
import { embed } from "boytacean-web";

const handle = await embed("#emulator", {
    romUrl: "https://example.com/game.gb",
    palette: "christmas"
});

await handle.pause();
await handle.resume();
await handle.loadRom("https://example.com/other.gb");
handle.destroy();
```

The `embed()` promise resolves once the ROM has booted and the emulation is running. The returned handle also exposes the `emulator` instance for the more advanced use cases, and the `container` element it has been mounted into.

The first argument accepts either a CSS selector or an already resolved element.

When loading the browser bundle from a script tag instead of a bundler, the same API is available as `window.Boytacean.embed`:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/boytacean-web/embed/embed.js"></script>

<script type="module">
    while (!window.Boytacean) await new Promise((r) => setTimeout(r, 50));
    await window.Boytacean.embed("#emulator", { romUrl: "https://example.com/game.gb" });
</script>
```

### Options

| Option          | Type              | Default        | Description                                                                          |
| --------------- | ----------------- | -------------- | ------------------------------------------------------------------------------------ |
| `romUrl`        | String            | bundled ROM    | The URL of the ROM to load at startup, the origin must support CORS.                 |
| `playlistUrl`   | String            | —              | The URL of a JSON [playlist](playlists.md), enables the playlist section.            |
| `palette`       | String            | `basic`        | The palette to use at startup.                                                       |
| `background`    | String            | `264653`       | The background color of the emulator area, hexadecimal without the `#` prefix.       |
| `backgrounds`   | String[]          | built-in list  | The background colors offered in the UI.                                             |
| `fullscreen`    | Boolean           | `false`        | If the emulator should start in fullscreen mode.                                     |
| `keyboard`      | Boolean           | `false`        | If the on-screen keyboard should start visible.                                      |
| `debug`         | Boolean           | `false`        | If the debug panels should start visible.                                            |
| `verbose`       | Boolean           | `false`        | If information should be logged in verbose mode.                                     |
| `info`          | Boolean           | `false`        | If the informative panels should be shown.                                           |
| `storagePrefix` | String            | `boytacean:`   | The prefix for the `localStorage` keys.                                              |
| `styles`        | Boolean \| String | `true`         | Controls the loading of the stylesheet, see [Styles](#styles).                       |
| `contain`       | Boolean           | `true`         | If the page level side effects should be contained, see [Containment](#containment). |

## Styles

The stylesheet is taken care of automatically in both setups: the bundler build imports it, and the browser build finds it next to its own script tag. Nothing needs to be done in the common cases.

If your setup strips the CSS import (or serves it from somewhere else), import it explicitly:

```javascript
import "boytacean-web/module/embed.css";
```

Alternatively, point the `styles` option at the stylesheet (`styles: "/assets/embed.css"`), or set it to `false` to take full control over the styling.

## Containment

The emulator UI is primarily built for the standalone front-end, where it owns the whole page. When embedded it would otherwise paint the host page's `<body>`, pin its footer to the viewport and lay itself out against the viewport size, so by default the embed:

- Redirects the background color to its own container instead of the document's body.
- Scopes the layout and the footer to the element it has been mounted into.
- Makes the responsive breakpoints follow the width of the emulator rather than the one of the window, so that a narrow emulator in a wide page still lays out correctly.

Set `contain` to `false` only when the emulator owns the complete page.

Note that the styling is **not** fully isolated — the emulator's CSS and the host page's CSS share the same document. If the host page has aggressive global rules (or the emulator's rules would disturb it), use an [`<iframe>`](#iframe) instead.

## Storage

Battery backed cartridge RAM and the emulator settings are persisted to the host page's `localStorage`, namespaced under the `storagePrefix` (`boytacean:` by default) so that they do not collide with the keys of the host page.

Two embeds sharing a prefix also share their saved games, which is usually the desired behaviour. Give them distinct prefixes to keep them separate.

## Multiple instances

Multiple emulators can coexist in the same page, each with its own ROM, palette and storage:

```javascript
await embed("#first", { romUrl: "/first.gb", storagePrefix: "first:" });
await embed("#second", { romUrl: "/second.gb", storagePrefix: "second:" });
```

Note that every running instance consumes a full emulation loop, so keep the number of simultaneously running emulators low, and `pause()` the ones that are not visible.

## iframe

For maximum isolation (separate styling, storage and JavaScript context) embed the hosted front-end in an `<iframe>` and configure it through the [GET parameters](../README.md#configuration):

```html
<iframe
    src="https://boytacean.joao.me/?rom_url=https://example.com/game.gb&fs=1"
    allow="gamepad; autoplay"
    style="width: 100%; height: 600px; border: 0;"
></iframe>
```

The `allow="gamepad"` attribute is required for the Web Gamepad API to work inside the frame.

## Requirements

- The ROM (and playlist) origins must allow **CORS**, they are fetched directly by the browser.
- Host pages that set a `Content-Security-Policy` need `wasm-unsafe-eval` in `script-src` for the WebAssembly core to be instantiated.
- Audio only starts after a user interaction, as required by the browser autoplay policies.
- The containment relies on CSS container queries and `:has()`, available in all the major browsers since 2023.
- The React component requires React 18.

## Legal

Embedding makes it easy to publish a ROM alongside the emulator. Only distribute ROMs you have the right to distribute, such as homebrew, public domain titles, or your own dumps.
