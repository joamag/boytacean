import { default as wasm, GameBoy } from "./lib/boytacean.js";
import info from "./package.json";

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

const SOUND_DATA =
    "data:audio/mpeg;base64,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AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP/7ksD/gAAAASwAAAAAAAAlgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/+5LA/4AAAAEsAAAAAAAAJYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA//uSwP+AAAABLAAAAAAAACWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP/7ksD/gAAAASwAAAAAAAAlgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/+5LA/4AAAAEsAAAAAAAAJYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA//uSwP+AAAABLAAAAAAAACWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP/7ksD/gAAAASwAAAAAAAAlgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAD/+5LA/4AAAAEsAAAAAAAAJYAAAAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAgACAAIAAA";

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
    "1": 0x01,
    "2": 0x02,
    "3": 0x03,
    "4": 0x0c,
    q: 0x04,
    w: 0x05,
    e: 0x06,
    r: 0x0d,
    a: 0x07,
    s: 0x08,
    d: 0x09,
    f: 0x0e,
    z: 0x0a,
    x: 0x00,
    c: 0x0b,
    v: 0x0f
};

// @ts-ignore: ts(2580)
const ROM_PATH = require("../../res/roms/ld_r_r.gb");

type State = {
    gameBoy: GameBoy;
    engine: string;
    logicFrequency: number;
    visualFrequency: number;
    timerFrequency: number;
    idleFrequency: number;
    canvas: HTMLCanvasElement;
    canvasScaled: HTMLCanvasElement;
    canvasCtx: CanvasRenderingContext2D;
    canvasScaledCtx: CanvasRenderingContext2D;
    image: ImageData;
    videoBuff: DataView;
    toastTimeout: number;
    paused: boolean;
    background_index: number;
    nextTickTime: number;
    fps: number;
    frameStart: number;
    frameCount: number;
    romName: string;
    romData: Uint8Array;
    romSize: number;
};

type Global = {
    modalCallback: Function;
};

const state: State = {
    gameBoy: null,
    engine: null,
    logicFrequency: LOGIC_HZ,
    visualFrequency: VISUAL_HZ,
    timerFrequency: TIMER_HZ,
    idleFrequency: IDLE_HZ,
    canvas: null,
    canvasScaled: null,
    canvasCtx: null,
    canvasScaledCtx: null,
    image: null,
    videoBuff: null,
    toastTimeout: null,
    paused: false,
    background_index: 0,
    nextTickTime: 0,
    fps: 0,
    frameStart: new Date().getTime(),
    frameCount: 0,
    romName: null,
    romData: null,
    romSize: 0
};

const global: Global = {
    modalCallback: null
};

const sound = ((data = SOUND_DATA, volume = 0.2) => {
    const sound = new Audio(data);
    sound.volume = volume;
    sound.muted = true;
    return sound;
})();

const main = async () => {
    // initializes the WASM module, this is required
    // so that the global symbols become available
    await wasm();

    // initializes the complete set of sub-systems
    // and registers the event handlers
    await init();
    await register();

    // start the emulator subsystem with the initial
    // ROM retrieved from a remote data source
    await start({ loadRom: true });

    // runs the sequence as an infinite loop, running
    // the associated CPU cycles accordingly
    while (true) {
        // in case the machin is paused we must delay the execution
        // a little bit until the paused state is recovered
        if (state.paused) {
            await new Promise((resolve) => {
                setTimeout(resolve, 1000 / state.idleFrequency);
            });
            continue;
        }

        // obtains the current time, this value is going
        // to be used to compute the need for tick computation
        let currentTime = new Date().getTime();

        try {
            tick(currentTime);
        } catch (err) {
            // sets the default error message to be displayed
            // to the user
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
            // and the developer (for dianostics)
            showToast(message, true, 5000);
            console.error(err);

            // pauses the machine, allowing the end-user to act
            // on the error in a proper fashion
            pause();

            // if we're talking about a panic proper action must be taken
            // which in this case it means restarting both the WASM sub
            // system and the machine state (to be able to recover)
            // also sets the default color on screen to indicate the issue
            if (isPanic) {
                await clearCanvas(undefined, {
                    // @ts-ignore: ts(2580)
                    image: require("./res/storm.png"),
                    imageScale: 0.4
                });

                await wasm();
                await start({ restore: false });
            }
        }

        // calculates the amount of time until the next draw operation
        // this is the amount of time that is going to be pending
        currentTime = new Date().getTime();
        const pendingTime = Math.max(state.nextTickTime - currentTime, 0);

        // waits a little bit for the next frame to be draw,
        // this should control the flow of render
        await new Promise((resolve) => {
            setTimeout(resolve, pendingTime);
        });
    }
};

