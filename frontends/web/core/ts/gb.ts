import {
    default as _wasm,
    Cartridge,
    ClockFrame,
    GameBoy,
    GameBoyMode,
    GameBoySpeed,
    PadKey,
    SaveStateFormat,
    StateManager
} from "boytacean";
import {
    base64ToBuffer,
    BenchmarkResult,
    bufferToBase64,
    EmulatorLogic,
    PixelFormat,
    SaveState,
    Size,
    TickParams,
    Validation
} from "emukit/logic";

import { PALETTES, PALETTES_MAP } from "./palettes";
import { defaultStorage, StorageAdapter } from "./storage";

/**
 * The frequency at which the Game Boy emulator should
 * run "normally".
 */
export const LOGIC_HZ = 4194304;

/**
 * The frequency at witch the the visual loop is going to
 * run, increasing this value will have a consequence in
 * the visual frames per second (FPS) of emulation.
 */
export const VISUAL_HZ = 59.7275;

export const DISPLAY_WIDTH = 160;
export const DISPLAY_HEIGHT = 144;
export const DISPLAY_SCALE = 2;

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
    B: PadKey.B
};

/**
 * Enumeration with the values for the complete set of available
 * serial devices that can be used in the emulator.
 */
export enum SerialDevice {
    Null = "null",
    Logger = "logger",
    Printer = "printer"
}

/**
 * The set of options that can be used to control the way the
 * core is initialized and the way it interacts with the
 * surrounding environment.
 */
export type CoreOptions = {
    /**
     * The path (or module) from which the WASM binary of the
     * emulator is going to be loaded, in case it's not provided
     * the default wasm-bindgen resolution is used instead.
     */
    wasmPath?: string;

    /**
     * The storage adapter to be used in the persistence of both
     * the battery backed RAM and the settings, defaults to the
     * Web Storage API whenever it's available.
     */
    storage?: StorageAdapter;

    /**
     * Associative map for extra settings to be used in
     * opaque storage operations, associated setting name
     * with its value as a string.
     */
    extraSettings?: Record<string, string | boolean>;
};

/**
 * The core instance that is currently receiving the callbacks
 * triggered from the WASM side.
 *
 * The WASM module binds its callbacks to the `window` namespace,
 * which means that only a single core instance can be active at
 * any given time.
 */
