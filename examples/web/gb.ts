import {
    Emulator,
    EmulatorBase,
    Feature,
    PixelFormat,
    RomInfo
} from "./react/app";
import { PALETTES } from "./palettes";

import {
    Cartridge,
    default as _wasm,
    GameBoy,
    PadKey,
    PpuMode
} from "./lib/boytacean.js";
import info from "./package.json";
import { base64ToBuffer, bufferToBase64 } from "./util";

declare const require: any;

const LOGIC_HZ = 4194304;

const VISUAL_HZ = 59.7275;
const IDLE_HZ = 10;

/**
 * The rate at which the local storage RAM state flush
 * operation is going to be performed, this value is the
 * number of seconds in between flush operations (eg: 5 seconds).
 */
const STORE_RATE = 5;

const SAMPLE_RATE = 2;

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

const ROM_PATH = require("../../res/roms/20y.gb");

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

    private logicFrequency: number = LOGIC_HZ;
    private visualFrequency: number = VISUAL_HZ;
    private idleFrequency: number = IDLE_HZ;

    private paused: boolean = false;
    private nextTickTime: number = 0;
    private fps: number = 0;
    private frameStart: number = new Date().getTime();
    private frameCount: number = 0;
    private paletteIndex: number = 0;
    private storeCycles: number = LOGIC_HZ * STORE_RATE;

    private romName: string | null = null;
    private romData: Uint8Array | null = null;
    private romSize: number = 0;
    private cartridge: Cartridge | null = null;

    async main({ romUrl }: { romUrl?: string }) {
        // initializes the WASM module, this is required
        // so that the global symbols become available
        await wasm();

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
            let currentTime = new Date().getTime();

            try {
                pending = this.tick(
                    currentTime,
                    pending,
                    Math.round(this.logicFrequency / this.visualFrequency)
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
                // which in this case it means restarting both the WASM sub
                // system and the machine state (to be able to recover)
                // also sets the default color on screen to indicate the issue
                if (isPanic) {
                    await wasm();
                    await this.boot({ restore: false });

                    this.trigger("error");
                }
            }

            // calculates the amount of time until the next draw operation
            // this is the amount of time that is going to be pending
            currentTime = new Date().getTime();
            const pendingTime = Math.max(this.nextTickTime - currentTime, 0);

            // waits a little bit for the next frame to be draw,
            // this should control the flow of render
            await new Promise((resolve) => {
                setTimeout(resolve, pendingTime);
            });
        }
    }

    tick(currentTime: number, pending: number, cycles: number = 70224) {
        // in case the time to draw the next frame has not been
        // reached the flush of the "tick" logic is skipped
        if (currentTime < this.nextTickTime) return pending;

        // calculates the number of ticks that have elapsed since the
        // last draw operation, this is critical to be able to properly
        // operate the clock of the CPU in frame drop situations
        if (this.nextTickTime === 0) this.nextTickTime = currentTime;
        let ticks = Math.ceil(
            (currentTime - this.nextTickTime) /
                ((1 / this.visualFrequency) * 1000)
        );
        ticks = Math.max(ticks, 1);

        // initializes the counter of cycles with the pending number
        // of cycles coming from the previous tick
        let counterCycles = pending;

        let lastFrame = -1;

        while (true) {
            // limits the number of cycles to the provided
            // cycle value passed as a parameter
            if (counterCycles >= cycles) {
                break;
            }

            // runs the Game Boy clock, this operations should
            // include the advance of both the CPU and the PPU
            const tickCycles = this.gameBoy?.clock() ?? 0;
            counterCycles += tickCycles;

            // in case the current PPU mode is VBlank and the
            // frame is different from the previously rendered
            // one then it's time to update the canvas
            if (
                this.gameBoy?.ppu_mode() === PpuMode.VBlank &&
                this.gameBoy?.ppu_frame() !== lastFrame
            ) {
                // updates the reference to the last frame index
                // to be used for comparison in the next tick
                lastFrame = this.gameBoy?.ppu_frame();

                // triggers the frame event indicating that
                // a new frame is now available for drawing
                this.trigger("frame");
            }

            // in case the current cartridge is battery backed
            // then we need to check if a RAM flush to local
            // storage operation is required
            if (this.cartridge && this.cartridge.has_battery()) {
                this.storeCycles -= tickCycles;
                if (this.storeCycles <= 0) {
                    this.storeRam();
                    this.storeCycles = this.logicFrequency * STORE_RATE;
                }
            }
        }

        // increments the number of frames rendered in the current
        // section, this value is going to be used to calculate FPS
        this.frameCount += 1;

        // in case the target number of frames for FPS control
        // has been reached calculates the number of FPS and
        // flushes the value to the screen
        if (this.frameCount >= this.visualFrequency * SAMPLE_RATE) {
            const currentTime = new Date().getTime();
            const deltaTime = (currentTime - this.frameStart) / 1000;
            const fps = Math.round(this.frameCount / deltaTime);
            this.fps = fps;
            this.frameCount = 0;
            this.frameStart = currentTime;
        }

        // updates the next update time reference to the, so that it
        // can be used to control the game loop
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
        engine = "neo",
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
            ({ name: romName, data: romData } = await this.fetchRom(romPath));
        } else if (romName === null || romData === null) {
            [romName, romData] = [this.romName, this.romData];
        }

        // selects the proper engine for execution
        // and builds a new instance of it
        switch (engine) {
            case "neo":
                this.gameBoy = new GameBoy();
                break;
            default:
                if (!this.gameBoy) {
                    throw new Error("No engine requested");
                }
                break;
        }

        // runs the initial palette set operation, restoring
        // the palette of the emulator according to the currently
        // selected one
        this.setPalette();

        // resets the Game Boy engine to restore it into
        // a valid state ready to be used
        this.gameBoy.reset();
        this.gameBoy.load_boot_default();
        const cartridge = this.gameBoy.load_rom_ws(romData!);

        // updates the name of the currently selected engine
        // to the one that has been provided (logic change)
        if (engine) this._engine = engine;

        // updates the ROM name in case there's extra information
        // coming from the cartridge
        romName = cartridge.title() ? cartridge.title() : romName;

        // updates the complete set of global information that
        // is going to be displayed
        this.setRom(romName!, romData!, cartridge);

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

    get name(): string {
        return "Boytacean";
    }

    get device(): string {
        return "Game Boy";
    }

    get deviceUrl(): string {
        return "https://en.wikipedia.org/wiki/Game_Boy";
    }

    get engines() {
        return ["neo"];
    }

    get engine() {
        return this._engine;
    }

    get version(): string {
        return info.version;
    }

    get versionUrl(): string {
        return "https://gitlab.stage.hive.pt/joamag/boytacean/-/blob/master/CHANGELOG.md";
    }

    get features(): Feature[] {
        return [
            Feature.Debug,
            Feature.Palettes,
            Feature.Benchmark,
            Feature.Keyboard,
            Feature.KeyboardGB
        ];
    }

    get romExts(): string[] {
        return ["gb"];
    }

    get pixelFormat(): PixelFormat {
        return PixelFormat.RGB;
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
        this.trigger("frequency", value);
    }

    get frequencyDelta(): number | null {
        return 400000;
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

    getTile(index: number): Uint8Array {
        return this.gameBoy?.get_tile_buffer(index) ?? new Uint8Array();
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
        this.nextTickTime = new Date().getTime();
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

    changePalette() {
        this.paletteIndex += 1;
        this.paletteIndex %= PALETTES.length;
        this.setPalette();
    }

    benchmark(count = 50000000) {
        let cycles = 0;
        this.pause();
        try {
            const initial = Date.now();
            for (let i = 0; i < count; i++) {
                cycles += this.gameBoy?.clock() ?? 0;
            }
            const delta = (Date.now() - initial) / 1000;
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

    private loadRam() {
        if (!this.gameBoy || !this.cartridge || !window.localStorage) return;
        const ramDataB64 = localStorage.getItem(this.cartridge.title());
        if (!ramDataB64) return;
        const ramData = base64ToBuffer(ramDataB64);
        this.gameBoy.set_ram_data(ramData);
    }

    private storeRam() {
        if (!this.gameBoy || !this.cartridge || !window.localStorage) return;
        const title = this.cartridge.title();
        const ramData = this.gameBoy.ram_data_eager();
        const ramDataB64 = bufferToBase64(ramData);
        localStorage.setItem(title, ramDataB64);
    }

    private setPalette(index?: number) {
        index ??= this.paletteIndex;
        const palette = PALETTES[index];
        this.gameBoy?.set_palette_colors_ws(palette.colors);
    }

    private async fetchRom(
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
        panic: (message: string) => void;
    }
}

window.panic = (message: string) => {
    console.error(message);
};

const wasm = async () => {
    await _wasm();
    GameBoy.set_panic_hook_ws();
};
