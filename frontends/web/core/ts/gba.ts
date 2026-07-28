import {
    default as _wasm,
    GameBoyAdvance,
    GbaClockFrame,
    GbaRomInfo,
    PadKey
} from "boytacean";
import {
    base64ToBuffer,
    bufferToBase64,
    EmulatorLogic,
    PixelFormat,
    Size,
    TickParams
} from "emukit/logic";

import { CoreOptions } from "./gb";
import { defaultStorage, StorageAdapter } from "./storage";

/**
 * The frequency at which the Game Boy Advance emulator
 * should run "normally".
 */
export const GBA_LOGIC_HZ = 16777216;

/**
 * The frequency at witch the the visual loop is going to
 * run, increasing this value will have a consequence in
 * the visual frames per second (FPS) of emulation.
 */
export const GBA_VISUAL_HZ = 59.7275;

export const GBA_DISPLAY_WIDTH = 240;
export const GBA_DISPLAY_HEIGHT = 160;
export const GBA_DISPLAY_SCALE = 2;

/**
 * The rate at which the storage RAM state flush operation
 * is going to be performed, this value is the number of
 * seconds in between flush operations (eg: 5 seconds).
 */
const STORE_RATE = 5;

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
 * The core instance that is currently receiving the callbacks
 * triggered from the WASM side.
 *
 * The WASM module binds its callbacks to the `window` namespace,
 * which means that only a single core instance can be active at
 * any given time.
 */
let activeCore: GbaCore | null = null;

/**
 * Registers the provided core as the one that is going to receive
 * the callbacks triggered from the WASM side.
 *
 * The callbacks exposed by the WASM module are bound to the `window`
 * namespace, meaning that a single core instance can be active at
 * any given time.
 *
 * @param core The core instance to be set as the active one.
 */
const setActiveCore = (core: GbaCore) => {
    // warns about the takeover of the callbacks, as the previously
    // active core stops receiving the events from that moment on,
    // only a core that has already loaded a ROM is considered, so
    // that the sequential creation of cores is not reported as a
    // conflict
    if (activeCore && activeCore !== core && activeCore.loaded) {
        core.logger.warn(
            "Multiple Game Boy Advance cores detected, only the last one created receives the WASM callbacks"
        );
    }
    activeCore = core;
};

/**
 * Releases the provided core from the active position, so that it
 * stops receiving the callbacks triggered from the WASM side.
 *
 * A core that is not the active one is ignored, avoiding the release
 * of a core that has already been superseded by another one.
 *
 * @param core The core instance to be released.
 */
export const releaseGbaCore = (core: GbaCore) => {
    if (activeCore !== core) return;
    activeCore = null;
};

/**
 * Top level class that controls the emulation of the Game Boy
 * Advance system, joining all the elements together to bring
 * input/output of the associated machine.
 *
 * This structure is completely headless, meaning that it does not
 * have any kind of dependency on a UI framework and can be used
 * both in the browser and in a Node.js environment.
 */
export class GbaCore extends EmulatorLogic {
    /**
     * The GBA engine (probably coming from WASM) that
     * is going to be used for the emulation.
     */
    protected gba: GameBoyAdvance | null = null;

    /**
     * The descriptive name of the engine that is currently
     * in use to emulate the system.
     */
    protected _engine: string | null = null;

    protected logicFrequency = GBA_LOGIC_HZ;
    protected visualFrequency = GBA_VISUAL_HZ;

    /**
     * Number of pending CPU cycles from the previous tick.
     * This is used to keep track of the overflow cycles.
     */
    private pending = 0;

    /**
     * The frequency at which the battery backed RAM is going
     * to be flushed to the storage adapter.
     */
    private flushCycles: number = GBA_LOGIC_HZ * STORE_RATE;

    protected romName: string | null = null;
    protected romData: Uint8Array | null = null;
    protected romSize = 0;
    protected romInfoI: GbaRomInfo | null = null;

    /**
     * The path (or module) from which the WASM binary of the
     * emulator is going to be loaded.
     */
    private wasmPath: string | null = null;

    /**
     * The storage adapter that is going to be used in the
     * persistence of both the RAM and the settings.
     */
    protected storage: StorageAdapter;

    /**
     * Associative map for extra settings to be used in
     * opaque storage operations, associated setting name
     * with its value as a string.
     */
    protected extraSettings: Record<string, string | boolean> = {};

