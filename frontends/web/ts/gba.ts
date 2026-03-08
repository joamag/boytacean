import {
    AudioSpecs,
    base64ToBuffer,
    BenchmarkResult,
    bufferToBase64,
    Compilation,
    Compiler,
    DebugPanel,
    Emulator,
    EmulatorLogic,
    Entry,
    Feature,
    Frequency,
    FrequencySpecs,
    HelpPanel,
    PixelFormat,
    RomInfo,
    SaveState,
    SectionInfo,
    Size,
    TickParams,
    Validation
} from "emukit";
import { loadAsync } from "jszip";

import {
    default as _wasm,
    GameBoyAdvance,
    GbaClockFrame,
    GbaRomInfo,
    Info,
    PadKey
} from "../lib/boytacean";
import info from "../package.json";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const process: any;

/**
 * The frequency at which the GBA emulator should
 * run "normally".
 */
const LOGIC_HZ = 16777216;

/**
 * The frequency at witch the the visual loop is going to
 * run, increasing this value will have a consequence in
 * the visual frames per second (FPS) of emulation.
 */
const VISUAL_HZ = 59.7275;

/**
 * The rate (in seconds) at which the battery backed RAM
 * is going to be flushed to the `localStorage`.
 */
const STORE_RATE = 5;

const DISPLAY_WIDTH = 240;
const DISPLAY_HEIGHT = 160;
const DISPLAY_SCALE = 2;

const KEYS_NAME: Record<string, number> = {
    ArrowUp: PadKey.Up,
    ArrowDown: PadKey.Down,
    ArrowLeft: PadKey.Left,
    ArrowRight: PadKey.Right,
    Start: PadKey.Start,
    Select: PadKey.Select,
    A: PadKey.A,
    B: PadKey.B,
    L: PadKey.L,
    R: PadKey.R
};

/**
 * Top level class that controls the GBA emulator behaviour
 * and "joins" all the elements together to bring input/output
 * of the associated machine.
 */
export class GbaEmulator extends EmulatorLogic implements Emulator {
    /**
     * The GBA engine (probably coming from WASM) that
     * is going to be used for the emulation.
     */
    private gba: GameBoyAdvance | null = null;

    /**
     * The descriptive name of the engine that is currently
     * in use to emulate the system.
     */
    private _engine: string | null = null;

    protected logicFrequency = LOGIC_HZ;
    protected visualFrequency = VISUAL_HZ;

    /**
     * Number of pending CPU cycles from the previous tick.
     * This is used to keep track of the overflow cycles.
     */
    private pending = 0;

    /**
     * The frequency at which the battery backed RAM is going
     * to be flushed to the `localStorage`.
     */
    private flushCycles: number = LOGIC_HZ * STORE_RATE;

    private romName: string | null = null;
    private romData: Uint8Array | null = null;
    private romSize = 0;
    private _romInfo: GbaRomInfo | null = null;

    /**
     * Associative map for extra settings to be used in
     * opaque local storage operations, associated setting
     * name with its value as a string.
     */
    private extraSettings: Record<string, string | boolean> = {};

    /**
     * Current frame structure used in the clocking operations
     * of the emulator, allowing deferred frame buffer retrieval.
     */
    private clockFrame: GbaClockFrame | null = null;

    constructor(extraSettings = {}) {
        super();
        this.extraSettings = extraSettings;
    }

    /**
     * Initializes the global module structures.
     */
    async init() {
        // initializes the WASM module, this is required
        // so that the global symbols become available
        await wasm();
    }