const tick = (currentTime: number) => {
    // in case the time to draw the next frame has not been
    // reached the flush of the "tick" logic is skiped
    if (currentTime < state.nextTickTime) return;

    // calculates the number of ticks that have elapsed since the
    // last draw operation, this is critical to be able to properly
    // operate the clock of the CPU in frame drop situations
    if (state.nextTickTime === 0) state.nextTickTime = currentTime;
    let ticks = Math.ceil(
        (currentTime - state.nextTickTime) /
            ((1 / state.visualFrequency) * 1000)
    );
    ticks = Math.max(ticks, 1);

    let counterTicks = 0;

    while (true) {
        // limits the number of ticks to the typical number
        // of ticks required to do a complete PPU draw
        if (counterTicks >= 70224) {
            break;
        }

        // runs the Game Boy clock, this operations should
        // include the advance of both the CPU and the PPU
        counterTicks += state.gameBoy.clock();
    }

    // updates the canvas object with the new
    // visual information coming in
    updateCanvas(state.gameBoy.frame_buffer_eager());

    // increments the number of frames rendered in the current
    // section, this value is going to be used to calculate FPS
    state.frameCount += 1;

    // in case the target number of frames for FPS control
    // has been reached calculates the number of FPS and
    // flushes the value to the screen
    if (state.frameCount === state.visualFrequency * SAMPLE_RATE) {
        const currentTime = new Date().getTime();
        const deltaTime = (currentTime - state.frameStart) / 1000;
        const fps = Math.round(state.frameCount / deltaTime);
        setFps(fps);
        state.frameCount = 0;
        state.frameStart = currentTime;
    }

    // updates the next update time reference to the, so that it
    // can be used to control the game loop
    state.nextTickTime += (1000 / state.visualFrequency) * ticks;
};

const start = async ({
    engine = "neo",
    restore = true,
    loadRom = false,
    romPath = ROM_PATH,
    romName = null as string,
    romData = null as Uint8Array
} = {}) => {
    // in case a remote ROM loading operation has been
    // requested then loads it from the remote origin
    if (loadRom) {
        [romName, romData] = await fetchRom(romPath);
    } else if (romName === null || romData === null) {
        [romName, romData] = [state.romName, state.romData];
    }

    // selects the proper engine for execution
    // and builds a new instance of it
    switch (engine) {
        case "neo":
            state.gameBoy = new GameBoy();
            break;

        default:
            if (!state.gameBoy) {
                throw new Error("No engine requested");
            }
            break;
    }

    // resets the Game Boy engine to restore it into
    // a valid state ready to be used
    //state.gameBoy.reset_hard(); @todo
    state.gameBoy.load_boot_static();
    state.gameBoy.load_rom(romData);

    // updates the name of the currently selected engine
    // to the one that has been provided (logic change)
    if (engine) state.engine = engine;

    // updates the complete set of global information that
    // is going to be displayed
    setEngine(state.engine);
    setRom(romName, romData);
    setLogicFrequency(state.logicFrequency);
    setFps(state.fps);

    // in case the restore (state) flag is set
    // then resumes the machine execution
    if (restore) resume();
};

