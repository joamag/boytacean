import {
    Emulator,
    Observable,
    PixelFormat,
    RomInfo,
    startApp
} from "./react/app";

import {
    Cartridge,
    default as _wasm,
    GameBoy,
    PadKey,
    PpuMode
} from "./lib/boytacean.js";
import info from "./package.json";

declare const require: any;

const PIXEL_UNSET_COLOR = 0x1b1a17ff;

const LOGIC_HZ = 600;
const VISUAL_HZ = 60;
const TIMER_HZ = 60;
const IDLE_HZ = 10;

const FREQUENCY_DELTA = 60;

const DISPLAY_WIDTH = 160;
const DISPLAY_HEIGHT = 144;
const DISPLAY_RATIO = DISPLAY_WIDTH / DISPLAY_HEIGHT;

const SAMPLE_RATE = 2;

const BACKGROUNDS = [
    "264653",
    "1b1a17",
    "023047",
    "bc6c25",
    "283618",
    "2a9d8f",
    "3a5a40"
];

const KEYS: Record<string, number> = {
    ArrowUp: PadKey.Up,
    ArrowDown: PadKey.Down,
    ArrowLeft: PadKey.Left,
    ArrowRight: PadKey.Right,
    Enter: PadKey.Start,
    " ": PadKey.Select,
    a: PadKey.A,
    s: PadKey.B
};

const ROM_PATH = require("../../res/roms/20y.gb");

/**
 * Top level class that controls the emulator behaviour
 * and "joins" all the elements together to bring input/output
 * of the associated machine.
 */
class GameboyEmulator extends Observable implements Emulator {
    /**
     * The Game Boy engine (probably coming from WASM) that
     * is going to be used for the emulation.
     */
    private gameBoy: GameBoy | null = null;

    /**
     * The descriptive name of the engine that is currently
     * in use to emulate the system.
     */
    private engine: string | null = null;

    private logicFrequency: number = LOGIC_HZ;
    private visualFrequency: number = VISUAL_HZ;
    private timerFrequency: number = TIMER_HZ;
    private idleFrequency: number = IDLE_HZ;

    private canvas: HTMLCanvasElement | null = null;
    private canvasScaled: HTMLCanvasElement | null = null;
    private canvasCtx: CanvasRenderingContext2D | null = null;
    private canvasScaledCtx: CanvasRenderingContext2D | null = null;
    private image: ImageData | null = null;
    private videoBuff: DataView | null = null;
    private toastTimeout: number | null = null;
    private paused: boolean = false;
    private nextTickTime: number = 0;
    private fps: number = 0;
    private frameStart: number = new Date().getTime();
    private frameCount: number = 0;

    private romName: string | null = null;
    private romData: Uint8Array | null = null;
    private romSize: number = 0;
    private cartridge: Cartridge | null = null;

