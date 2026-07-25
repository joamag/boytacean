import { startApp } from "emukit";

import { GameboyEmulator } from "./gb";

import "./embed.css";

/**
 * The prefix used to namespace the `localStorage` keys of an
 * embedded emulator, avoiding collisions with the keys that the
 * host page (or another embed) may be using.
 */
export const EMBED_STORAGE_PREFIX = "boytacean:";

/**
 * List of available background theme colors that can be used
 * to style the main emulator area.
 */
export const BACKGROUNDS = [
    "264653",
    "1b1a17",
    "023047",
    "bc6c25",
    "283618",
    "2a9d8f",
    "3a5a40"
];

/**
 * The name of the stylesheet that is emitted next to the embed bundle,
 * loaded on demand so that a single script tag is enough to embed the
 * emulator in an external page.
 */
const STYLESHEET_NAME = "embed.css";

/**
 * The name of the embed bundle, used to locate its own script tag and
 * from it the base location of the sibling assets.
 */
const BUNDLE_NAME = "embed.js";

/**
 * The attribute used to mark the injected stylesheet, avoids the
 * duplicate injection of the styles for multiple emulators.
 */
const STYLESHEET_ATTRIBUTE = "data-boytacean-styles";

/**
 * The class set on the mount node of an embedded emulator, used by the
 * embed stylesheet to keep the layout contained within the host element.
 */
const MOUNT_CLASS = "boytacean-embed";

/**
 * Sequence counter used to generate unique element identifiers
 * for the mount nodes of the multiple embedded emulators.
 */
let sequence = 0;

/**
 * Tries to determine the location the embed bundle has been served
 * from, by looking up its own script tag.
 *
 * Note that `import.meta.url` is not usable for this purpose as it gets
 * inlined at build time by the bundler, and `document.currentScript` is
 * always `null` for module scripts.
 *
 * @returns The URL of the bundle or `null` in case it's not possible to
 * determine it (eg: the code has been re-bundled by the host).
 */
const bundleUrl = (): string | null => {
    const script = document.querySelector<HTMLScriptElement>(
        `script[src$="${BUNDLE_NAME}"], script[src*="${BUNDLE_NAME}?"]`
    );
    return script?.src ?? null;
};

/**
 * Determines if the emulator stylesheet is already present in the
 * document, by probing one of the rules it's known to define.
 *
 * This is what makes the injection unnecessary for the setups where the
 * CSS reaches the page through a bundler instead of a script tag.
 */
const hasStyles = (): boolean => {
    const probe = document.createElement("div");
    probe.className = MOUNT_CLASS;
    document.body.appendChild(probe);
    const applied = getComputedStyle(probe).position === "relative";
    probe.remove();
    return applied;
};

/**
 * Injects the stylesheet required by the emulator UI into the host
 * page, the operation is idempotent.
 *
 * By default the stylesheet is resolved relative to the location of the
 * embed bundle, which is the expected layout of the published package.
 * Nothing is done when the styles are already in the page, which is the
 * case when the emulator is imported through a bundler.
 *
 * @param href The location of the stylesheet, inferred from the
 * bundle's own location in case it's not provided.
 */
export const ensureStyles = (href?: string) => {
    if (typeof document === "undefined") return;
    if (document.querySelector(`link[${STYLESHEET_ATTRIBUTE}]`)) return;
    if (hasStyles()) return;

    const base = href ?? bundleUrl();
    if (!base) {
        console.warn(
            `Boytacean is unable to locate its stylesheet, import ` +
                `"boytacean-web/embed/embed.css" in the host page or set ` +
                `the "styles" option to its location`
        );
        return;
    }

    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href ?? new URL(STYLESHEET_NAME, base).toString();
    link.setAttribute(STYLESHEET_ATTRIBUTE, "");
    document.head.appendChild(link);
};

/**
 * The set of options that control the initial state of an embedded
 * emulator, mirrors the GET parameters supported by the standalone
 * web front-end.
 */