const register = async () => {
    await Promise.all([
        registerDrop(),
        registerKeys(),
        registerButtons(),
        registerKeyboard(),
        registerCanvas(),
        registerToast(),
        registerModal()
    ]);
};

const init = async () => {
    await Promise.all([initBase(), initCanvas()]);
};

const registerDrop = () => {
    document.addEventListener("drop", async (event) => {
        if (
            !event.dataTransfer.files ||
            event.dataTransfer.files.length === 0
        ) {
            return;
        }

        event.preventDefault();
        event.stopPropagation();

        const overlay = document.getElementById("overlay");
        overlay.classList.remove("visible");

        const file = event.dataTransfer.files[0];

        if (!file.name.endsWith(".gb")) {
            showToast("This is probably not a Game Boy ROM file!", true);
            return;
        }

        const arrayBuffer = await file.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);

        start({ engine: null, romName: file.name, romData: romData });

        showToast(`Loaded ${file.name} ROM successfully!`);
    });
    document.addEventListener("dragover", async (event) => {
        if (!event.dataTransfer.items || event.dataTransfer.items[0].type)
            return;

        event.preventDefault();

        const overlay = document.getElementById("overlay");
        overlay.classList.add("visible");
    });
    document.addEventListener("dragenter", async (event) => {
        if (!event.dataTransfer.items || event.dataTransfer.items[0].type)
            return;
        const overlay = document.getElementById("overlay");
        overlay.classList.add("visible");
    });
    document.addEventListener("dragleave", async (event) => {
        if (!event.dataTransfer.items || event.dataTransfer.items[0].type)
            return;
        const overlay = document.getElementById("overlay");
        overlay.classList.remove("visible");
    });
};

const registerKeys = () => {
    document.addEventListener("keydown", (event) => {
        const keyCode = KEYS[event.key];
        if (keyCode !== undefined) {
            //state.gameBoy.key_press_ws(keyCode); @todo
            return;
        }

        switch (event.key) {
            case "+":
                setLogicFrequency(state.logicFrequency + FREQUENCY_DELTA);
                break;

            case "-":
                setLogicFrequency(state.logicFrequency - FREQUENCY_DELTA);
                break;

            case "Escape":
                minimize();
                break;
        }
    });

    document.addEventListener("keyup", (event) => {
        const keyCode = KEYS[event.key];
        if (keyCode !== undefined) {
            //state.gameBoy.key_lift_ws(keyCode); @todo
            return;
        }
    });
};