    async main() {
        // initializes the WASM module, this is required
        // so that the global symbols become available
        await wasm();

        // initializes the complete set of sub-systems
        // and registers the event handlers
        await this.init();
        await this.register();

        // start the emulator subsystem with the initial
        // ROM retrieved from a remote data source
        await this.start({ loadRom: true });

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
                pending = this.tick(currentTime, pending);
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
                this.showToast(message, true, 5000);
                console.error(err);

                // pauses the machine, allowing the end-user to act
                // on the error in a proper fashion
                this.pause();

                // if we're talking about a panic, proper action must be taken
                // which in this case it means restarting both the WASM sub
                // system and the machine state (to be able to recover)
                // also sets the default color on screen to indicate the issue
                if (isPanic) {
                    await this.clearCanvas(undefined, {
                        image: require("./res/storm.png"),
                        imageScale: 0.2
                    });

                    await wasm();
                    await this.start({ restore: false });
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
            counterCycles += this.gameBoy!.clock();

            // in case the current PPU mode is VBlank and the
            // frame is different from the previously rendered
            // one then it's time to update the canvas
            if (
                this.gameBoy!.ppu_mode() == PpuMode.VBlank &&
                this.gameBoy!.ppu_frame() != lastFrame
            ) {
                // updates the canvas object with the new
                // visual information coming in
                this.updateCanvas(
                    this.gameBoy!.frame_buffer_eager(),
                    PixelFormat.RGB
                );
                this.trigger("frame");
                lastFrame = this.gameBoy!.ppu_frame();
            }
        }

        // increments the number of frames rendered in the current
        // section, this value is going to be used to calculate FPS
        this.frameCount += 1;

        // in case the target number of frames for FPS control
        // has been reached calculates the number of FPS and
        // flushes the value to the screen
        if (this.frameCount === this.visualFrequency * SAMPLE_RATE) {
            const currentTime = new Date().getTime();
            const deltaTime = (currentTime - this.frameStart) / 1000;
            const fps = Math.round(this.frameCount / deltaTime);
            this.setFps(fps);
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
     * @param options The options that are going to be used in the
     * starting of the machine.
     */
    async start({
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
            [romName, romData] = await this.fetchRom(romPath);
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

        // resets the Game Boy engine to restore it into
        // a valid state ready to be used
        this.gameBoy.reset();
        this.gameBoy.load_boot_default();
        const cartridge = this.gameBoy.load_rom_ws(romData!);

        // updates the ROM name in case there's extra information
        // coming from the cartridge
        romName = cartridge.title() ? cartridge.title() : romName;

        // updates the name of the currently selected engine
        // to the one that has been provided (logic change)
        if (engine) this.engine = engine;

        // updates the complete set of global information that
        // is going to be displayed
        this.setEngine(this.engine!);
        this.setRom(romName!, romData!, cartridge);
        this.setLogicFrequency(this.logicFrequency);
        this.setFps(this.fps);

        // in case the restore (state) flag is set
        // then resumes the machine execution
        if (restore) this.resume();
    }

    // @todo remove this method, or at least most of it
    async register() {
        await Promise.all([
            this.registerDrop(),
            this.registerKeys(),
            this.registerButtons(),
            this.registerKeyboard(),
            this.registerCanvas(),
            this.registerToast(),
            this.registerModal()
        ]);
    }

    async init() {
        await Promise.all([this.initBase(), this.initCanvas()]);
    }

    registerDrop() {
        document.addEventListener("drop", async (event) => {
            if (
                !event.dataTransfer!.files ||
                event.dataTransfer!.files.length === 0
            ) {
                return;
            }

            event.preventDefault();
            event.stopPropagation();

            const overlay = document.getElementById("overlay")!;
            overlay.classList.remove("visible");

            const file = event.dataTransfer!.files[0];

            if (!file.name.endsWith(".gb")) {
                this.showToast(
                    "This is probably not a Game Boy ROM file!",
                    true
                );
                return;
            }

            const arrayBuffer = await file.arrayBuffer();
            const romData = new Uint8Array(arrayBuffer);

            this.start({ engine: null, romName: file.name, romData: romData });

            this.showToast(`Loaded ${file.name} ROM successfully!`);
        });
        document.addEventListener("dragover", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;

            event.preventDefault();

            const overlay = document.getElementById("overlay")!;
            overlay.classList.add("visible");
        });
        document.addEventListener("dragenter", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;
            const overlay = document.getElementById("overlay")!;
            overlay.classList.add("visible");
        });
        document.addEventListener("dragleave", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;
            const overlay = document.getElementById("overlay")!;
            overlay.classList.remove("visible");
        });
    }

    registerKeys() {
        document.addEventListener("keydown", (event) => {
            const keyCode = KEYS[event.key];
            if (keyCode !== undefined) {
                this.gameBoy!.key_press(keyCode);
                return;
            }

            switch (event.key) {
                case "+":
                    this.setLogicFrequency(
                        this.logicFrequency + FREQUENCY_DELTA
                    );
                    break;

                case "-":
                    this.setLogicFrequency(
                        this.logicFrequency - FREQUENCY_DELTA
                    );
                    break;

                case "Escape":
                    this.minimize();
                    break;
            }
        });

        document.addEventListener("keyup", (event) => {
            const keyCode = KEYS[event.key];
            if (keyCode !== undefined) {
                this.gameBoy!.key_lift(keyCode);
                return;
            }
        });
    }