    /**
     * Current frame structure used in the clocking operations
     * of the emulator, allowing deferred frame buffer retrieval.
     */
    protected clockFrame: GbaClockFrame | null = null;

    constructor({ wasmPath, storage, extraSettings = {} }: CoreOptions = {}) {
        super();
        this.wasmPath = wasmPath ?? null;
        this.storage = storage ?? defaultStorage();
        this.extraSettings = extraSettings;
        setActiveCore(this);
    }

    /**
     * Initializes the global module structures.
     */
    async init() {
        // initializes the WASM module, this is required
        // so that the global symbols become available
        await this.wasm();
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
     * - Flushes the RAM to the storage in case the cartridge is battery backed.
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
        // then we need to check if a RAM flush to the storage
        // operation is required
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
        // releases the references to the structures of the previous
        // WASM instance, as they would otherwise be used against the
        // fresh linear memory that the reload creates
        this.gba = null;
        this.romInfoI = null;
        this.clockFrame = null;

        await this.wasm(false);
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
        romPath = undefined,
        romName = null,
        romData = null
    }: {
        engine?: string | null;
        restore?: boolean;
        reuse?: boolean;
        loadRom?: boolean;
        romPath?: string;
        romName?: string | null;
        romData?: Uint8Array | null;
    } = {}) {
        // in case a remote ROM loading operation has been
        // requested then loads it from the remote origin
        if (loadRom && romPath !== undefined) {
            ({ name: romName, data: romData } =
                await GbaCore.fetchRom(romPath));
        } else if (romName === null || romData === null) {
            [romName, romData] = [this.romName, this.romData];
        }

        // in case either the ROM's name or data is not available
        // throws an error as the boot process is not possible
        if (!romName || !romData) {
            throw new Error("Unable to load initial ROM");
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
        const romInfo = this.gba.load_rom_wa(romData);

        // prints some debug information about the cartridge that
        // has just been loaded, this should provide some insights
        this.logger.info(`${romInfo.description(9)}`);

        // updates the name of the currently selected engine
        // to the one that has been provided (logic change)
        if (engine) this._engine = engine;

        // updates the ROM name in case there's extra information
        // coming from the ROM info
        romName = romInfo.title() ? romInfo.title() : romName;

        // updates the complete set of global information that
        // is going to be displayed
        this.setRom(romName, romData, romInfo);

        // in case there's a battery involved tries to load the
        // current RAM from the storage
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
        this.romInfoI = romInfo;
    }

    get instance(): GameBoyAdvance | null {
        return this.gba;
    }

    get engines(): string[] {
        return ["gba"];
    }

    get engine(): string {
        return this._engine || "gba";
    }

    get pixelFormat(): PixelFormat {
        return PixelFormat.RGB;
    }

    get dimensions(): Size {
        return {
            width: GBA_DISPLAY_WIDTH,
            height: GBA_DISPLAY_HEIGHT,
            scale: GBA_DISPLAY_SCALE
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

    get audioSpecs() {
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

    get frequency(): number {
        return this.logicFrequency;
    }

    set frequency(value: number) {
        value = Math.max(value, 0);
        this.logicFrequency = value;
        this.trigger("frequency", value);
    }

    /**
     * The name of the ROM that is currently loaded in the machine,
     * null in case no ROM has been loaded yet.
     */
    get loadedRomName(): string | null {
        return this.romName;
    }

    /**
     * The size in bytes of the ROM that is currently loaded in the
     * machine, zero in case no ROM has been loaded yet.
     */
    get loadedRomSize(): number {
        return this.romSize;
    }

    /**
     * If a ROM has already been loaded into the machine, meaning
     * that the emulation is able to run.
     */
    get loaded(): boolean {
        return this.romData !== null;
    }

    /**
     * The header information of the ROM that is currently loaded in
     * the machine, null in case no ROM has been loaded yet.
     */
    get loadedRomInfo(): GbaRomInfo | null {
        return this.romInfoI;
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
    }

    resumeAudio() {
        this.gba?.set_apu_enabled(true);
    }

    getAudioState(): boolean {
        return this.gba?.apu_enabled() ?? false;
    }

    /**
     * Tries for save/flush the current machine RAM into the
     * storage adapter, so that it can be latter restored.
     */
    protected saveRam() {
        if (!this.gba) return;
        if (!this.gba.has_battery()) return;
        const title = this.gba.rom_title();
        const ramData = this.gba.ram_data_eager();
        const ramDataB64 = bufferToBase64(ramData);

        // the storage operation is guarded as it runs from the tick
        // operation, meaning that a failing adapter (eg: exhausted
        // quota) would otherwise break the emulation loop
        try {
            this.storage.setItem(title, ramDataB64);
        } catch (err) {
            this.logger.warn(`Failed to save RAM data (${err})`);
        }
    }

    /**
     * Tries to load game RAM from the storage adapter using the
     * current ROM title as the name of the item and decoding it
     * using Base64.
     */
    protected loadRam() {
        if (!this.gba) return;
        const ramDataB64 = this.storage.getItem(this.gba.rom_title());
        if (!ramDataB64) return;
        const ramData = base64ToBuffer(ramDataB64);
        this.gba.set_ram_data(ramData);
    }

    protected storeSettings() {
        const settings = {
            ...this.extraSettings
        };

        // guards the storage operation as this method is called as
        // part of the boot sequence, which should not be interrupted
        // by a failing storage adapter
        try {
            this.storage.setItem("settings", JSON.stringify(settings));
        } catch (err) {
            this.logger.warn(`Failed to store settings (${err})`);
        }
    }

    /**
     * Loads the WASM module of the emulator, using the path that has
     * been provided at construction time and falling back to the
     * default wasm-bindgen resolution strategy.
     *
     * @param setHook If the panic hook should be set as part of the
     * WASM module loading operation.
     */
    protected async wasm(setHook = true) {
        // resolves the path of the WASM binary, falling back to the
        // one that sits next to the `boytacean` package
        const wasmPath = this.wasmPath ?? GbaCore.defaultWasmPath();

        // waits for the WASM module to be (hard) re-loaded
        // this should be an expensive operation, uses fallback
        // logic to determine if the new set of arguments for
        // wasm-bindgen should be used
        //
        // any failure of the resolved path falls back to the default
        // resolution of wasm-bindgen, as the path is a best effort
        // guess that does not hold in every bundler layout, while the
        // default one is relative to the module of the binary itself
        try {
            await (wasmPath ? _wasm({ module_or_path: wasmPath }) : _wasm());
        } catch (err) {
            if (!wasmPath) throw err;
            await _wasm();
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
    }

    /**
     * Resolves the default location of the WASM binary, which is the
     * one distributed by the `boytacean` package.
     *
     * The binary is looked up through the `boytacean` package itself,
     * so that it's found under any `node_modules` layout, falling
     * back to the sibling `lib` directory that is only present in the
     * web front-end of the repository.
     *
     * Whenever neither of them applies null is returned, handing the
     * resolution over to wasm-bindgen, which looks the binary up next
     * to its own module and is therefore correct in the browser.
     *
     * @returns The URL of the WASM binary or null in case it cannot
     * be resolved in the current environment.
     */
    protected static defaultWasmPath(): string | null {
        // asks the module resolver for the real location of the binary,
        // this is the most reliable strategy but it's only available
        // outside of the browser (eg: Node.js), where it also survives
        // an arbitrary `node_modules` layout (hoisted, nested, ...)
        try {
            const resolve = (
                import.meta as ImportMeta & {
                    resolve?: (specifier: string) => string;
                }
            ).resolve;
            if (resolve) return resolve("boytacean/boytacean_bg.wasm");
        } catch (err) {
            // ignores the error and moves to the next strategy, as the
            // specifier may not be resolvable from this module
        }

        // falls back to the `lib` directory that sits next to the core
        // in the web front-end of the repository, where the WASM binary
        // is generated by `wasm-pack` instead of being installed
        //
        // only a file URL is considered, as that is the shape of a
        // module that runs straight from the repository, while a
        // bundled or installed module would otherwise be handed a
        // directory that does not exist in its own layout
        //
        // notice that no attempt is made to resolve the binary through
        // a bare specifier, as bundlers resolve those at build time and
        // the `boytacean` package is aliased to a local file in the web
        // front-end, the browser case is instead covered by the default
        // resolution of wasm-bindgen, which looks the binary up next to
        // its own module and is reached whenever null is returned here
        try {
            const base = import.meta.url;
            if (base.startsWith("file:") && !base.includes("/node_modules/")) {
                return new URL("../../lib/boytacean_bg.wasm", base).href;
            }
        } catch (err) {
            // ignores the error and hands the resolution over to
            // wasm-bindgen by returning null below
        }

        return null;
    }

    protected static async fetchRom(
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