const registerButtons = () => {
    const engine = document.getElementById("engine");
    engine.addEventListener("click", () => {
        const name = state.engine == "neo" ? "classic" : "neo";
        start({ engine: name });
        showToast(
            `Game Boy running in engine "${name.toUpperCase()}" from now on!`
        );
    });

    const logicFrequencyPlus = document.getElementById("logic-frequency-plus");
    logicFrequencyPlus.addEventListener("click", () => {
        setLogicFrequency(state.logicFrequency + FREQUENCY_DELTA);
    });

    const logicFrequencyMinus = document.getElementById(
        "logic-frequency-minus"
    );
    logicFrequencyMinus.addEventListener("click", () => {
        setLogicFrequency(state.logicFrequency - FREQUENCY_DELTA);
    });

    const buttonPause = document.getElementById("button-pause");
    buttonPause.addEventListener("click", () => {
        toggleRunning();
    });

    const buttonReset = document.getElementById("button-reset");
    buttonReset.addEventListener("click", () => {
        reset();
    });

    const buttonBenchmark = document.getElementById("button-benchmark");
    buttonBenchmark.addEventListener("click", async () => {
        const result = await showModal(
            "Are you sure you want to start a benchmark?\nThe benchmark is considered an expensive operation!",
            "Confirm"
        );
        if (!result) return;
        buttonBenchmark.classList.add("enabled");
        pause();
        try {
            const initial = Date.now();
            const count = 500000000;
            for (let i = 0; i < count; i++) {
                state.gameBoy.clock();
            }
            const delta = (Date.now() - initial) / 1000;
            const frequency_mhz = count / delta / 1000 / 1000;
            showToast(
                `Took ${delta.toFixed(
                    2
                )} seconds to run ${count} ticks (${frequency_mhz.toFixed(
                    2
                )} Mhz)!`,
                undefined,
                7500
            );
        } finally {
            resume();
            buttonBenchmark.classList.remove("enabled");
        }
    });

    const buttonFullscreen = document.getElementById("button-fullscreen");
    buttonFullscreen.addEventListener("click", () => {
        maximize();
    });

    const buttonKeyboard = document.getElementById("button-keyboard");
    buttonKeyboard.addEventListener("click", () => {
        const sectionKeyboard = document.getElementById("section-keyboard");
        const separatorKeyboard = document.getElementById("separator-keyboard");
        const sectionNarrative = document.getElementById("section-narrative");
        const separatorNarrative = document.getElementById(
            "separator-narrative"
        );
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

    const buttonInformation = document.getElementById("button-information");
    buttonInformation.addEventListener("click", () => {
        const sectionDiag = document.getElementById("section-diag");
        const separatorDiag = document.getElementById("separator-diag");
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

    const buttonTheme = document.getElementById("button-theme");
    buttonTheme.addEventListener("click", () => {
        state.background_index =
            (state.background_index + 1) % BACKGROUNDS.length;
        const background = BACKGROUNDS[state.background_index];
        setBackground(background);
    });

    const buttonUploadFile = document.getElementById(
        "button-upload-file"
    ) as HTMLInputElement;
    buttonUploadFile.addEventListener("change", async () => {
        if (!buttonUploadFile.files || buttonUploadFile.files.length === 0) {
            return;
        }

        const file = buttonUploadFile.files[0];

        const arrayBuffer = await file.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);

        buttonUploadFile.value = "";

        start({ engine: null, romName: file.name, romData: romData });

        showToast(`Loaded ${file.name} ROM successfully!`);
    });
};

const registerKeyboard = () => {
    const keyboard = document.getElementById("keyboard");
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
        k.addEventListener("mousedown", function (event) {
            const keyCode = KEYS[this.textContent.toLowerCase()];
            //state.gameBoy.key_press_ws(keyCode); @todo
            event.preventDefault();
            event.stopPropagation();
        });

        k.addEventListener("touchstart", function (event) {
            const keyCode = KEYS[this.textContent.toLowerCase()];
            //state.gameBoy.key_press_ws(keyCode); @todo
            event.preventDefault();
            event.stopPropagation();
        });

        k.addEventListener("mouseup", function (event) {
            const keyCode = KEYS[this.textContent.toLowerCase()];
            //state.gameBoy.key_lift_ws(keyCode); @todo
            event.preventDefault();
            event.stopPropagation();
        });

        k.addEventListener("touchend", function (event) {
            const keyCode = KEYS[this.textContent.toLowerCase()];
            //state.gameBoy.key_lift_ws(keyCode); @todo
            event.preventDefault();
            event.stopPropagation();
        });
    });
};

const registerCanvas = () => {
    const canvasClose = document.getElementById("canvas-close");
    canvasClose.addEventListener("click", () => {
        minimize();
    });
};

const registerToast = () => {
    const toast = document.getElementById("toast");
    toast.addEventListener("click", () => {
        toast.classList.remove("visible");
    });
};

const registerModal = () => {
    const modalClose = document.getElementById("modal-close");
    modalClose.addEventListener("click", () => {
        hideModal(false);
    });

    const modalCancel = document.getElementById("modal-cancel");
    modalCancel.addEventListener("click", () => {
        hideModal(false);
    });

    const modalConfirm = document.getElementById("modal-confirm");
    modalConfirm.addEventListener("click", () => {
        hideModal(true);
    });

    document.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
            hideModal(false);
        }
    });
};