    registerButtons() {
        const engine = document.getElementById("engine")!;
        engine.addEventListener("click", () => {
            const name = this.engine == "neo" ? "classic" : "neo";
            this.start({ engine: name });
            this.showToast(
                `Game Boy running in engine "${name.toUpperCase()}" from now on!`
            );
        });

        const logicFrequencyPlus = document.getElementById(
            "logic-frequency-plus"
        )!;
        logicFrequencyPlus.addEventListener("click", () => {
            this.setLogicFrequency(this.logicFrequency + FREQUENCY_DELTA);
        });

        const logicFrequencyMinus = document.getElementById(
            "logic-frequency-minus"
        )!;
        logicFrequencyMinus.addEventListener("click", () => {
            this.setLogicFrequency(this.logicFrequency - FREQUENCY_DELTA);
        });

        const buttonPause = document.getElementById("button-pause")!;
        buttonPause.addEventListener("click", () => {
            this.toggleRunning();
        });

        const buttonReset = document.getElementById("button-reset")!;
        buttonReset.addEventListener("click", () => {
            this.reset();
        });

        const buttonBenchmark = document.getElementById("button-benchmark")!;
        buttonBenchmark.addEventListener("click", async () => {
            const result = await this.showModal(
                "Are you sure you want to start a benchmark?\nThe benchmark is considered an expensive operation!",
                "Confirm"
            );
            if (!result) return;
            buttonBenchmark.classList.add("enabled");
            this.pause();
            try {
                const initial = Date.now();
                const count = 500000000;
                for (let i = 0; i < count; i++) {
                    this.gameBoy!.clock();
                }
                const delta = (Date.now() - initial) / 1000;
                const frequency_mhz = count / delta / 1000 / 1000;
                this.showToast(
                    `Took ${delta.toFixed(
                        2
                    )} seconds to run ${count} ticks (${frequency_mhz.toFixed(
                        2
                    )} Mhz)!`,
                    undefined,
                    7500
                );
            } finally {
                this.resume();
                buttonBenchmark.classList.remove("enabled");
            }
        });

        const buttonFullscreen = document.getElementById("button-fullscreen")!;
        buttonFullscreen.addEventListener("click", () => {
            this.maximize();
        });

        const buttonKeyboard = document.getElementById("button-keyboard")!;
        buttonKeyboard.addEventListener("click", () => {
            const sectionKeyboard =
                document.getElementById("section-keyboard")!;
            const separatorKeyboard =
                document.getElementById("separator-keyboard")!;
            const sectionNarrative =
                document.getElementById("section-narrative")!;
            const separatorNarrative = document.getElementById(
                "separator-narrative"
            )!;
            if (buttonKeyboard.classList.contains("enabled")) {
                sectionKeyboard.style.display = "none";
                separatorKeyboard.style.display = "none";
                sectionNarrative.style.display = "block";
                separatorNarrative.style.display = "block";
                buttonKeyboard.classList.remove("enabled");
            } else {
                sectionKeyboard.style.display = "block";
                separatorKeyboard.style.display = "block";
                sectionNarrative.style.display = "none";
                separatorNarrative.style.display = "none";
                buttonKeyboard.classList.add("enabled");
            }
        });

        const buttonDebug = document.getElementById("button-debug")!;
        buttonDebug.addEventListener("click", () => {
            const sectionDebug = document.getElementById("section-debug")!;
            const separatorDebug = document.getElementById("separator-debug")!;
            const sectionNarrative =
                document.getElementById("section-narrative")!;
            const separatorNarrative = document.getElementById(
                "separator-narrative"
            )!;
            if (buttonDebug.classList.contains("enabled")) {
                sectionDebug.style.display = "none";
                separatorDebug.style.display = "none";
                sectionNarrative.style.display = "block";
                separatorNarrative.style.display = "block";
                buttonDebug.classList.remove("enabled");
            } else {
                sectionDebug.style.display = "block";
                separatorDebug.style.display = "block";
                sectionNarrative.style.display = "none";
                separatorNarrative.style.display = "none";
                buttonDebug.classList.add("enabled");

                const canvasTiles = document.getElementById(
                    "canvas-tiles"
                ) as HTMLCanvasElement;
                const canvasTilesCtx = canvasTiles.getContext("2d")!;
                canvasTilesCtx.imageSmoothingEnabled = false;

                const canvasImage = canvasTilesCtx.createImageData(
                    canvasTiles.width,
                    canvasTiles.height
                );
                const videoBuff = new DataView(canvasImage.data.buffer);

                /**
                 * Draws the tile at the given index to the proper
                 * vertical offset in the given context and buffer.
                 *
                 * @param index The index of the sprite to be drawn.
                 * @param format The pixel format of the sprite.
                 */
                const drawTile = (
                    index: number,
                    context: CanvasRenderingContext2D,
                    buffer: DataView,
                    format: PixelFormat = PixelFormat.RGB
                ) => {
                    const pixels = this.gameBoy!.get_tile_buffer(index);
                    const line = Math.floor(index / 16);
                    const column = index % 16;
                    let offset =
                        (line * canvasTiles.width * 8 + column * 8) *
                        PixelFormat.RGBA;
                    let counter = 0;
                    for (
                        let index = 0;
                        index < pixels.length;
                        index += format
                    ) {
                        const color =
                            (pixels[index] << 24) |
                            (pixels[index + 1] << 16) |
                            (pixels[index + 2] << 8) |
                            (format == PixelFormat.RGBA
                                ? pixels[index + 3]
                                : 0xff);
                        buffer.setUint32(offset, color);

                        counter++;
                        if (counter == 8) {
                            counter = 0;
                            offset +=
                                (canvasTiles.width - 7) * PixelFormat.RGBA;
                        } else {
                            offset += PixelFormat.RGBA;
                        }
                    }
                    context.putImageData(canvasImage, 0, 0);
                };

                for (let index = 0; index < 384; index++) {
                    drawTile(index, canvasTilesCtx, videoBuff);
                }

                const vram = this.gameBoy!.vram_eager();
                const step = 16;
                for (let index = 0; index < vram.length; index += step) {
                    let line = `${(index + 0x8000)
                        .toString(16)
                        .padStart(4, "0")}`;
                    for (let j = 0; j < step; j++) {
                        line += ` ${vram[index + j]
                            .toString(16)
                            .padStart(2, "0")}`;
                    }
                    console.info(line);
                }
            }
        });

        const buttonInformation =
            document.getElementById("button-information")!;
        buttonInformation.addEventListener("click", () => {
            const sectionDiag = document.getElementById("section-diag")!;
            const separatorDiag = document.getElementById("separator-diag")!;
            if (buttonInformation.classList.contains("enabled")) {
                sectionDiag.style.display = "none";
                separatorDiag.style.display = "none";
                buttonInformation.classList.remove("enabled");
            } else {
                sectionDiag.style.display = "block";
                separatorDiag.style.display = "block";
                buttonInformation.classList.add("enabled");
            }
        });

        const buttonUploadFile = document.getElementById(
            "button-upload-file"
        ) as HTMLInputElement;
        buttonUploadFile.addEventListener("change", async () => {
            if (
                !buttonUploadFile.files ||
                buttonUploadFile.files.length === 0
            ) {
                return;
            }

            const file = buttonUploadFile.files[0];

            const arrayBuffer = await file.arrayBuffer();
            const romData = new Uint8Array(arrayBuffer);

            buttonUploadFile.value = "";

            this.start({ engine: null, romName: file.name, romData: romData });

            this.showToast(`Loaded ${file.name} ROM successfully!`);
        });
    }