export type EmbedOptions = {
    /**
     * The URL of the ROM to be loaded at startup, the remote origin
     * must support CORS. In case it's not provided the default ROM
     * that is bundled with the emulator is used instead.
     */
    romUrl?: string;
    /**
     * The URL of a JSON playlist file with a list of ROMs, enables
     * the playlist section in the emulator UI.
     */
    playlistUrl?: string;
    /**
     * The name of the palette to be used at startup (eg: `christmas`,
     * `hogwards`, `mariobros`, etc.).
     */
    palette?: string;
    /**
     * The background color (hexadecimal, without the `#` prefix) to be
     * used in the emulator area.
     */
    background?: string;
    /**
     * The list of background colors made available in the UI, defaults
     * to the same list used by the standalone front-end.
     */
    backgrounds?: string[];
    /**
     * If the emulator should occupy the complete area of its container.
     */
    fullscreen?: boolean;
    /**
     * If the on-screen keyboard should start visible, a sensible default
     * for touch oriented embeds.
     */
    keyboard?: boolean;
    /**
     * If the debug panels should start visible.
     */
    debug?: boolean;
    /**
     * If information should be logged in verbose mode, implied by
     * {@link EmbedOptions.debug}.
     */
    verbose?: boolean;
    /**
     * If the informative panels (eg: ROM name, framerate) should be shown,
     * defaults to `false` for embeds to keep the surface minimal.
     */
    info?: boolean;
    /**
     * The prefix used to namespace the `localStorage` keys, set it to an
     * empty string to share the storage with the standalone front-end.
     */
    storagePrefix?: string;
    /**
     * Controls the loading of the emulator stylesheet, set it to `false`
     * in case the host page imports the CSS itself (eg: through a
     * bundler) or to a string to load it from a custom location.
     */
    styles?: boolean | string;
    /**
     * If the page level side effects of the emulator UI (namely the
     * painting of the document's body with the background color) should
     * be contained within the emulator's own container.
     *
     * Should only be disabled when the emulator owns the complete page.
     */
    contain?: boolean;
};

/**
 * The handle returned by the {@link embed} function, provides basic
 * lifecycle control over an embedded emulator instance.
 */
export type EmbedHandle = {
    /**
     * The underlying emulator instance, exposes the complete API for
     * the more advanced integration use cases.
     */
    emulator: GameboyEmulator;
    /**
     * The element the emulator has been mounted into.
     */
    container: HTMLElement;
    /**
     * Suspends the emulation, no cycles are executed while paused.
     */
    pause: () => Promise<void>;
    /**
     * Resumes a previously paused emulation.
     */
    resume: () => Promise<void>;
    /**
     * Loads a new ROM from a remote URL, replacing the one currently
     * running, the remote origin must support CORS.
     */
    loadRom: (romUrl: string) => Promise<void>;
    /**
     * Pauses the emulation and removes the emulator from the DOM.
     */
    destroy: () => void;
};

/**
 * Keeps the emulator background color from leaking into the host page.
 *
 * The underlying UI toolkit paints the document's body with the current
 * background color, which is acceptable for the standalone front-end but
 * not for an embed, so the original value is continuously restored and
 * the color is applied to the emulator's own container instead.
 *
 * Note that the color is taken from the emulator's own background
 * notification and never read back from the body, otherwise multiple
 * embeds in the same page would race with each other over it.
 *
 * @param container The element the background color is redirected to.
 * @param emulator The emulator instance that owns the container.
 * @returns The function that stops the containment.
 */