    /**
     * Runs a tick operation in the current emulator, this operation should
     * be triggered at a regular interval to ensure that the emulator is
     * properly updated.
     *
     * Not necessarily executed once per frame, but rather once per logic
     * emulator unit.
     *
     * The tick operation is responsible for the following operations:
     * - Clocks the system by the target number of cycles.
     * - Triggers the frame event in case there's a frame to be processed.
     * - Triggers the audio event, allowing the deferred retrieval of the audio buffer.
     *
     * @params params The parameters to be used in the tick operation.
     */
    async tick(params: TickParams) {
        // in case the reference to the system is not set then
        // returns the control flow immediately (not possible to tick)
        if (!this.gba) return;

        // calculates the target cycles for clocking in the current
        // tick operation, this is the ideal value and the concrete
        // execution should not match this value
        const targetCycles = params.cycles - this.pending;

        // clocks the system by the target number of cycles (deducted
        // by the carryover cycles) and then in case there's at least
        // a frame to be processed triggers the frame event, allowing
        // the deferred retrieval of the frame buffer
        this.clockFrame = this.gba.clocks_frame_buffer(
            Math.max(targetCycles, 0)
        );
        const executedCycles = Number(this.clockFrame.cycles);
        if (this.clockFrame.frames > 0) {
            this.trigger("frame", { count: this.clockFrame.frames });
        }

        // triggers the audio event, meaning that the audio should be
        // processed for the current emulator, effectively emptying
        // the audio buffer that is pending processing
        this.trigger("audio");

        // in case the current cartridge is battery backed
        // then we need to check if a RAM flush to local
        // storage operation is required
        if (this.gba.has_battery()) {
            this.flushCycles -= executedCycles;
            if (this.flushCycles <= 0) {
                this.saveRam();
                this.flushCycles = this.logicFrequency * STORE_RATE;
            }
        }

        // triggers the tick event, indicating that a new tick
        // operation has been performed and providing some information
        // about the number of cycles that have been executed
        this.trigger("tick", { cycles: executedCycles });

        // calculates the new number of pending (overflow) cycles
        // that are going to be added to the next iteration
        this.pending = executedCycles - targetCycles;
    }

    async hardReset() {
        await wasm(false);
        await this.boot({
            engine: this._engine || "gba",
            restore: false,
            reuse: false
        });
    }

    /**
     * Starts the current machine, setting the internal structure in
     * a proper state to start drawing and receiving input.
     *
     * This method can also be used to load a new ROM into the machine.
     *
     * @param options The options that are going to be used in the
     * starting of the machine, includes information on the ROM and
     * the emulator engine to use.
     */
    async boot({
        engine = "gba",
        restore = true,
        reuse = true,
        loadRom = false,
        romPath = null,
        romName = null,
        romData = null
    }: {
        engine?: string | null;
        restore?: boolean;
        reuse?: boolean;
        loadRom?: boolean;
        romPath?: string | null;
        romName?: string | null;
        romData?: Uint8Array | null;
    } = {}) {
        // in case a remote ROM loading operation has been
        // requested then loads it from the remote origin
        if (loadRom && romPath) {
            ({ name: romName, data: romData } =
                await GbaEmulator.fetchRom(romPath));
        } else if (romName === null || romData === null) {
            [romName, romData] = [this.romName, this.romData];
        }

        // in case either the ROM's name or data is not available
        // returns silently, the emulator will wait for a ROM to
        // be loaded via the file uploader
        if (!romName || !romData) {
            return;
        }

        // checks if the current operation is a create operation
        // meaning that a new emulator instance is being created
        const isCreate = !(this.gba && reuse);

        // builds a new instance of the GBA emulator
        this.gba = isCreate
            ? new GameBoyAdvance()
            : (this.gba as GameBoyAdvance);

        // prints some debug information about the emulator that
        // has just been booted, this should provide some insights
        if (isCreate) {
            this.logger.info(
                `Creating Boytacean GBA emulator (${engine ?? "current"})...`
            );
            this.logger.info(`${this.gba.description(9)}`);
        } else {
            this.logger.info(
                `Resetting Boytacean GBA emulator (${engine ?? "current"})...`
            );
        }

        // resets the GBA engine to restore it into
        // a valid state ready to be used
        this.gba.reset();

        // loads the ROM file into the system and retrieves
        // the ROM info instance associated with it
        const cartridge = this.gba.load_rom_wa(romData);

        // prints some debug information about the cartridge that
        // has just been loaded, this should provide some insights
        this.logger.info(`${cartridge.description(9)}`);

        // updates the name of the currently selected engine
        // to the one that has been provided (logic change)
        if (engine) this._engine = engine;

        // updates the ROM name in case there's extra information
        // coming from the ROM info
        romName = cartridge.title() ? cartridge.title() : romName;

        // updates the complete set of global information that
        // is going to be displayed
        this.setRom(romName, romData, cartridge);

        // in case there's a battery involved tries to load the
        // current RAM from the local storage
        if (this.gba.has_battery()) {
            this.loadRam();
        }

        // in case the restore (state) flag is set
        // then resumes the machine execution
        if (restore) {
            await this.resume();
        }

        // triggers the booted event indicating that the
        // emulator has finished the loading process
        this.trigger("booted");
    }