    // @todo this should be converted into a component
    registerKeyboard() {
        const keyboard = document.getElementById("keyboard")!;
        const keys = keyboard.getElementsByClassName("key");

        keyboard.addEventListener("touchstart", function (event) {
            event.preventDefault();
            event.stopPropagation();
        });

        keyboard.addEventListener("touchend", function (event) {
            event.preventDefault();
            event.stopPropagation();
        });

        Array.prototype.forEach.call(keys, (k: Element) => {
            k.addEventListener(
                "mousedown",
                function (this: HTMLElement, event) {
                    const keyCode = KEYS[this.textContent!.toLowerCase()];
                    //this.gameBoy.key_press_ws(keyCode); @todo
                    event.preventDefault();
                    event.stopPropagation();
                }
            );

            k.addEventListener(
                "touchstart",
                function (this: HTMLElement, event) {
                    const keyCode = KEYS[this.textContent!.toLowerCase()];
                    //this.gameBoy.key_press_ws(keyCode); @todo
                    event.preventDefault();
                    event.stopPropagation();
                }
            );

            k.addEventListener("mouseup", function (this: HTMLElement, event) {
                const keyCode = KEYS[this.textContent!.toLowerCase()];
                //this.gameBoy.key_lift_ws(keyCode); @todo
                event.preventDefault();
                event.stopPropagation();
            });

            k.addEventListener("touchend", function (this: HTMLElement, event) {
                const keyCode = KEYS[this.textContent!.toLowerCase()];
                //this.gameBoy.key_lift_ws(keyCode); @todo
                event.preventDefault();
                event.stopPropagation();
            });
        });
    }