const containBackground = (
    container: HTMLElement,
    emulator: GameboyEmulator
): (() => void) => {
    const body = document.body;
    const original = body.style.backgroundColor;

    // redirects the background notification of this specific instance
    // into its own container, preserving the original behaviour
    const onBackground = emulator.onBackground.bind(emulator);
    emulator.onBackground = (background: string) => {
        onBackground(background);
        container.style.backgroundColor = `#${background}`;
    };

    const observer = new MutationObserver(() => {
        if (body.style.backgroundColor === original) return;
        body.style.backgroundColor = original;
    });
    observer.observe(body, { attributes: true, attributeFilter: ["style"] });

    return () => observer.disconnect();
};

/**
 * Resolves the provided target into a concrete DOM element, accepting
 * either a CSS selector or an already resolved element.
 */
const resolveTarget = (target: string | HTMLElement): HTMLElement => {
    const element =
        typeof target === "string" ? document.querySelector(target) : target;
    if (!element) {
        throw new Error(`Unable to find the target element (${target})`);
    }
    return element as HTMLElement;
};

/**
 * Creates a Game Boy emulator inside of the provided target element.
 *
 * This is the entry point meant to be used by external websites, it
 * takes care of the complete setup process (WASM loading, UI mounting
 * and ROM booting) and resolves once the emulator is running.
 *
 * @param target The element (or CSS selector) the emulator is going
 * to be mounted into.
 * @param options The options that control the initial state of the
 * emulator.
 * @returns The handle that allows basic control over the emulator.
 */
export const embed = async (
    target: string | HTMLElement,
    options: EmbedOptions = {}
): Promise<EmbedHandle> => {
    const {
        romUrl,
        playlistUrl,
        palette,
        background,
        backgrounds = BACKGROUNDS,
        fullscreen = false,
        keyboard = false,
        debug = false,
        verbose = false,
        info = false,
        storagePrefix = EMBED_STORAGE_PREFIX,
        styles = true,
        contain = true
    } = options;

    const container = resolveTarget(target);

    if (styles !== false) {
        ensureStyles(typeof styles === "string" ? styles : undefined);
    }

    // creates the mount node with a generated identifier, required
    // as the underlying EmuKit app is mounted by element id and the
    // host page is not expected to provide one
    const mount = document.createElement("div");
    mount.id = `boytacean-${sequence++}`;
    container.appendChild(mount);

    // the containment is what distinguishes an embed from the standalone
    // front-end, so it also drives the layout scoping of the mount node
    if (contain) mount.classList.add(MOUNT_CLASS);

    const emulator = new GameboyEmulator(
        { background: background, debug: debug || verbose },
        { storagePrefix: storagePrefix }
    );

    const release = contain ? containBackground(container, emulator) : () => {};

    if (playlistUrl) {
        emulator.playlistUrl = playlistUrl;
        await emulator.loadPlaylist();
    }

    await emulator.init();

    startApp(mount.id, {
        emulator: emulator,
        fullscreen: fullscreen,
        info: info,
        debug: debug,
        keyboard: keyboard,
        sections: playlistUrl ? ["Playlist"] : [],
        palette: palette,
        background: background,
        backgrounds: backgrounds
    });

    // exposes the emulator globally in case no other instance has
    // claimed the slot yet, keeping the WASM global callbacks working
    // for the (most common) single embed scenario
    if (!window.emulator) window.emulator = emulator;

    // starts the emulator waiting for the booted event instead of the
    // start promise, as the latter only settles once the emulation loop
    // terminates, meaning that it would never resolve for a healthy run
    await new Promise<void>((resolve, reject) => {
        const onBooted = () => {
            emulator.unbind("booted", onBooted);
            resolve();
        };
        emulator.bind("booted", onBooted);
        emulator
            .start({ romUrl: romUrl ?? emulator.defaultRomUrl ?? undefined })
            .catch(reject);
    });

    return {
        emulator: emulator,
        container: container,
        pause: async () => await emulator.pause(),
        resume: async () => await emulator.resume(),
        loadRom: async (romUrl: string) =>
            await emulator.boot({ loadRom: true, romPath: romUrl }),
        destroy: () => {
            emulator.pause();
            release();
            mount.remove();
        }
    };
};
