import {
    AudioSpecs,
    base64ToBuffer,
    BenchmarkResult,
    bufferToBase64,
    Compilation,
    Compiler,
    DebugPanel,
    Emulator,
    EmulatorBase,
    Entry,
    Feature,
    Frequency,
    FrequencySpecs,
    HelpPanel,
    PixelFormat,
    RomInfo,
    SaveState,
    SectionInfo,
    Size
} from "emukit";
import { PALETTES, PALETTES_MAP } from "./palettes";
import {
    DebugAudio,
    DebugGeneral,
    DebugSettings,
    HelpFaqs,
    HelpKeyboard,
    SerialSection
} from "../react";

import {
    Cartridge,
    default as _wasm,
    GameBoy,
    GameBoyMode,
    GameBoySpeed,
    Info,
    PadKey,
    SaveStateFormat,
    StateManager
} from "../lib/boytacean";
import info from "../package.json";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

/**
 * The frequency at which the Game Boy emulator should
 * run "normally".
 */
const LOGIC_HZ = 4194304;

/**
 * The frequency at witch the the visual loop is going to
 * run, increasing this value will have a consequence in
 * the visual frames per second (FPS) of emulation.
 */
const VISUAL_HZ = 59.7275;

/**
 * The frequency of the pause polling update operation,
 * increasing this value will make resume from emulation
 * paused state fasted.
 */
const IDLE_HZ = 10;

const DISPLAY_WIDTH = 160;
const DISPLAY_HEIGHT = 144;
const DISPLAY_SCALE = 2;

/**
 * The rate at which the local storage RAM state flush
 * operation is going to be performed, this value is the
 * number of seconds in between flush operations (eg: 5 seconds).
 */
const STORE_RATE = 5;

/**
 * The sample rate that is going to be used for FPS calculus,
 * meaning that every N seconds we will calculate the number
 * of frames rendered divided by the N seconds.
 */
const FPS_SAMPLE_RATE = 3;

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

const ROM_PATH = require("../../../res/roms/demo/pocket.gb");

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
 * Top level class that controls the emulator behaviour
 * and "joins" all the elements together to bring input/output
 * of the associated machine.
 */
export class GameboyEmulator extends EmulatorBase implements Emulator {
    /**
     * The Game Boy engine (probably coming from WASM) that
     * is going to be used for the emulation.
     */
    private gameBoy: GameBoy | null = null;

    /**
     * The descriptive name of the engine that is currently
     * in use to emulate the system.
     */
    private _engine: string | null = null;

    /**
     * If the GB running mode should be automatically inferred
     * from the GBC flag in the cartridge. Meaning that if the
     * cartridge is a GBC compatible or GBC only the GBC emulation
     * mode is going to be used, otherwise the DMG mode is used
     * instead. This should provide an optimal usage experience.
     */
    private autoMode = false;

    private logicFrequency = LOGIC_HZ;
    private visualFrequency = VISUAL_HZ;
    private idleFrequency = IDLE_HZ;

    private paused = false;
    private nextTickTime = 0;
    private fps = 0;
    private frameStart: number = EmulatorBase.now();
    private frameCount = 0;
    private paletteIndex = 0;

    /**
     * The frequency at which the battery backed RAM is going
     * to be flushed to the `localStorage`.
     */
    private flushCycles: number = LOGIC_HZ * STORE_RATE;

    private romName: string | null = null;
    private romData: Uint8Array | null = null;
    private romSize = 0;
    private cartridge: Cartridge | null = null;

    private _serialDevice: SerialDevice = SerialDevice.Null;