    registerCanvas() {
        const canvasClose = document.getElementById("canvas-close")!;
        canvasClose.addEventListener("click", () => {
            this.minimize();
        });
    }

    registerToast() {
        const toast = document.getElementById("toast")!;
        toast.addEventListener("click", () => {
            toast.classList.remove("visible");
        });
    }

    registerModal() {
        const modalClose = document.getElementById("modal-close")!;
        modalClose.addEventListener("click", () => {
            this.hideModal(false);
        });

        const modalCancel = document.getElementById("modal-cancel")!;
        modalCancel.addEventListener("click", () => {
            this.hideModal(false);
        });

        const modalConfirm = document.getElementById("modal-confirm")!;
        modalConfirm.addEventListener("click", () => {
            this.hideModal(true);
        });

        document.addEventListener("keydown", (event) => {
            if (event.key === "Escape") {
                this.hideModal(false);
            }
        });
    }

    async initBase() {
        this.setVersion(info.version);
    }

    async initCanvas() {
        // initializes the off-screen canvas that is going to be
        // used in the drawing process
        this.canvas = document.createElement("canvas");
        this.canvas.width = DISPLAY_WIDTH;
        this.canvas.height = DISPLAY_HEIGHT;
        this.canvasCtx = this.canvas.getContext("2d")!;

        this.canvasScaled = document.getElementById(
            "engine-canvas"
        ) as HTMLCanvasElement;
        this.canvasScaled.width =
            this.canvasScaled.width * window.devicePixelRatio;
        this.canvasScaled.height =
            this.canvasScaled.height * window.devicePixelRatio;
        this.canvasScaledCtx = this.canvasScaled.getContext("2d")!;

        this.canvasScaledCtx.scale(
            this.canvasScaled.width / this.canvas.width,
            this.canvasScaled.height / this.canvas.height
        );
        this.canvasScaledCtx.imageSmoothingEnabled = false;

        this.image = this.canvasCtx.createImageData(
            this.canvas.width,
            this.canvas.height
        );
        this.videoBuff = new DataView(this.image.data.buffer);
    }

    updateCanvas(pixels: Uint8Array, format: PixelFormat = PixelFormat.RGB) {
        let offset = 0;
        for (let index = 0; index < pixels.length; index += format) {
            const color =
                (pixels[index] << 24) |
                (pixels[index + 1] << 16) |
                (pixels[index + 2] << 8) |
                (format == PixelFormat.RGBA ? pixels[index + 3] : 0xff);
            this.videoBuff!.setUint32(offset, color);
            offset += PixelFormat.RGBA;
        }
        this.canvasCtx!.putImageData(this.image!, 0, 0);
        this.canvasScaledCtx!.drawImage(this.canvas!, 0, 0);
    }

