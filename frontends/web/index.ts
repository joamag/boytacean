import { startApp } from "emukit";
import { GameboyEmulator } from "./ts";

const BACKGROUNDS = [
    "264653",
    "1b1a17",
    "023047",
    "bc6c25",
    "283618",
    "2a9d8f",
    "3a5a40"
];

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

    // creates the emulator structure and initializes the
    // React app with both the parameters and the emulator
    const emulator = new GameboyEmulator();
    startApp("app", {
        emulator: emulator,
        fullscreen: fullscreen,
        debug: debug,
        keyboard: keyboard,
        palette: palette,
        background: background,
        backgrounds: BACKGROUNDS
    });
    await emulator.main({ romUrl: romUrl });
})();
