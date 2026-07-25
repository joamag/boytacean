import { embed, EmbedHandle, EmbedOptions } from "./embed";

/**
 * The default tag name used to register the emulator custom element
 * in the browser's custom elements registry.
 */
export const ELEMENT_NAME = "boytacean-emulator";

/**
 * Parses a string attribute value using the "loose" boolean rules
 * that are common in HTML, an empty value (eg: `<x flag>`) is
 * considered to be `true`.
 */
const parseBoolean = (value: string | null): boolean | undefined => {
    if (value === null) return undefined;
    if (value === "") return true;
    return ["1", "true", "True", "yes"].includes(value);
};

/**
 * Custom element that runs a Game Boy emulator, meant to be used by
 * external websites that want to embed Boytacean using plain HTML.
 *
 * The attributes mirror the {@link EmbedOptions} structure using the
 * kebab-case naming that is idiomatic in HTML.
 *
 * @example
 * <boytacean-emulator rom-url="/roms/game.gb" fullscreen></boytacean-emulator>
 */
export class BoytaceanEmulatorElement extends HTMLElement {
    /**
     * The handle of the underlying emulator, only available after the
     * `ready` event has been dispatched.
     */
    private handle: EmbedHandle | null = null;

    /**
     * The promise of the pending mount operation, used to avoid
     * duplicate mounts in case the element is re-connected.
     */
    private pending: Promise<EmbedHandle> | null = null;

    /**
     * The emulator instance associated with the element, `null` in case
     * the emulator has not finished booting yet.
     */
    get emulator() {
        return this.handle?.emulator ?? null;
    }

    connectedCallback() {
        if (this.pending) return;

        // guarantees the element establishes a layout context, otherwise
        // the emulator would be mounted into a zero height container
        if (!this.style.display) this.style.display = "block";

        this.pending = embed(this, this.options())
            .then((handle) => {
                this.handle = handle;
                this.dispatchEvent(
                    new CustomEvent("ready", { detail: { handle: handle } })
                );
                return handle;
            })
            .catch((err) => {
                this.dispatchEvent(
                    new CustomEvent("error", { detail: { error: err } })
                );
                throw err;
            });
    }

    disconnectedCallback() {
        this.handle?.destroy();
        this.handle = null;
        this.pending = null;
    }

    async pause() {
        await (await this.pending)?.pause();
    }

    async resume() {
        await (await this.pending)?.resume();
    }

    async loadRom(romUrl: string) {
        await (await this.pending)?.loadRom(romUrl);
    }

    /**
     * Builds the embed options from the element's attributes, omitting
     * the ones that have not been set so that the defaults apply.
     */
    private options(): EmbedOptions {
        const options: EmbedOptions = {};
        const string = (name: string) => this.getAttribute(name) ?? undefined;
        const boolean = (name: string) => parseBoolean(this.getAttribute(name));

        const romUrl = string("rom-url");
        const playlistUrl = string("playlist-url");
        const palette = string("palette");
        const background = string("background");
        const storagePrefix = string("storage-prefix");
        const styles = string("styles");
        const fullscreen = boolean("fullscreen");
        const keyboard = boolean("keyboard");
        const debug = boolean("debug");
        const info = boolean("info");

        if (romUrl !== undefined) options.romUrl = romUrl;
        if (playlistUrl !== undefined) options.playlistUrl = playlistUrl;
        if (palette !== undefined) options.palette = palette;
        if (background !== undefined) options.background = background;
        if (storagePrefix !== undefined) options.storagePrefix = storagePrefix;
        if (styles !== undefined) {
            // an explicit "false" disables the injection, any other value
            // is taken as the location of the stylesheet to be loaded
            options.styles = styles === "false" ? false : styles;
        }
        if (fullscreen !== undefined) options.fullscreen = fullscreen;
        if (keyboard !== undefined) options.keyboard = keyboard;
        if (debug !== undefined) options.debug = debug;
        if (info !== undefined) options.info = info;

        return options;
    }
}

/**
 * Registers the emulator custom element in the browser's registry,
 * the operation is idempotent so it's safe to call it multiple times.
 *
 * @param name The tag name to be used in the registration.
 */
export const defineEmulatorElement = (name = ELEMENT_NAME) => {
    if (typeof customElements === "undefined") return;
    if (customElements.get(name)) return;
    customElements.define(name, BoytaceanEmulatorElement);
};