    async clearCanvas(
        color = PIXEL_UNSET_COLOR,
        {
            image = null,
            imageScale = 1
        }: { image?: string | null; imageScale?: number } = {}
    ) {
        this.canvasScaledCtx!.fillStyle = `#${color
            .toString(16)
            .toUpperCase()}`;
        this.canvasScaledCtx!.fillRect(
            0,
            0,
            this.canvasScaled!.width,
            this.canvasScaled!.height
        );

        // in case an image was requested then uses that to load
        // an image at the center of the screen properly scaled
        if (image) {
            const img = await new Promise<HTMLImageElement>((resolve) => {
                const img = new Image();
                img.onload = () => {
                    resolve(img);
                };
                img.src = image;
            });
            const [imgWidth, imgHeight] = [
                img.width * imageScale * window.devicePixelRatio,
                img.height * imageScale * window.devicePixelRatio
            ];
            const [x0, y0] = [
                this.canvasScaled!.width / 2 - imgWidth / 2,
                this.canvasScaled!.height / 2 - imgHeight / 2
            ];
            this.canvasScaledCtx!.setTransform(1, 0, 0, 1, 0, 0);
            try {
                this.canvasScaledCtx!.drawImage(
                    img,
                    x0,
                    y0,
                    imgWidth,
                    imgHeight
                );
            } finally {
                this.canvasScaledCtx!.scale(
                    this.canvasScaled!.width / this.canvas!.width,
                    this.canvasScaled!.height / this.canvas!.height
                );
            }
        }
    }

    async showToast(message: string, error = false, timeout = 3500) {
        const toast = document.getElementById("toast")!;
        toast.classList.remove("error");
        if (error) toast.classList.add("error");
        toast.classList.add("visible");
        toast.textContent = message;
        if (this.toastTimeout) clearTimeout(this.toastTimeout);
        this.toastTimeout = setTimeout(() => {
            toast.classList.remove("visible");
            this.toastTimeout = null;
        }, timeout);
    }

    async showModal(message: string, title = "Alert"): Promise<boolean> {
        const modalContainer = document.getElementById("modal-container")!;
        const modalTitle = document.getElementById("modal-title")!;
        const modalText = document.getElementById("modal-text")!;
        modalContainer.classList.add("visible");
        modalTitle.textContent = title;
        modalText.innerHTML = message.replace(/\n/g, "<br/>");
        const result = (await new Promise((resolve) => {
            global.modalCallback = resolve;
        })) as boolean;
        return result;
    }

    async hideModal(result = true) {
        const modalContainer = document.getElementById("modal-container")!;
        modalContainer.classList.remove("visible");
        if (global.modalCallback) global.modalCallback(result);
        global.modalCallback = null;
    }

    setVersion(value: string) {
        document.getElementById("version")!.textContent = value;
    }

    setEngine(name: string, upper = true) {
        name = upper ? name.toUpperCase() : name;
        document.getElementById("engine")!.textContent = name;
    }

    setRom(name: string, data: Uint8Array, cartridge: Cartridge) {
        this.romName = name;
        this.romData = data;
        this.romSize = data.length;
        this.cartridge = cartridge;

        this.trigger("rom:loaded");
    }

    setLogicFrequency(value: number) {
        if (value < 0) this.showToast("Invalid frequency value!", true);
        value = Math.max(value, 0);
        this.logicFrequency = value;
        document.getElementById("logic-frequency")!.textContent = String(value);
    }

    setFps(value: number) {
        if (value < 0) this.showToast("Invalid FPS value!", true);
        value = Math.max(value, 0);
        this.fps = value;
    }

    getName() {
        return "Boytacean";
    }

    getVersion() {
        return info.version;
    }

    getVersionUrl() {
        return "https://gitlab.stage.hive.pt/joamag/boytacean/-/blob/master/CHANGELOG.md";
    }

    getPixelFormat(): PixelFormat {
        return PixelFormat.RGB;
    }

    /**
     * Returns the array buffer that contains the complete set of
     * pixel data that is going to be drawn.
     *
     * @returns The current pixel data for the emulator display.
     */
    getImageBuffer(): Uint8Array {
        return this.gameBoy!.frame_buffer_eager();
    }

