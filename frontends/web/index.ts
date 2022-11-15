import { startApp } from "./react/app";
import { GameboyEmulator } from "./gb";

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
    // parses the current location URL as retrieves
    // some of the "relevant" GET parameters for logic
    const params = new URLSearchParams(window.location.search);
    const romUrl = params.get("rom_url") ?? params.get("url") ?? undefined;
    const fullscreen = ["1", "true", "True"].includes(
        params.get("fullscreen") ?? ""
    );
    const debug = ["1", "true", "True"].includes(params.get("debug") ?? "");
    const keyboard = ["1", "true", "True"].includes(
        params.get("keyboard") ?? ""
    );
    const palette = params.get("palette") ?? params.get("palette") ?? undefined;

    // creates the emulator structure and initializes the
    // React app with both the parameters and the emulator
    const emulator = new GameboyEmulator();
    startApp("app", {
        emulator: emulator,
        fullscreen: fullscreen,
        debug: debug,
        keyboard: keyboard,
        palette: palette,
        backgrounds: BACKGROUNDS
    });
    await emulator.main({ romUrl: romUrl });
})();
