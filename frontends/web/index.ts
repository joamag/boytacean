import { BACKGROUNDS, embed } from "./ts";

(async () => {
    // tries to load the settings from the local storage
    // to be used as fallback process for GET parameters
    let settings: Record<string, string> = {};
    if (window.localStorage) {
        settings = JSON.parse(localStorage.getItem("settings") ?? "{}");
    }

    // parses the current location URL as retrieves
    // some of the "relevant" GET parameters for logic
    const params = new URLSearchParams(window.location.search);
    const romUrl = params.get("rom_url") ?? params.get("url") ?? undefined;
    const fullscreen = ["1", "true", "True"].includes(
        params.get("fullscreen") ?? params.get("fs") ?? ""
    );
    const debug = ["1", "true", "True"].includes(params.get("debug") ?? "");
    const verbose = ["1", "true", "True"].includes(params.get("verbose") ?? "");
    const keyboard = ["1", "true", "True"].includes(
        params.get("keyboard") ?? ""
    );
    const palette = params.get("palette") ?? settings["palette"] ?? undefined;
    const background =
        params.get("background") ??
        params.get("theme") ??
        settings["background"] ??
        settings["theme"] ??
        undefined;
    const playlistUrl =
        params.get("playlist_url") ?? params.get("playlist") ?? undefined;

    // creates the emulator and mounts it in the app element, sharing the
    // same code path used by the external embeds, note that the storage
    // prefix is kept empty so that the saved games of the standalone
    // front-end remain compatible with the previous versions
    const { emulator } = await embed("#app", {
        romUrl: romUrl,
        playlistUrl: playlistUrl,
        palette: palette,
        background: background,
        backgrounds: BACKGROUNDS,
        fullscreen: fullscreen,
        keyboard: keyboard,
        debug: debug,
        verbose: verbose,
        info: !playlistUrl,
        storagePrefix: "",
        styles: false,
        contain: false
    });

    // sets the emulator in the global scope this is useful
    // to be able to access the emulator from global functions
    window.emulator = emulator;
})();