    getRomInfo(): RomInfo {
        return {
            name: this.romName || undefined,
            data: this.romData || undefined,
            size: this.romData?.length,
            extra: {
                romType: this.cartridge?.rom_type_s(),
                romSize: this.cartridge?.rom_size_s(),
                ramSize: this.cartridge?.ram_size_s()
            }
        };
    }

    getFramerate(): number {
        return this.fps;
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
        const buttonPause = document.getElementById("button-pause")!;
        const img = buttonPause.getElementsByTagName("img")[0];
        const span = buttonPause.getElementsByTagName("span")[0];
        buttonPause.classList.add("enabled");
        img.src = require("./res/play.svg");
        span.textContent = "Resume";
    }

    resume() {
        this.paused = false;
        this.nextTickTime = new Date().getTime();
        const buttonPause = document.getElementById("button-pause")!;
        const img = buttonPause.getElementsByTagName("img")[0];
        const span = buttonPause.getElementsByTagName("span")[0];
        buttonPause.classList.remove("enabled");
        img.src = require("./res/pause.svg");
        span.textContent = "Pause";
    }

    /**
     * Resets the emulator machine to the start state and loads
     * the ROM that is currently set in the emulator.
     */
    reset() {
        this.start({ engine: null });
    }

    toggleWindow() {
        this.maximize();
    }

    /**
     * Maximizes the emulator's viewport taking up all the available
     * window space. This method is responsible for keeping the aspect
     * ratio of the emulator canvas according to the width/height ratio.
     */
    maximize() {
        const canvasContainer = document.getElementById("canvas-container")!;
        canvasContainer.classList.add("fullscreen");

        window.addEventListener("resize", this.crop);

        this.crop();
    }

    /**
     * Restore the emulator's viewport to the minimal size, should make all
     * the other emulator's meta-information (info, buttons, etc.) visible.
     */
    minimize() {
        const canvasContainer = document.getElementById("canvas-container")!;
        const engineCanvas = document.getElementById("engine-canvas")!;
        canvasContainer.classList.remove("fullscreen");
        engineCanvas.style.width = "";
        engineCanvas.style.height = "";
        window.removeEventListener("resize", this.crop);
    }

    crop() {
        // @todo let's make this more flexible
        const engineCanvas = document.getElementById("engine-canvas")!;

        // calculates the window ratio as this is fundamental to
        // determine the proper way to crop the fullscreen
        const windowRatio = window.innerWidth / window.innerHeight;

        // in case the window is wider (more horizontal than the base ratio)
        // this means that we must crop horizontally
        if (windowRatio > DISPLAY_RATIO) {
            engineCanvas.style.width = `${
                window.innerWidth * (DISPLAY_RATIO / windowRatio)
            }px`;
            engineCanvas.style.height = `${window.innerHeight}px`;
        } else {
            engineCanvas.style.width = `${window.innerWidth}px`;
            engineCanvas.style.height = `${
                window.innerHeight * (windowRatio / DISPLAY_RATIO)
            }px`;
        }
    }

    async fetchRom(romPath: string): Promise<[string, Uint8Array]> {
        // extracts the name of the ROM from the provided
        // path by splitting its structure
        const romPathS = romPath.split(/\//g);
        let romName = romPathS[romPathS.length - 1].split("?")[0];
        const romNameS = romName.split(/\./g);
        romName = `${romNameS[0]}.${romNameS[romNameS.length - 1]}`;

        // loads the ROM data and converts it into the
        // target byte array buffer (to be used by WASM)
        const response = await fetch(ROM_PATH);
        const blob = await response.blob();
        const arrayBuffer = await blob.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);

        // returns both the name of the ROM and the data
        // contents as a byte array
        return [romName, romData];
    }
}

type Global = {
    modalCallback: Function | null;
};

//@todo check if this is really required
const global: Global = {
    modalCallback: null
};

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

(async () => {
    const emulator = new GameboyEmulator();
    startApp("app", emulator, BACKGROUNDS);
    await emulator.main();
})();