const initBase = async () => {
    const background = BACKGROUNDS[state.background_index];
    setBackground(background);
    setVersion(info.version);
};

const initCanvas = async () => {
    // initializes the off-screen canvas that is going to be
    // used in the drawing process
    state.canvas = document.createElement("canvas");
    state.canvas.width = DISPLAY_WIDTH;
    state.canvas.height = DISPLAY_HEIGHT;
    state.canvasCtx = state.canvas.getContext("2d");

    state.canvasScaled = document.getElementById(
        "chip-canvas"
    ) as HTMLCanvasElement;
    state.canvasScaled.width =
        state.canvasScaled.width * window.devicePixelRatio;
    state.canvasScaled.height =
        state.canvasScaled.height * window.devicePixelRatio;
    state.canvasScaledCtx = state.canvasScaled.getContext("2d");

    state.canvasScaledCtx.scale(
        state.canvasScaled.width / state.canvas.width,
        state.canvasScaled.height / state.canvas.height
    );
    state.canvasScaledCtx.imageSmoothingEnabled = false;

    state.image = state.canvasCtx.createImageData(
        state.canvas.width,
        state.canvas.height
    );
    state.videoBuff = new DataView(state.image.data.buffer);
};

const updateCanvas = (pixels: Uint8Array) => {
    let offset = 0;
    for (let index = 0; index < pixels.length; index += 3) {
        const color =
            (pixels[index] << 24) |
            (pixels[index + 1] << 16) |
            (pixels[index + 2] << 8) |
            0xff;
        state.videoBuff.setUint32(offset, color);
        offset += 4;
    }
    state.canvasCtx.putImageData(state.image, 0, 0);
    state.canvasScaledCtx.drawImage(state.canvas, 0, 0);
};

const clearCanvas = async (
    color = PIXEL_UNSET_COLOR,
    { image = null as string, imageScale = 1 } = {}
) => {
    state.canvasScaledCtx.fillStyle = `#${color.toString(16).toUpperCase()}`;
    state.canvasScaledCtx.fillRect(
        0,
        0,
        state.canvasScaled.width,
        state.canvasScaled.height
    );

    // in case an image was requested then uses that to load
    // an image at the center of the screen
    if (image) {
        const img = await new Promise<HTMLImageElement>((resolve) => {
            const img = new Image();
            img.onload = () => {
                resolve(img);
            };
            img.src = image;
        });
        const [imgWidth, imgHeight] = [
            img.width * imageScale,
            img.height * imageScale
        ];
        const [x0, y0] = [
            state.canvasScaled.width / 2 - imgWidth / 2,
            state.canvasScaled.height / 2 - imgHeight / 2
        ];
        state.canvasScaledCtx.setTransform(1, 0, 0, 1, 0, 0);
        try {
            state.canvasScaledCtx.drawImage(img, x0, y0, imgWidth, imgHeight);
        } finally {
            state.canvasScaledCtx.scale(
                state.canvasScaled.width / state.canvas.width,
                state.canvasScaled.height / state.canvas.height
            );
        }
    }
};

const showToast = async (message: string, error = false, timeout = 3500) => {
    const toast = document.getElementById("toast");
    toast.classList.remove("error");
    if (error) toast.classList.add("error");
    toast.classList.add("visible");
    toast.textContent = message;
    if (state.toastTimeout) clearTimeout(state.toastTimeout);
    state.toastTimeout = setTimeout(() => {
        toast.classList.remove("visible");
        state.toastTimeout = null;
    }, timeout);
};

const showModal = async (
    message: string,
    title = "Alert"
): Promise<boolean> => {
    const modalContainer = document.getElementById("modal-container");
    const modalTitle = document.getElementById("modal-title");
    const modalText = document.getElementById("modal-text");
    modalContainer.classList.add("visible");
    modalTitle.textContent = title;
    modalText.innerHTML = message.replace(/\n/g, "<br/>");
    const result = (await new Promise((resolve) => {
        global.modalCallback = resolve;
    })) as boolean;
    return result;
};