    /**
     * Associative map for extra settings to be used in
     * opaque local storage operations, associated setting
     * name with its value as a string.
     */
    private extraSettings: Record<string, string> = {};

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
     * Runs the initialization and main loop execution for
     * the Game Boy emulator.
     * The main execution of this function should be an
     * infinite loop running machine `tick` operations.
     *
     * @param options The set of options that are going to be
     * used in he Game Boy emulator initialization.
     */
    async main({ romUrl }: { romUrl?: string }) {
        // boots the emulator subsystem with the initial
        // ROM retrieved from a remote data source
        await this.boot({ loadRom: true, romPath: romUrl ?? undefined });

        // the counter that controls the overflowing cycles
        // from tick to tick operation
        let pending = 0;

        // runs the sequence as an infinite loop, running
        // the associated CPU cycles accordingly
        while (true) {
            // in case the machine is paused we must delay the execution
            // a little bit until the paused state is recovered
            if (this.paused) {
                await new Promise((resolve) => {
                    setTimeout(resolve, 1000 / this.idleFrequency);
                });
                continue;
            }

            // obtains the current time, this value is going
            // to be used to compute the need for tick computation
            let currentTime = EmulatorBase.now();

            try {
                pending = this.tick(
                    currentTime,
                    pending,
                    Math.round(
                        (this.logicFrequency *
                            (this.gameBoy?.multiplier() ?? 1)) /
                            this.visualFrequency
                    )
                );
            } catch (err) {
                // sets the default error message to be displayed
                // to the user, this value may be overridden in case
                // a better and more explicit message can be determined
                let message = String(err);

                // verifies if the current issue is a panic one
                // and updates the message value if that's the case
                const messageNormalized = (err as Error).message.toLowerCase();
                const isPanic =
                    messageNormalized.startsWith("unreachable") ||
                    messageNormalized.startsWith("recursive use of an object");
                if (isPanic) {
                    message = "Unrecoverable error, restarting Game Boy";
                }

                // displays the error information to both the end-user
                // and the developer (for diagnostics)
                this.trigger("message", {
                    text: message,
                    error: true,
                    timeout: 5000
                });
                console.error(err);

                // pauses the machine, allowing the end-user to act
                // on the error in a proper fashion
                this.pause();

                // if we're talking about a panic, proper action must be taken
                // which in this case means restarting both the WASM sub
                // system and the machine state (to be able to recover)
                if (isPanic) {
                    await wasm(true);
                    await this.boot({ restore: false });

                    this.trigger("error");
                }
            }

            // calculates the amount of time until the next draw operation
            // this is the amount of time that is going to be pending
            currentTime = EmulatorBase.now();
            const pendingTime = Math.max(this.nextTickTime - currentTime, 0);

            // waits a little bit for the next frame to be draw,
            // this should control the flow of render
            await new Promise((resolve) => {
                setTimeout(resolve, pendingTime);
            });
        }
    }