let activeCore: GameBoyCore | null = null;

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
const setActiveCore = (core: GameBoyCore) => {
    // warns about the takeover of the callbacks, as the previously
    // active core stops receiving the speed switch, serial, printer
    // and rumble events from that moment on, only a core that has
    // already loaded a ROM is considered, so that the sequential
    // creation of cores is not reported as a conflict
    if (activeCore && activeCore !== core && activeCore.loaded) {
        core.logger.warn(
            "Multiple Game Boy cores detected, only the last one created receives the WASM callbacks"
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
export const releaseCore = (core: GameBoyCore) => {
    if (activeCore !== core) return;
    activeCore = null;
};

/**
 * Top level class that controls the emulation of the Game Boy
 * system, joining all the elements together to bring input/output
 * of the associated machine.
 *
 * This structure is completely headless, meaning that it does not
 * have any kind of dependency on a UI framework and can be used
 * both in the browser and in a Node.js environment.
 */
export class GameBoyCore extends EmulatorLogic {
    /**
     * The Game Boy engine (probably coming from WASM) that
     * is going to be used for the emulation.
     */
    protected gameBoy: GameBoy | null = null;

    /**
     * The descriptive name of the engine that is currently
     * in use to emulate the system.
     */
    protected _engine: string | null = null;

    /**
     * If the GB running mode should be automatically inferred
     * from the GBC flag in the cartridge. Meaning that if the
     * cartridge is a GBC compatible or GBC only the GBC emulation
     * mode is going to be used, otherwise the DMG mode is used
     * instead. This should provide an optimal usage experience.
     */
    protected autoMode = false;

    protected logicFrequency = LOGIC_HZ;
    protected visualFrequency = VISUAL_HZ;

    protected paletteIndex = 0;

    /**
     * Number of pending CPU cycles from the previous tick.
     * This is used to keep track of the overflow cycles.
     */
    private pending = 0;

    /**
     * The frequency at which the battery backed RAM is going
     * to be flushed to the storage adapter.
     */
    private flushCycles: number = LOGIC_HZ * STORE_RATE;

    protected romName: string | null = null;
    protected romData: Uint8Array | null = null;
    protected romSize = 0;
    protected cartridge: Cartridge | null = null;

    private _serialDevice: SerialDevice = SerialDevice.Null;

    /**
     * The path from which the WASM binary is going to be loaded,
     * kept around so that hard resets can reload the module.
     */
    private wasmPath: string | null = null;

    /**
     * The storage adapter used in the persistence of both the
     * battery backed RAM and the emulator settings.
     */
    private storage: StorageAdapter;

    /**
     * Associative map for extra settings to be used in
     * opaque storage operations, associated setting
     * name with its value as a string.
     */
    protected extraSettings: Record<string, string | boolean> = {};

    /**
     * Current frame structure used in the clocking operations
     * of the emulator, allowing deferred frame buffer retrieval.
     */
    private clockFrame: ClockFrame | null = null;

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
        if (!this.gameBoy) return;

        // uses the Game Boy multiplier to re-calculate the number
        // of cycles to be used for the tick
        const tickCycles = params.cycles * (this.gameBoy?.multiplier() ?? 1);

        // calculates the target cycles for clocking in the current
        // tick operation, this is the ideal value and the concrete
        // execution should not match this value
        const targetCycles = tickCycles - this.pending;

        // clocks the system by the target number of cycles (deducted
        // by the carryover cycles) and then in case there's at least
        // a frame to be processed triggers the frame event, allowing
        // the deferred retrieval of the frame buffer
        this.clockFrame = this.gameBoy.clocks_frame_buffer(
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
        if (this.cartridge && this.cartridge.has_battery()) {
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
        this.gameBoy = null;
        this.cartridge = null;
        this.clockFrame = null;

        await this.wasm(false);
        await this.boot({
            engine: this._engine || "auto",
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
        engine = "auto",
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
                await GameBoyCore.fetchRom(romPath));
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
        const isCreate = !(this.gameBoy && reuse);

        // selects the proper engine for execution
        // and builds a new instance of it
        switch (engine) {
            case "auto":
                this.gameBoy = isCreate
                    ? new GameBoy(GameBoyMode.Dmg)
                    : (this.gameBoy as GameBoy);
                this.gameBoy.set_mode(GameBoyMode.Dmg);
                this.autoMode = true;
                break;
            case "cgb":
                this.gameBoy = isCreate
                    ? new GameBoy(GameBoyMode.Cgb)
                    : (this.gameBoy as GameBoy);
                this.gameBoy.set_mode(GameBoyMode.Cgb);
                this.autoMode = false;
                break;
            case "dmg":
                this.gameBoy = isCreate
                    ? new GameBoy(GameBoyMode.Dmg)
                    : (this.gameBoy as GameBoy);
                this.gameBoy.set_mode(GameBoyMode.Dmg);
                this.autoMode = false;
                break;
            default:
                if (!this.gameBoy) {
                    throw new Error("No engine requested");
                }
                break;
        }

        // runs the initial palette update operation, restoring
        // the palette of the emulator according to the currently
        // selected one
        this.updatePalette();

        // in case the auto emulation mode is enabled runs the
        // inference logic to try to infer the best mode from the
        // GBC header in the cartridge data
        if (this.autoMode) {
            this.gameBoy.infer_mode_wa(romData);
        }

        // prints some debug information about the emulator that
        // has just been booted, this should provide some insights
        if (isCreate) {
            this.logger.info(
                `Creating Boytacean emulator (${engine ?? "current"})...`
            );
            this.logger.info(`${this.gameBoy.description(9)}`);
        } else {
            this.logger.info(
                `Resetting Boytacean emulator (${engine ?? "current"})...`
            );
        }

        // resets the Game Boy engine to restore it into
        // a valid state ready to be used
        this.gameBoy.reset();
        this.gameBoy.load_unsafe(true);

        // loads the ROM file into the system and retrieves
        // the cartridge instance associated with it
        const cartridge = this.gameBoy.load_rom_wa(romData);

        // prints some debug information about the cartridge that
        // has just been loaded, this should provide some insights
        this.logger.info(`${cartridge.description(9)}`);

        // loads the callbacks so that the Typescript code
        // gets notified about the various events triggered
        // in the WASM side
        this.gameBoy.load_callbacks_wa();

        // in case there's a serial device involved tries to load
        // it and initialize for the current Game Boy machine
        this.loadSerialDevice();

        // updates the name of the currently selected engine
        // to the one that has been provided (logic change)
        if (engine) this._engine = engine;

        // updates the ROM name in case there's extra information
        // coming from the cartridge
        romName = cartridge.title() ? cartridge.title() : romName;

        // updates the complete set of global information that
        // is going to be displayed
        this.setRom(romName, romData, cartridge);

        // in case there's a battery involved tries to load the
        // current RAM from the storage
        if (cartridge.has_battery()) {
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

    setRom(name: string, data: Uint8Array, cartridge: Cartridge) {
        this.romName = name;
        this.romData = data;
        this.romSize = data.length;
        this.cartridge = cartridge;
    }

    get instance(): GameBoy | null {
        return this.gameBoy;
    }

    get engines(): string[] {
        return ["auto", "cgb", "dmg"];
    }

    get engine(): string {
        return this._engine || "auto";
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
            this.gameBoy?.frame_buffer_eager() ??
            new Uint8Array()
        );
    }

    get audioSpecs() {
        return {
            samplingRate: this.gameBoy?.audio_sampling_rate() ?? 44100,
            channels: this.gameBoy?.audio_channels() ?? 2
        };
    }

    get audioBuffer(): Float32Array[] {
        const internalBuffer = this.gameBoy?.audio_buffer_eager(true) ?? [];
        const leftStream = new Float32Array(internalBuffer.length / 2);
        const rightStream = new Float32Array(internalBuffer.length / 2);
        for (let index = 0; index < internalBuffer.length; index += 2) {
            leftStream[index / 2] = internalBuffer[index] / 100.0;
            rightStream[index / 2] = internalBuffer[index + 1] / 100.0;
        }
        return [leftStream, rightStream];
    }

    get frequency(): number {
        return this.logicFrequency;
    }

    set frequency(value: number) {
        value = Math.max(value, 0);
        this.logicFrequency = value;
        this.gameBoy?.set_clock_freq(value);
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

    get registers(): Record<string, string | number> {
        const registers = this.gameBoy?.registers();
        if (!registers) return {};
        return {
            pc: registers.pc,
            sp: registers.sp,
            a: registers.a,
            b: registers.b,
            c: registers.c,
            d: registers.d,
            e: registers.e,
            h: registers.h,
            l: registers.l,
            scy: registers.scy,
            scx: registers.scx,
            wy: registers.wy,
            wx: registers.wx,
            ly: registers.ly,
            lyc: registers.lyc
        };
    }

    get speed(): GameBoySpeed {
        return this.gameBoy?.speed() ?? GameBoySpeed.Normal;
    }

    get audioOutput(): Record<string, number> {
        const output = this.gameBoy?.audio_all_output();
        if (!output) return {};
        return {
            master: output[0],
            ch1: output[1],
            ch2: output[2],
            ch3: output[3],
            ch4: output[4]
        };
    }

    get palette(): string | undefined {
        const paletteObj = PALETTES[this.paletteIndex];
        return paletteObj.name;
    }

    set palette(value: string | undefined) {
        if (value === undefined) return;
        const paletteObj = PALETTES_MAP[value];
        this.paletteIndex = Math.max(PALETTES.indexOf(paletteObj), 0);
        this.updatePalette();
    }

    get serialDevice(): SerialDevice {
        return this._serialDevice;
    }

    set serialDevice(value: SerialDevice) {
        this._serialDevice = value;
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

    keyPress(key: string) {
        const keyCode = KEYS_NAME[key];
        if (keyCode === undefined) return;
        this.gameBoy?.key_press(keyCode);
    }

    keyLift(key: string) {
        const keyCode = KEYS_NAME[key];
        if (keyCode === undefined) return;
        this.gameBoy?.key_lift(keyCode);
    }

    async serializeState(): Promise<Uint8Array> {
        if (!this.gameBoy) throw new Error("Unable to serialize state");
        return StateManager.save_wa(this.gameBoy);
    }

    async unserializeState(data: Uint8Array) {
        if (!this.gameBoy) throw new Error("Unable to unserialize state");
        StateManager.load_wa(data, this.gameBoy);
    }

    async buildState(index: number, data: Uint8Array): Promise<SaveState> {
        try {
            let state = null;
            const format = StateManager.format_wa(data);
            switch (format) {
                case SaveStateFormat.Bos:
                case SaveStateFormat.Bosc:
                    state = StateManager.read_bos_auto_wa(data);
                    break;
                case SaveStateFormat.Bess:
                    state = StateManager.read_bess_wa(data);
                    break;
                default:
                    throw new Error(`Invalid state format ${format}`);
            }
            const timestamp = Number(state.timestamp_wa());
            return {
                index: index,
                timestamp: timestamp > 0 ? timestamp : undefined,
                agent: state.agent_wa(),
                model: state.model_wa(),
                title: state.title_wa(),
                format: StateManager.format_str_wa(data),
                size: data.length,
                thumbnail: state.has_image_wa()
                    ? state.image_eager_wa()
                    : undefined
            };
        } catch (err) {
            return {
                index: index,
                error: err instanceof Error ? err : new Error(String(err))
            };
        }
    }

    async validateState(data: Uint8Array, validation: Validation) {
        StateManager.validate_wa(data, validation.title);
    }

    pauseVideo() {
        this.gameBoy?.set_ppu_enabled(false);
    }

    resumeVideo() {
        this.gameBoy?.set_ppu_enabled(true);
    }

    getVideoState(): boolean {
        return this.gameBoy?.ppu_enabled() ?? false;
    }

    pauseAudio() {
        this.gameBoy?.set_apu_enabled(false);
        this.trigger("audio-state", { state: "paused", stateBool: false });
    }

    resumeAudio() {
        this.gameBoy?.set_apu_enabled(true);
        this.trigger("audio-state", { state: "resumed", stateBool: true });
    }

    getAudioState(): boolean {
        return this.gameBoy?.apu_enabled() ?? false;
    }

    getTile(index: number): Uint8Array {
        return this.gameBoy?.get_tile_buffer(index) ?? new Uint8Array();
    }

    changePalette(): string {
        this.paletteIndex += 1;
        this.paletteIndex %= PALETTES.length;
        this.updatePalette();
        return PALETTES[this.paletteIndex].name;
    }

    benchmark(count = 50000000): BenchmarkResult {
        let cycles = 0;
        this.pause();
        try {
            const initial = EmulatorLogic.now();
            for (let i = 0; i < count; i++) {
                cycles += this.gameBoy?.clock() ?? 0;
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

    loadSerialDevice(device?: SerialDevice) {
        device = device ?? this.serialDevice;
        switch (device) {
            case SerialDevice.Null:
                this.loadNullDevice();
                break;

            case SerialDevice.Logger:
                this.loadLoggerDevice();
                break;

            case SerialDevice.Printer:
                this.loadPrinterDevice();
                break;
        }
    }

    loadNullDevice(set = true) {
        this.gameBoy?.load_null_wa();
        if (set) this.serialDevice = SerialDevice.Null;
    }

    loadLoggerDevice(set = true) {
        this.gameBoy?.load_logger_wa();
        if (set) this.serialDevice = SerialDevice.Logger;
    }

    loadPrinterDevice(set = true) {
        this.gameBoy?.load_printer_wa();
        if (set) this.serialDevice = SerialDevice.Printer;
    }

    onSpeedSwitch(speed: GameBoySpeed) {
        this.trigger("speed", { data: speed });
    }

    onLoggerDevice(data: Uint8Array) {
        this.trigger("logger", { data: data });
    }

    onPrinterDevice(imageBuffer: Uint8Array) {
        this.trigger("printer", { imageBuffer: imageBuffer });
    }

    /**
     * Tries for save/flush the current machine RAM into the
     * storage adapter, so that it can be latter restored.
     */
    protected saveRam() {
        if (!this.gameBoy || !this.cartridge) return;
        if (!this.cartridge.has_battery()) return;
        const title = this.cartridge.title();
        const ramData = this.gameBoy.ram_data_eager();
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
     * current cartridge title as the name of the item and
     * decoding it using Base64.
     */
    protected loadRam() {
        if (!this.gameBoy || !this.cartridge) return;
        const ramDataB64 = this.storage.getItem(this.cartridge.title());
        if (!ramDataB64) return;
        const ramData = base64ToBuffer(ramDataB64);
        this.gameBoy.set_ram_data(ramData);
    }

    protected storeSettings() {
        const settings = {
            palette: PALETTES[this.paletteIndex].name,
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

    protected updatePalette() {
        const palette = PALETTES[this.paletteIndex];
        this.gameBoy?.set_palette_colors_wa(palette.colors);
        this.storeSettings();
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
        // one that sits next to the `boytacean` package, this is
        // required as some bundlers strip the `import.meta.url` based
        // resolution that wasm-bindgen relies on by default
        const wasmPath = this.wasmPath ?? GameBoyCore.defaultWasmPath();

        // waits for the WASM module to be (hard) re-loaded
        // this should be an expensive operation, uses fallback
        // logic to determine if the new set of arguments for
        // wasm-bindgen should be used
        try {
            await (wasmPath ? _wasm({ module_or_path: wasmPath }) : _wasm());
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
                GameBoy.set_panic_hook_wa();
            } catch (err) {
                console.error(err);
            }
        }
    }

    /**
     * Resolves the default location of the WASM binary, which is the
     * one distributed next to the `boytacean` package.
     *
     * @returns The URL of the WASM binary or null in case it cannot
     * be resolved in the current environment.
     */
    protected static defaultWasmPath(): string | null {
        try {
            return new URL("../../lib/boytacean_bg.wasm", import.meta.url).href;
        } catch (err) {
            return null;
        }
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
        if (!response.ok) {
            throw new Error(
                `Unable to load ROM from ${romPath} (${response.status})`
            );
        }
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
        panic: (message: string) => void;
        speedCallback: (speed: GameBoySpeed) => void;
        loggerCallback: (data: Uint8Array) => void;
        printerCallback: (imageBuffer: Uint8Array) => void;
        rumbleCallback: (active: boolean) => void;
    }
}

if (typeof window !== "undefined") {
    window.panic = (message: string) => {
        console.error(message);
    };

    window.speedCallback = (speed: GameBoySpeed) => {
        activeCore?.onSpeedSwitch(speed);
    };

    window.loggerCallback = (data: Uint8Array) => {
        activeCore?.onLoggerDevice(data);
    };

    window.printerCallback = (imageBuffer: Uint8Array) => {
        activeCore?.onPrinterDevice(imageBuffer);
    };

    window.rumbleCallback = (active: boolean) => {
        if (!active) return;

        // runs the vibration actuator on the current window
        // this will probably affect only mobile devices
        window?.navigator?.vibrate?.(250);

        // iterates over all the available gamepads to run
        // the vibration actuator on each of them
        let gamepadIndex = 0;
        while (true) {
            const gamepad = navigator.getGamepads()[gamepadIndex];
            if (!gamepad) break;
            gamepad?.vibrationActuator?.playEffect?.("dual-rumble", {
                startDelay: 0,
                duration: 150,
                weakMagnitude: 0.8,
                strongMagnitude: 0.0
            });
            gamepadIndex++;
        }
    };
}