    setRom(name: string, data: Uint8Array, romInfo: GbaRomInfo) {
        this.romName = name;
        this.romData = data;
        this.romSize = data.length;
        this._romInfo = romInfo;
    }

    get instance(): GameBoyAdvance | null {
        return this.gba;
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
                Feature.Help,
                Feature.Debug,
                Feature.Themes,
                Feature.Benchmark,
                Feature.Keyboard,
                Feature.KeyboardGB,
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

    get sections(): SectionInfo[] {
        return [];
    }

    get help(): HelpPanel[] {
        return [];
    }

    get debug(): DebugPanel[] {
        return [];
    }

    get engines(): string[] {
        return ["gba"];
    }

    get engine(): string {
        return this._engine || "gba";
    }

    get romExts(): string[] {
        return ["gba", "zip"];
    }

    get stateExts(): string[] {
        return [];
    }

    get pixelFormat(): PixelFormat {
        return PixelFormat.RGB;
    }

    get dimensions(): Size {
        return {
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            scale: DISPLAY_SCALE
        };
    }

    /**
     * Returns the array buffer that contains the complete set of
     * pixel data that is going to be drawn.
     *
     * @returns The current pixel data for the emulator display.
     */
    get imageBuffer(): Uint8Array {
        return (
            this.clockFrame?.frame_buffer_eager() ??
            this.gba?.frame_buffer_eager() ??
            new Uint8Array()
        );
    }

    get audioSpecs(): AudioSpecs {
        return {
            samplingRate: this.gba?.audio_sampling_rate() ?? 32768,
            channels: this.gba?.audio_channels() ?? 2
        };
    }

    get audioBuffer(): Float32Array[] {
        const internalBuffer = this.gba?.audio_buffer_eager(true) ?? [];
        const leftStream = new Float32Array(internalBuffer.length / 2);
        const rightStream = new Float32Array(internalBuffer.length / 2);
        for (let index = 0; index < internalBuffer.length; index += 2) {
            leftStream[index / 2] = internalBuffer[index] / 32768.0;
            rightStream[index / 2] = internalBuffer[index + 1] / 32768.0;
        }
        return [leftStream, rightStream];
    }

    get romInfo(): RomInfo {
        return {
            name: this.romName ?? undefined,
            data: this.romData ?? undefined,
            size: this.romSize,
            extra: {}
        };
    }

    get frequency(): number {
        return this.logicFrequency;
    }

    set frequency(value: number) {
        value = Math.max(value, 0);
        this.logicFrequency = value;
        this.trigger("frequency", value);
    }

    get displayFrequency(): number {
        return this.visualFrequency;
    }

    set displayFrequency(value: number) {
        value = Math.max(value, 0);
        this.visualFrequency = value;
        this.trigger("display-frequency", value);
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

    get registers(): Record<string, string | number> {
        return {};
    }

    keyPress(key: string) {
        const keyCode = KEYS_NAME[key];
        if (keyCode === undefined) return;
        this.gba?.key_press(keyCode);
    }

    keyLift(key: string) {
        const keyCode = KEYS_NAME[key];
        if (keyCode === undefined) return;
        this.gba?.key_lift(keyCode);
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

    async serializeState(): Promise<Uint8Array> {
        throw new Error("Save states not supported for GBA");
    }

    async unserializeState(_data: Uint8Array) {
        throw new Error("Save states not supported for GBA");
    }

    async buildState(_index: number, _data: Uint8Array): Promise<SaveState> {
        throw new Error("Save states not supported for GBA");
    }

    async validateState(_data: Uint8Array, _validation: Validation) {
        throw new Error("Save states not supported for GBA");
    }

    pauseVideo() {
        this.gba?.set_ppu_enabled(false);
    }

    resumeVideo() {
        this.gba?.set_ppu_enabled(true);
    }

    getVideoState(): boolean {
        return this.gba?.ppu_enabled() ?? false;
    }

    pauseAudio() {
        this.gba?.set_apu_enabled(false);
        this.trigger("audio-state", { state: "paused", stateBool: false });
    }

    resumeAudio() {
        this.gba?.set_apu_enabled(true);
        this.trigger("audio-state", { state: "resumed", stateBool: true });
    }

    getAudioState(): boolean {
        return this.gba?.apu_enabled() ?? false;
    }

    benchmark(count = 50000000): BenchmarkResult {
        let cycles = 0;
        this.pause();
        try {
            const initial = EmulatorLogic.now();
            for (let i = 0; i < count; i++) {
                cycles += this.gba?.clock() ?? 0;
            }
            const delta = (EmulatorLogic.now() - initial) / 1000;
            const frequency_mhz = cycles / delta / 1000 / 1000;
            return {
                delta: delta,
                count: count,
                cycles: cycles,
                frequency_mhz: frequency_mhz
            };
        } finally {
            this.resume();
        }
    }

    onBackground(background: string) {
        this.extraSettings.background = background;
        this.storeSettings();
    }

    /**
     * Tries for save/flush the current machine RAM into the
     * `localStorage`, so that it can be latter restored.
     */
    private saveRam() {
        if (!this.gba || !window.localStorage) return;
        if (!this.gba.has_battery()) return;
        const title = this.gba.rom_title();
        const ramData = this.gba.ram_data_eager();
        const ramDataB64 = bufferToBase64(ramData);
        localStorage.setItem(title, ramDataB64);
    }

    /**
     * Tries to load game RAM from the `localStorage` using the
     * current ROM title as the name of the item and
     * decoding it using Base64.
     */
    private loadRam() {
        if (!this.gba || !window.localStorage) return;
        const ramDataB64 = localStorage.getItem(this.gba.rom_title());
        if (!ramDataB64) return;
        const ramData = base64ToBuffer(ramDataB64);
        this.gba.set_ram_data(ramData);
    }

    private storeSettings() {
        if (!window.localStorage) return;
        const settings = {
            ...this.extraSettings
        };
        localStorage.setItem("settings", JSON.stringify(settings));
    }

    private static async fetchRom(
        romPath: string
    ): Promise<{ name: string; data: Uint8Array }> {
        // extracts the name of the ROM from the provided
        // path by splitting its structure
        const romPathS = romPath.split(/\//g);
        let romName = romPathS[romPathS.length - 1].split("?")[0];
        const romNameS = romName.split(/\./g);
        romName = `${romNameS[0]}.${romNameS[romNameS.length - 1]}`;

        // loads the ROM data and converts it into the
        // target byte array buffer (to be used by WASM)
        const response = await fetch(romPath);
        const blob = await response.blob();
        const arrayBuffer = await blob.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);

        // returns both the name of the ROM and the data
        // contents as a byte array
        return {
            name: romName,
            data: romData
        };
    }
}

declare global {
    interface Window {
        gbaEmulator: GbaEmulator;
    }
}

const wasm = async (setHook = true) => {
    // waits for the WASM module to be (hard) re-loaded
    // this should be an expensive operation, uses fallback
    // logic to determine if the new set of arguments for
    // wasm-bindgen should be used
    try {
        await _wasm({ module_or_path: require("../lib/boytacean_bg.wasm") });
    } catch (err) {
        if (err instanceof TypeError) {
            await _wasm();
        } else {
            throw err;
        }
    }

    // in case the set hook flag is set, then tries to
    // set the panic hook for the WASM module, this call
    // may fail in some versions of wasm-bindgen as the
    // thread is still marked as "panicking", so we need to
    // wrap the call around try/catch
    if (setHook) {
        try {
            GameBoyAdvance.set_panic_hook_wa();
        } catch (err) {
            console.error(err);
        }
    }
};
