import {
    Compilation,
    Compiler,
    DebugPanel,
    Emulator,
    Entry,
    Feature,
    Frequency,
    FrequencySpecs,
    HelpPanel,
    RomInfo,
    SectionInfo
} from "emukit";
import { loadAsync } from "jszip";

import { GameBoyCore } from "../core";
import { Info } from "../lib/boytacean";
import info from "../package.json";
import {
    DebugAudio,
    DebugGeneral,
    DebugSettings,
    GamePlaylist,
    HelpFaqs,
    HelpKeyboard,
    SerialSection,
    TestSection
} from "../react";

import { fetchPlaylist, Playlist } from "./playlist";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const process: any;

const ROM_PATH = require("../../../res/roms/demo/pocket.gb");

/**
 * Top level class that controls the emulator behaviour, adapting
 * the headless Game Boy core into the EmuKit emulator interface,
 * providing the UI bound information required by it.
 */
export class GameboyEmulator extends GameBoyCore implements Emulator {
    /**
     * The URL of the JSON playlist file to be loaded
     * remotely, if not set the playlist feature is disabled.
     */
    private _playlistUrl: string | null = null;

    /**
     * The cached playlist structure that has been loaded
     * from the remote URL, avoiding redundant fetches.
     */
    private _playlist: Playlist | null = null;

    constructor(extraSettings: Record<string, string | boolean> = {}) {
        super({
            wasmPath: require("../lib/boytacean_bg.wasm"),
            extraSettings: extraSettings
        });
    }

    get playlistUrl(): string | null {
        return this._playlistUrl;
    }

    set playlistUrl(value: string | null) {
        this._playlistUrl = value;
        this._playlist = null;
    }

    /**
     * The default ROM URL from the loaded playlist, if
     * available. Used to boot the initial ROM when a
     * playlist is provided.
     */
    get defaultRomUrl(): string | null {
        return this._playlist?.defaultUrl ?? null;
    }

    /**
     * Loads the playlist from the remote URL and caches
     * it for subsequent usage in the playlist section.
     */
    async loadPlaylist() {
        if (!this._playlistUrl) return;
        try {
            this._playlist = await fetchPlaylist(this._playlistUrl);
        } catch (err) {
            this.logger.error(`Failed to load playlist (${err})`);
        }
    }

    async boot(options = {}) {
        await super.boot({ romPath: ROM_PATH, ...options });
    }

    get name(): string {
        return Info.name() ?? info.name;
    }

    get device(): Entry {
        return {
            text: Info.system(),
            url: "https://en.wikipedia.org/wiki/Game_Boy"
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
                Feature.Help,
                Feature.Debug,
                Feature.Themes,
                Feature.Palettes,
                Feature.Benchmark,
                Feature.Keyboard,
                Feature.KeyboardGB,
                Feature.Framerate,
                Feature.SaveState
            ],
            ...((this.extraSettings?.debug ?? false)
                ? [
                      Feature.LoopMode,
                      Feature.DisplayFrequency,
                      Feature.BootRomInfo,
                      Feature.RomTypeInfo,
                      Feature.Cyclerate,
                      Feature.Animationrate,
                      Feature.SkippedTicks,
                      Feature.EmulationSpeed
                  ]
                : [])
        ];
    }

    get sections(): SectionInfo[] {
        const _sections: SectionInfo[] = [
            {
                name: "Serial",
                icon: require("../res/serial.svg"),
                node: SerialSection({ emulator: this })
            }
        ];
        if (this._playlistUrl) {
            _sections.push({
                name: "Playlist",
                icon: require("../res/playlist.svg"),
                node: GamePlaylist({
                    entries: this._playlist?.entries ?? [],
                    playlist: this._playlist ?? undefined,
                    emulator: this
                })
            });
        }
        if (process.env.NODE_ENV === "development") {
            _sections.push({
                name: "Test",
                node: TestSection({})
            });
        }
        return _sections;
    }

    get help(): HelpPanel[] {
        return [
            {
                name: "Keyboard",
                node: HelpKeyboard({})
            },
            {
                name: "FAQs",
                node: HelpFaqs({})
            }
        ];
    }

    get debug(): DebugPanel[] {
        return [
            {
                name: "General",
                node: DebugGeneral({ emulator: this })
            },
            {
                name: "Audio",
                node: DebugAudio({ emulator: this })
            },
            {
                name: "Settings",
                node: DebugSettings({ emulator: this })
            }
        ];
    }

    get romExts(): string[] {
        return ["gb", "gbc", "zip"];
    }

    get stateExts(): string[] {
        return ["sav", ...Array.from({ length: 10 }, (_, i) => `s${i + 1}`)];
    }

    get romInfo(): RomInfo {
        return {
            name: this.romName ?? undefined,
            data: this.romData ?? undefined,
            size: this.romSize,
            extra: {
                bootRom: this.gameBoy?.boot_rom_s(),
                romType: this.cartridge?.rom_type_s(),
                romSize: this.cartridge?.rom_size_s(),
                ramSize: this.cartridge?.ram_size_s()
            }
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
        if (!this.gameBoy) return null;
        return {
            name: Info.compiler(),
            version: Info.compiler_version()
        };
    }

    get compilation(): Compilation | null {
        if (!this.gameBoy) return null;
        return {
            date: Info.compilation_date(),
            time: Info.compilation_time()
        };
    }

    get wasmEngine(): string | null {
        if (!this.gameBoy) return null;
        return Info.wasm_engine() ?? null;
    }

    async buildRomData(file: File): Promise<Uint8Array> {
        const arrayBuffer = await file.arrayBuffer();
        let romData = new Uint8Array(arrayBuffer);

        if (file.name.endsWith(".zip")) {
            const zip = await loadAsync(romData);
            const firstFile = Object.values(zip.files)[0];
            romData = await firstFile.async("uint8array");
        }

        return romData;
    }

    /**
     * Shows the game playlist modal overlay allowing the
     * user to search and select a ROM from the playlist.
     */
    async showPlaylist() {
        if (!this._playlistUrl) return;

        // fetches the playlist from the remote URL in case
        // it has not been cached yet, avoiding redundant requests
        if (!this._playlist) {
            try {
                this._playlist = await fetchPlaylist(this._playlistUrl);
            } catch (err) {
                this.handlers.showToast?.("Failed to load the playlist!", true);
                this.logger.error(`Failed to load playlist (${err})`);
                return;
            }
        }

        // shows the playlist modal with the game entries
        // allowing the user to search and select a ROM
        await this.handlers.showModal?.(
            this._playlist.name ?? "Game Playlist",
            undefined,
            GamePlaylist({
                entries: this._playlist.entries,
                playlist: this._playlist,
                emulator: this
            })
        );
    }
}

declare global {
    interface Window {
        emulator: GameboyEmulator;
    }

    interface Console {
        image(url: string, size?: number): void;
    }
}

console.image = (url: string, size = 80) => {
    const style = `font-size: ${size}px; background-image: url("${url}"); background-size: contain; background-repeat: no-repeat;`;
    console.log("%c     ", style);
};