    tick(currentTime: number, pending: number, cycles = 70224) {
        // in case the reference to the system is not set then
        // returns the control flow immediately (not possible to tick)
        if (!this.gameBoy) return pending;

        // in case the time to draw the next frame has not been
        // reached the flush of the "tick" logic is skipped
        if (currentTime < this.nextTickTime) return pending;

        // initializes the counter of cycles with the pending number
        // of cycles coming from the previous tick
        let counterCycles = pending;

        let lastFrame = this.gameBoy.ppu_frame();

        while (true) {
            // limits the number of cycles to the provided
            // cycle value passed as a parameter
            if (counterCycles >= cycles) {
                break;
            }

            // runs the Game Boy clock, this operations should
            // include the advance of both the CPU and the PPU
            const tickCycles = this.gameBoy.clock();
            counterCycles += tickCycles;

            // in case the frame is different from the previously
            // rendered one then it's time to update the canvas
            if (this.gameBoy.ppu_frame() !== lastFrame) {
                // updates the reference to the last frame index
                // to be used for comparison in the next tick
                lastFrame = this.gameBoy.ppu_frame();

                // triggers the frame event indicating that
                // a new frame is now available for drawing
                this.trigger("frame");
            }

            // in case the current cartridge is battery backed
            // then we need to check if a RAM flush to local
            // storage operation is required
            if (this.cartridge && this.cartridge.has_battery()) {
                this.flushCycles -= tickCycles;
                if (this.flushCycles <= 0) {
                    this.saveRam();
                    this.flushCycles = this.logicFrequency * STORE_RATE;
                }
            }
        }

        // triggers the audio event, meaning that the audio should be
        // processed for the current emulator, effectively emptying
        // the audio buffer that is pending processing
        this.trigger("audio");

        // increments the number of frames rendered in the current
        // section, this value is going to be used to calculate FPS
        this.frameCount += 1;

        // in case the target number of frames for FPS control
        // has been reached calculates the number of FPS and
        // flushes the value to the screen
        if (this.frameCount >= this.visualFrequency * FPS_SAMPLE_RATE) {
            const currentTime = EmulatorBase.now();
            const deltaTime = (currentTime - this.frameStart) / 1000;
            const fps = Math.round(this.frameCount / deltaTime);
            this.fps = fps;
            this.frameCount = 0;
            this.frameStart = currentTime;
        }

        // calculates the number of ticks that have elapsed since the
        // last draw operation, this is critical to be able to properly
        // operate the clock of the CPU in frame drop situations, meaning
        // a situation where the system resources are no able to emulate
        // the system on time and frames must be skipped (ticks > 1)
        if (this.nextTickTime === 0) this.nextTickTime = currentTime;
        let ticks = Math.ceil(
            (currentTime - this.nextTickTime) /
                ((1 / this.visualFrequency) * 1000)
        );
        ticks = Math.max(ticks, 1);

        // updates the next update time according to the number of ticks
        // that have elapsed since the last operation, this way this value
        // can better be used to control the game loop
        this.nextTickTime += (1000 / this.visualFrequency) * ticks;

        // calculates the new number of pending (overflow) cycles
        // that are going to be added to the next iteration
        return counterCycles - cycles;
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
        loadRom = false,
        romPath = ROM_PATH,
        romName = null,
        romData = null
    }: {
        engine?: string | null;
        restore?: boolean;
        loadRom?: boolean;
        romPath?: string;
        romName?: string | null;
        romData?: Uint8Array | null;
    } = {}) {
        // in case a remote ROM loading operation has been
        // requested then loads it from the remote origin
        if (loadRom) {
            ({ name: romName, data: romData } = await GameboyEmulator.fetchRom(
                romPath
            ));
        } else if (romName === null || romData === null) {
            [romName, romData] = [this.romName, this.romData];
        }

        // in case either the ROM's name or data is not available
        // throws an error as the boot process is not possible
        if (!romName || !romData) {
            throw new Error("Unable to load initial ROM");
        }

        // selects the proper engine for execution
        // and builds a new instance of it
        switch (engine) {
            case "auto":
                this.gameBoy = new GameBoy(GameBoyMode.Dmg);
                this.autoMode = true;
                break;
            case "cgb":
                this.gameBoy = new GameBoy(GameBoyMode.Cgb);
                this.autoMode = false;
                break;
            case "dmg":
                this.gameBoy = new GameBoy(GameBoyMode.Dmg);
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
            this.gameBoy.infer_mode_ws(romData);
        }

        // resets the Game Boy engine to restore it into
        // a valid state ready to be used
        this.gameBoy.reset();
        this.gameBoy.load(true);

        // loads the ROM file into the system and retrieves
        // the cartridge instance associated with it
        const cartridge = this.gameBoy.load_rom_ws(romData);

        // loads the callbacks so that the Typescript code
        // gets notified about the various events triggered
        // in the WASM side
        this.gameBoy.load_callbacks_ws();

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
        // current RAM from the local storage
        if (cartridge.has_battery()) this.loadRam();

        // in case the restore (state) flag is set
        // then resumes the machine execution
        if (restore) this.resume();

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
            Feature.Help,
            Feature.Debug,
            Feature.Themes,
            Feature.Palettes,
            Feature.Benchmark,
            Feature.Keyboard,
            Feature.KeyboardGB,
            Feature.RomTypeInfo,
            Feature.SaveState
        ];
    }

    get sections(): SectionInfo[] {
        return [
            {
                name: "Serial",
                icon: require("../res/serial.svg"),
                node: SerialSection({ emulator: this })
            }
        ];
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

    get engines(): string[] {
        return ["auto", "cgb", "dmg"];
    }

    get engine(): string {
        return this._engine || "auto";
    }

    get romExts(): string[] {
        return ["gb", "gbc"];
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
        return this.gameBoy?.frame_buffer_eager() ?? new Uint8Array();
    }

    get audioSpecs(): AudioSpecs {
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

    get romInfo(): RomInfo {
        return {
            name: this.romName ?? undefined,
            data: this.romData ?? undefined,
            size: this.romSize,
            extra: {
                romType: this.cartridge?.rom_type_s(),
                romSize: this.cartridge?.rom_size_s(),
                ramSize: this.cartridge?.ram_size_s()
            }
        };
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
        return this.gameBoy.wasm_engine_ws() ?? null;
    }

    get framerate(): number {
        return this.fps;
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

    toggleRunning() {
        if (this.paused) {
            this.resume();
        } else {
            this.pause();
        }
    }

    pause() {
        this.paused = true;
    }

    resume() {
        this.paused = false;
        this.nextTickTime = EmulatorBase.now();
    }

    reset() {
        this.boot({ engine: null });
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

    serializeState(): Uint8Array {
        if (!this.gameBoy) throw new Error("Unable to serialize state");
        return StateManager.save(this.gameBoy, SaveStateFormat.Bos);
    }

    unserializeState(data: Uint8Array) {
        if (!this.gameBoy) throw new Error("Unable to unserialize state");
        StateManager.load(data, this.gameBoy, SaveStateFormat.Bos);
    }

    buildState(index: number, data: Uint8Array): SaveState {
        const state = StateManager.read_bos(data);
        return {
            index: index,
            timestamp: Number(state.timestamp()),
            agent: state.agent(),
            model: state.model(),
            thumbnail: state.image_eager()
        };
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
            const initial = EmulatorBase.now();
            for (let i = 0; i < count; i++) {
                cycles += this.gameBoy?.clock() ?? 0;
            }
            const delta = (EmulatorBase.now() - initial) / 1000;
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
        this.gameBoy?.load_null_ws();
        if (set) this.serialDevice = SerialDevice.Null;
    }

    loadLoggerDevice(set = true) {
        this.gameBoy?.load_logger_ws();
        if (set) this.serialDevice = SerialDevice.Logger;
    }

    loadPrinterDevice(set = true) {
        this.gameBoy?.load_printer_ws();
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
     * `localStorage`, so that it can be latter restored.
     */
    private saveRam() {
        if (!this.gameBoy || !this.cartridge || !window.localStorage) return;
        if (!this.cartridge.has_battery()) return;
        const title = this.cartridge.title();
        const ramData = this.gameBoy.ram_data_eager();
        const ramDataB64 = bufferToBase64(ramData);
        localStorage.setItem(title, ramDataB64);
    }

    /**
     * Tries to load game RAM from the `localStorage` using the
     * current cartridge title as the name of the item and
     * decoding it using Base64.
     */
    private loadRam() {
        if (!this.gameBoy || !this.cartridge || !window.localStorage) return;
        const ramDataB64 = localStorage.getItem(this.cartridge.title());
        if (!ramDataB64) return;
        const ramData = base64ToBuffer(ramDataB64);
        this.gameBoy.set_ram_data(ramData);
    }

    private storeSettings() {
        if (!window.localStorage) return;
        const settings = {
            palette: PALETTES[this.paletteIndex].name,
            ...this.extraSettings
        };
        localStorage.setItem("settings", JSON.stringify(settings));
    }

    private updatePalette() {
        const palette = PALETTES[this.paletteIndex];
        this.gameBoy?.set_palette_colors_ws(palette.colors);
        this.storeSettings();
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
        emulator: GameboyEmulator;
        panic: (message: string) => void;
        speedCallback: (speed: GameBoySpeed) => void;
        loggerCallback: (data: Uint8Array) => void;
        printerCallback: (imageBuffer: Uint8Array) => void;
        rumbleCallback: (active: boolean) => void;
    }

    interface Console {
        image(url: string, size?: number): void;
    }
}

window.panic = (message: string) => {
    console.error(message);
};

window.speedCallback = (speed: GameBoySpeed) => {
    window.emulator.onSpeedSwitch(speed);
};

window.loggerCallback = (data: Uint8Array) => {
    window.emulator.onLoggerDevice(data);
};

window.printerCallback = (imageBuffer: Uint8Array) => {
    window.emulator.onPrinterDevice(imageBuffer);
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

console.image = (url: string, size = 80) => {
    const style = `font-size: ${size}px; background-image: url("${url}"); background-size: contain; background-repeat: no-repeat;`;
    console.log("%c     ", style);
};

const wasm = async (setHook = true) => {
    await _wasm();

    // in case the set hook flag is set, then tries to
    // set the panic hook for the WASM module, this call
    // may fail in some versions of wasm-bindgen as the
    // thread is still marked as "panicking", so we need to
    // wrap the call around try/catch
    if (setHook) {
        try {
            GameBoy.set_panic_hook_ws();
        } catch (err) {
            console.error(err);
        }
    }
};
