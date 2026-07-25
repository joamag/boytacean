import {
    BoytaceanEmulatorElement,
    defineEmulatorElement,
    embed,
    ensureStyles
} from "./ts";

export * from "./ts/element";
export * from "./ts/embed";
export { GameboyEmulator, SerialDevice } from "./ts/gb";
export { PALETTES, PALETTES_MAP } from "./ts/palettes";

// registers the custom element as a side effect of the import, so that
// a plain `<script type="module" src="...">` tag is enough to make the
// `<boytacean-emulator>` tag usable in the host page
defineEmulatorElement();

// exposes the API in the global scope, the browser oriented bundle is
// not an ES module with named exports, so this is what allows the
// imperative API to be reached from a plain script tag
window.Boytacean = {
    embed: embed,
    ensureStyles: ensureStyles,
    defineEmulatorElement: defineEmulatorElement,
    BoytaceanEmulatorElement: BoytaceanEmulatorElement
};

declare global {
    interface Window {
        Boytacean: {
            embed: typeof embed;
            ensureStyles: typeof ensureStyles;
            defineEmulatorElement: typeof defineEmulatorElement;
            BoytaceanEmulatorElement: typeof BoytaceanEmulatorElement;
        };
    }
}
