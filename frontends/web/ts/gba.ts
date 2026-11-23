import {
    Compilation,
    Compiler,
    Emulator,
    Entry,
    Feature,
    Frequency,
    FrequencySpecs,
    RomInfo
} from "emukit";
import { loadAsync } from "jszip";

import { GbaCore } from "../core";
import { Info } from "../lib/boytacean";
import info from "../package.json";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

const ROM_PATH = require("../../../res/roms.gba/demo/geburtstag.gba");

/**
 * Top level class that controls the GBA emulator behaviour, adapting
 * the headless GBA core into the EmuKit emulator interface,
 * providing the UI bound information required by it.
 */
export class GbaEmulator extends GbaCore implements Emulator {
    constructor(extraSettings: Record<string, string | boolean> = {}) {
        super({
            wasmPath: require("../lib/boytacean_bg.wasm"),
            extraSettings: extraSettings
        });
    }

    /**
     * Starts the current machine, booting the default demo ROM
     * (GeBurtstAg) in case no other ROM is provided, as the Game
     * Boy emulator does with its own demo ROM.
     *
     * @param options The options that are going to be used in the
     * starting of the machine.
     */
    async boot(
        { romPath = ROM_PATH, ...options } = {} as {
            romPath?: string;
            romData?: Uint8Array | null;
        }
    ) {
        await super.boot({ romPath: romPath, ...options });
    }

    get name(): string {
        return Info.name() ?? info.name;
    }

    get device(): Entry {
        return {
            text: "Game Boy Advance",
            url: "https://en.wikipedia.org/wiki/Game_Boy_Advance"
        };
    }

    get icon(): string | undefined {
        return require("../res/star.png");
    }

    get version(): Entry | undefined {
        return {
            text: Info.version() ?? info.version,
            url: "https://github.com/joamag/boytacean/blob/master/CHANGELOG.md"
        };
    }

    get repository(): Entry {
        return {
            text: "GitHub",
            url: "https://github.com/joamag/boytacean"
        };
    }

    get features(): Feature[] {
        return [
            ...[
                Feature.Themes,
                Feature.Benchmark,
                Feature.Keyboard,
                Feature.KeyboardGBA,
                Feature.Framerate
            ],
            ...((this.extraSettings?.debug ?? false)
                ? [
                      Feature.LoopMode,
                      Feature.DisplayFrequency,
                      Feature.Cyclerate,
                      Feature.Animationrate,
                      Feature.SkippedTicks,
                      Feature.EmulationSpeed
                  ]
                : [])
        ];
    }

    get romExts(): string[] {
        return ["gba", "zip"];
    }

    get romInfo(): RomInfo {
        return {
            name: this.romName ?? undefined,
            data: this.romData ?? undefined,
            size: this.romSize
        };
    }

    get frequencySpecs(): FrequencySpecs {
        return {
            unit: Frequency.MHz,
            delta: 400000,
            places: 2
        };
    }

    get compiler(): Compiler | null {
        if (!this.gba) return null;
        return {
            name: Info.compiler(),
            version: Info.compiler_version()
        };
    }

    get compilation(): Compilation | null {
        if (!this.gba) return null;
        return {
            date: Info.compilation_date(),
            time: Info.compilation_time()
        };
    }

    get wasmEngine(): string | null {
        if (!this.gba) return null;
        return Info.wasm_engine() ?? null;
    }

    /**
     * The registers of the machine, empty as the GBA engine does not
     * expose them yet, required by the EmuKit emulator interface.
     */
    get registers(): Record<string, string | number> {
        return {};
    }

    async buildRomData(file: File): Promise<Uint8Array> {
        const arrayBuffer = await file.arrayBuffer();
        let romData = new Uint8Array(arrayBuffer);

        if (file.name.endsWith(".zip")) {
            const zip = await loadAsync(romData);
            const firstFile = Object.values(zip.files)[0];
            romData = new Uint8Array(await firstFile.async("uint8array"));
        }

        return romData;
    }
}