const hideModal = async (result = true) => {
    const modalContainer = document.getElementById("modal-container");
    modalContainer.classList.remove("visible");
    if (global.modalCallback) global.modalCallback(result);
    global.modalCallback = null;
};

const setVersion = (value: string) => {
    document.getElementById("version").textContent = value;
};

const setEngine = (name: string, upper = true) => {
    name = upper ? name.toUpperCase() : name;
    document.getElementById("engine").textContent = name;
};

const setRom = (name: string, data: Uint8Array) => {
    state.romName = name;
    state.romData = data;
    state.romSize = data.length;
    document.getElementById("rom-name").textContent = name;
    document.getElementById("rom-size").textContent = String(data.length);
};

const setLogicFrequency = (value: number) => {
    if (value < 0) showToast("Invalid frequency value!", true);
    value = Math.max(value, 0);
    state.logicFrequency = value;
    document.getElementById("logic-frequency").textContent = String(value);
};

const setFps = (value: number) => {
    if (value < 0) showToast("Invalid FPS value!", true);
    value = Math.max(value, 0);
    state.fps = value;
    document.getElementById("fps-count").textContent = String(value);
};

const setBackground = (value: string) => {
    document.body.style.backgroundColor = `#${value}`;
    document.getElementById(
        "footer-background"
    ).style.backgroundColor = `#${value}f2`;
};

const toggleRunning = () => {
    if (state.paused) {
        resume();
    } else {
        pause();
    }
};

const pause = () => {
    state.paused = true;
    const buttonPause = document.getElementById("button-pause");
    const img = buttonPause.getElementsByTagName("img")[0];
    const span = buttonPause.getElementsByTagName("span")[0];
    buttonPause.classList.add("enabled");
    // @ts-ignore: ts(2580)
    img.src = require("./res/play.svg");
    span.textContent = "Resume";
};

const resume = () => {
    state.paused = false;
    state.nextTickTime = new Date().getTime();
    const buttonPause = document.getElementById("button-pause");
    const img = buttonPause.getElementsByTagName("img")[0];
    const span = buttonPause.getElementsByTagName("span")[0];
    buttonPause.classList.remove("enabled");
    // @ts-ignore: ts(2580)
    img.src = require("./res/pause.svg");
    span.textContent = "Pause";
};

const toggleWindow = () => {
    maximize();
};

const maximize = () => {
    const canvasContainer = document.getElementById("canvas-container");
    canvasContainer.classList.add("fullscreen");

    window.addEventListener("resize", crop);

    crop();
};

const minimize = () => {
    const canvasContainer = document.getElementById("canvas-container");
    const chipCanvas = document.getElementById("chip-canvas");
    canvasContainer.classList.remove("fullscreen");
    chipCanvas.style.width = null;
    chipCanvas.style.height = null;
    window.removeEventListener("resize", crop);
};

const crop = () => {
    const chipCanvas = document.getElementById("chip-canvas");

    // calculates the window ratio as this is fundamental to
    // determine the proper way to crop the fulscreen
    const windowRatio = window.innerWidth / window.innerHeight;

    // in case the window is wider (more horizontal than the base ratio)
    // this means that we must crop horizontaly
    if (windowRatio > DISPLAY_RATIO) {
        chipCanvas.style.width = `${
            window.innerWidth * (DISPLAY_RATIO / windowRatio)
        }px`;
        chipCanvas.style.height = `${window.innerHeight}px`;
    } else {
        chipCanvas.style.width = `${window.innerWidth}px`;
        chipCanvas.style.height = `${
            window.innerHeight * (windowRatio / DISPLAY_RATIO)
        }px`;
    }
};

const reset = () => {
    start({ engine: null });
};

const fetchRom = async (romPath: string): Promise<[string, Uint8Array]> => {
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
};

(async () => {
    await main();
})();
