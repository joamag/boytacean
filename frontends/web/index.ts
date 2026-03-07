import { Emulator, startApp } from "emukit";

import { GameboyEmulator, GbaEmulator } from "./ts";

/**
 * List of available background theme colors that can be used
 * to style the main emulator area.
 */
const BACKGROUNDS = [
    "264653",
    "1b1a17",
    "023047",
    "bc6c25",
    "283618",
    "2a9d8f",
    "3a5a40"
];

/**
 * Checks if the given URL points to a GBA ROM by examining
 * the file extension.
 */
const isGbaUrl = (url: string): boolean => {
    const path = url.split("?")[0].toLowerCase();
    return path.endsWith(".gba");
};

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
    const gba = ["1", "true", "True"].includes(params.get("gba") ?? "");
    const palette = params.get("palette") ?? settings["palette"] ?? undefined;
    const background =
        params.get("background") ??
        params.get("theme") ??
        settings["background"] ??
        settings["theme"] ??
        undefined;

    // determines if the emulator should run in GBA mode,
    // either via the explicit ?gba=1 parameter or by
    // examining the ROM URL file extension
    const isGba = gba || (romUrl ? isGbaUrl(romUrl) : false);

    // creates the appropriate emulator structure based on the
    // detected ROM type and initializes the React app with
    // both the parameters and the emulator
    const emulator: Emulator = isGba
        ? new GbaEmulator({
              background: background,
              debug: debug || verbose
          })
        : new GameboyEmulator({
              background: background,
              debug: debug || verbose
          });
    await emulator.init();
    startApp("app", {
        emulator: emulator,
        fullscreen: fullscreen,
        debug: debug,
        keyboard: keyboard,
        palette: isGba ? undefined : palette,
        background: background,
        backgrounds: BACKGROUNDS
    });

    // sets the emulator in the global scope this is useful
    // to be able to access the emulator from global functions
    (window as unknown as Record<string, unknown>).emulator = emulator;

    // starts the emulator with the provided ROM URL, this is
    // going to run the main emulator (infinite) loop
    await emulator.start({ romUrl: romUrl });
})();
