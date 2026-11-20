import assert from "assert";
import { readFile } from "fs/promises";
import { resolve } from "path";
import { pathToFileURL } from "url";

import { bufferToBase64, logger, LogLevel } from "emukit/logic";

import { GbaCore, releaseGbaCore } from "../../ts/gba";
import { NullStorageAdapter, StorageAdapter } from "../../ts/storage";

/**
 * The directory of the GBA test ROMs, used to exercise the complete
 * boot, tick and persistence cycle of the core.
 */
const ROMS_PATH = resolve(
    __dirname,
    "../../../../../res/roms.gba/test/jsmolka_gba-tests"
);

/**
 * The URL of the test ROM backed by a battery (SRAM).
 */
const SRAM_ROM_URL = pathToFileURL(resolve(ROMS_PATH, "sram.gba")).href;

/**
 * The URL of the test ROM without any kind of save support.
 */
const NONE_ROM_URL = pathToFileURL(resolve(ROMS_PATH, "none.gba")).href;

/**
 * The number of cycles in a complete frame of the GBA.
 */
const FRAME_CYCLES = 280896;

/**
 * The number of cycles in between two flush operations of the RAM
 * to the storage adapter (5 seconds of emulation).
 */
const FLUSH_CYCLES = 16777216 * 5;

/**
 * The set of cores that have been built by a test, released after it
 * so that the active core does not leak into the following ones.
 */
let cores: GbaCore[] = [];

/**
 * Builds a core instance backed by the provided storage adapter,
 * defaulting to a null one, so that no Web Storage API is required
 * to run the tests.
 *
 * @param storage The storage adapter to be used by the core.
 * @returns The core instance to be used in testing.
 */
const buildCore = (
    storage: StorageAdapter = new NullStorageAdapter()
): GbaCore => {
    const core = new GbaCore({ storage: storage });
    cores.push(core);
    return core;
};

/**
 * Builds a storage adapter backed by the provided in memory items,
 * allowing the persistence operations of the core to be verified.
 *
 * @param items The in memory items to be used as the storage.
 * @returns The storage adapter backed by the items.
 */
const buildStorage = (items: Record<string, string>): StorageAdapter => ({
    getItem: (key: string) => (key in items ? items[key] : null),
    setItem: (key: string, value: string) => {
        items[key] = value;
    }
});

/**
 * Fetches the file at the provided URL from the local file system,
 * standing in for the Fetch API of Node.js, that does not support
 * file URLs, so that both the WASM binary and the ROMs are loaded.
 *
 * @param input The URL of the file to be fetched.
 * @returns The response with the contents of the file or a not found
 * one in case the file does not exist.
 */
const fetchFile = async (input: string | URL | Request): Promise<Response> => {
    try {
        const data = await readFile(new URL(String(input)));
        return new Response(new Uint8Array(data), {
            headers: { "Content-Type": "application/wasm" }
        });
    } catch (err) {
        return new Response(null, { status: 404 });
    }
};

/**
 * Reads the test ROM at the provided URL from the local file system.
 *
 * @param url The URL of the test ROM to be read.
 * @returns The contents of the test ROM.
 */
const readRom = async (url: string): Promise<Uint8Array> =>
    new Uint8Array(await readFile(new URL(url)));

/**
 * Runs the provided operation while capturing the warnings that are
 * sent to the console, restoring it afterwards.
 *
 * @param caller The operation to be run under capture.
 * @returns The complete set of captured warning messages.
 */
const captureWarnings = async (
    caller: () => Promise<void>
): Promise<string[]> => {
    const warnings: string[] = [];
    const warn = console.warn;
    console.warn = (message: string) => warnings.push(message);
    try {
        await caller();
    } finally {
        console.warn = warn;
    }
    return warnings;
};

describe("GbaCore", function () {
    let fetch: typeof global.fetch;
    let level: LogLevel;

    before(async () => {
        // replaces the Fetch API so that the WASM module is loaded
        // from the local file system, silencing the information that
        // is logged on every boot of the core
        fetch = global.fetch;
        level = logger.level;
        global.fetch = fetchFile as typeof global.fetch;
        logger.level = LogLevel.WARN;
        await buildCore().init();
    });

    after(() => {
        global.fetch = fetch;
        logger.level = level;
    });

    beforeEach(() => {
        cores = [];
    });

    afterEach(() => {
        cores.forEach((core) => releaseGbaCore(core));
    });

    describe("#tick()", function () {
        this.timeout(10000);

        it("should be able to ignore a tick without a GBA instance", async () => {
            const core = buildCore();
            let ticks = 0;
            core.bind("tick", () => ticks++);
            await core.tick({ cycles: FRAME_CYCLES });
            assert.strictEqual(ticks, 0);
        });

        it("should be able to clock the frames of a ROM", async () => {
            const core = buildCore();
            await core.boot({
                romName: "none.gba",
                romData: await readRom(NONE_ROM_URL)
            });
            let frames = 0;
            let cycles = 0;
            core.bind("frame", (_owner, params) => {
                frames += (params as { count: number }).count;
            });
            core.bind("tick", (_owner, params) => {
                cycles += (params as { cycles: number }).cycles;
            });
            await core.tick({ cycles: FRAME_CYCLES * 2 });
            assert.ok(frames >= 1);
            assert.ok(cycles >= FRAME_CYCLES * 2);
            assert.strictEqual(core.imageBuffer.length, 240 * 160 * 3);
        });

        it("should be able to flush the RAM of a battery backed ROM", async () => {
            const items: Record<string, string> = {};
            const core = buildCore(buildStorage(items));
            await core.boot({
                romName: "sram.gba",
                romData: await readRom(SRAM_ROM_URL)
            });
            await core.tick({ cycles: FRAME_CYCLES });
            assert.deepStrictEqual(Object.keys(items), []);
            await core.tick({ cycles: FLUSH_CYCLES });
            assert.deepStrictEqual(Object.keys(items), ["gba:1337:GBA Tests"]);
            assert.strictEqual(
                items["gba:1337:GBA Tests"],
                bufferToBase64(core.instance?.ram_data_eager() ?? [])
            );
        });

        it("should be able to survive a failing storage adapter", async () => {
            const core = buildCore({
                getItem: () => null,
                setItem: () => {
                    throw new Error("Quota exceeded");
                }
            });
            await core.boot({
                romName: "sram.gba",
                romData: await readRom(SRAM_ROM_URL)
            });
            const warnings = await captureWarnings(() =>
                core.tick({ cycles: FLUSH_CYCLES })
            );
            assert.strictEqual(warnings.length, 1);
            assert.match(warnings[0], /Failed to save RAM data/);
            await core.tick({ cycles: FRAME_CYCLES });
        });
    });

    describe("#hardReset()", function () {
        it("should be able to reboot the ROM on a new instance", async () => {
            const core = buildCore();
            await core.boot({
                romName: "sram.gba",
                romData: await readRom(SRAM_ROM_URL)
            });
            const instance = core.instance;
            await core.hardReset();
            assert.ok(core.instance);
            assert.notStrictEqual(core.instance, instance);
            assert.strictEqual(core.loadedRomName, "GBA Tests");
            assert.strictEqual(core.loadedRomInfo?.game_code(), "1337");
        });
    });

    describe("#boot()", function () {
        it("should be able to reject a boot without a ROM", async () => {
            await assert.rejects(buildCore().boot(), {
                message: "Unable to load initial ROM"
            });
        });

        it("should be able to boot a ROM from its data", async () => {
            const core = buildCore();
            const romData = await readRom(SRAM_ROM_URL);
            let booted = 0;
            core.bind("booted", () => booted++);
            await core.boot({ romName: "sram.gba", romData: romData });
            assert.ok(core.instance);
            assert.strictEqual(core.loaded, true);
            assert.strictEqual(core.loadedRomName, "GBA Tests");
            assert.strictEqual(core.loadedRomSize, romData.length);
            assert.strictEqual(core.loadedRomInfo?.game_code(), "1337");
            assert.strictEqual(booted, 1);
        });

        it("should be able to boot a ROM from a path", async () => {
            const core = buildCore();
            await core.boot({ loadRom: true, romPath: SRAM_ROM_URL });
            assert.strictEqual(core.loadedRomName, "GBA Tests");
            assert.strictEqual(core.loadedRomInfo?.game_code(), "1337");
        });

        it("should be able to reject a ROM that can not be fetched", async () => {
            const core = buildCore();
            const romPath = pathToFileURL(
                resolve(ROMS_PATH, "missing.gba")
            ).href;
            await assert.rejects(
                core.boot({ loadRom: true, romPath: romPath }),
                {
                    message: `Unable to load ROM from ${romPath} (404)`
                }
            );
            assert.strictEqual(core.loaded, false);
        });

        it("should be able to reuse the GBA instance", async () => {
            const core = buildCore();
            const romData = await readRom(SRAM_ROM_URL);
            await core.boot({ romName: "sram.gba", romData: romData });
            const instance = core.instance;
            await core.boot({ romName: "sram.gba", romData: romData });
            assert.strictEqual(core.instance, instance);
            await core.boot({
                romName: "sram.gba",
                romData: romData,
                reuse: false
            });
            assert.notStrictEqual(core.instance, instance);
        });

        it("should be able to restore the RAM of a battery backed ROM", async () => {
            const core = buildCore(
                buildStorage({
                    "gba:1337:GBA Tests": bufferToBase64(
                        new Uint8Array([0x12, 0x34, 0x56, 0x78])
                    )
                })
            );
            await core.boot({
                romName: "sram.gba",
                romData: await readRom(SRAM_ROM_URL)
            });
            const ramData = core.instance?.ram_data_eager() ?? [];
            assert.strictEqual(ramData.length, 32768);
            assert.deepStrictEqual(
                Array.from(ramData.slice(0, 5)),
                [0x12, 0x34, 0x56, 0x78, 0xff]
            );
        });

        it("should be able to ignore the RAM of a Game Boy ROM with the same title", async () => {
            const core = buildCore(
                buildStorage({
                    "GBA Tests": bufferToBase64(
                        new Uint8Array([0x12, 0x34, 0x56, 0x78])
                    )
                })
            );
            await core.boot({
                romName: "sram.gba",
                romData: await readRom(SRAM_ROM_URL)
            });
            const ramData = core.instance?.ram_data_eager() ?? [];
            assert.notDeepStrictEqual(
                Array.from(ramData.slice(0, 4)),
                [0x12, 0x34, 0x56, 0x78]
            );
        });

        it("should be able to reject an invalid ROM", async () => {
            const core = buildCore();
            await assert.rejects(
                core.boot({
                    romName: "invalid.gba",
                    romData: new Uint8Array([1, 2, 3])
                })
            );
            assert.strictEqual(core.loaded, false);
        });
    });

    describe("#setRom()", function () {
        it("should be able to stage a ROM before its header is parsed", () => {
            const core = buildCore();
            core.setRom("game.gba", new Uint8Array([1, 2, 3]), null);
            assert.strictEqual(core.loadedRomName, "game.gba");
            assert.strictEqual(core.loadedRomSize, 3);
            assert.strictEqual(core.loadedRomInfo, null);
            assert.strictEqual(core.instance, null);
        });
    });

    describe("#instance", function () {
        it("should be able to start without a GBA instance", () => {
            assert.strictEqual(buildCore().instance, null);
        });
    });

    describe("#engines", function () {
        it("should be able to provide the available engines", () => {
            assert.deepStrictEqual(buildCore().engines, ["gba"]);
        });
    });

    describe("#engine", function () {
        it("should be able to default to the gba engine", () => {
            assert.strictEqual(buildCore().engine, "gba");
        });
    });

    describe("#dimensions", function () {
        it("should be able to provide the GBA dimensions", () => {
            const dimensions = buildCore().dimensions;
            assert.strictEqual(dimensions.width, 240);
            assert.strictEqual(dimensions.height, 160);
        });
    });

    describe("#imageBuffer", function () {
        it("should be able to return an empty buffer when idle", () => {
            assert.strictEqual(buildCore().imageBuffer.length, 0);
        });
    });

    describe("#audioSpecs", function () {
        it("should be able to fall back to the default specs", () => {
            const audioSpecs = buildCore().audioSpecs;
            assert.strictEqual(audioSpecs.samplingRate, 32768);
            assert.strictEqual(audioSpecs.channels, 2);
        });
    });

    describe("#audioBuffer", function () {
        it("should be able to return empty streams when idle", () => {
            const audioBuffer = buildCore().audioBuffer;
            assert.strictEqual(audioBuffer.length, 2);
            assert.strictEqual(audioBuffer[0].length, 0);
            assert.strictEqual(audioBuffer[1].length, 0);
        });
    });

    describe("#frequency", function () {
        it("should be able to default to the GBA frequency", () => {
            assert.strictEqual(buildCore().frequency, 16777216);
        });

        it("should be able to change the frequency", () => {
            const core = buildCore();
            core.frequency = 8388608;
            assert.strictEqual(core.frequency, 8388608);
        });

        it("should be able to clamp a negative frequency", () => {
            const core = buildCore();
            core.frequency = -1;
            assert.strictEqual(core.frequency, 0);
        });
    });

    describe("#loadedRomName", function () {
        it("should be able to start without a ROM name", () => {
            assert.strictEqual(buildCore().loadedRomName, null);
        });
    });

    describe("#loadedRomSize", function () {
        it("should be able to start with a zeroed ROM size", () => {
            assert.strictEqual(buildCore().loadedRomSize, 0);
        });
    });

    describe("#loaded", function () {
        it("should be able to start with no ROM loaded", () => {
            assert.strictEqual(buildCore().loaded, false);
        });

        it("should be able to detect a loaded ROM", () => {
            const core = buildCore();
            core.setRom("POCKET-DEMO", new Uint8Array([1, 2, 3]), null);
            assert.strictEqual(core.loaded, true);
            assert.strictEqual(core.loadedRomName, "POCKET-DEMO");
            assert.strictEqual(core.loadedRomSize, 3);
        });
    });

    describe("#loadedRomInfo", function () {
        it("should be able to start without ROM information", () => {
            assert.strictEqual(buildCore().loadedRomInfo, null);
        });
    });

    describe("#keyPress()", function () {
        it("should be able to ignore an unknown key", () => {
            assert.doesNotThrow(() => buildCore().keyPress("Unknown"));
        });

        it("should be able to ignore a key with no instance", () => {
            assert.doesNotThrow(() => buildCore().keyPress("Start"));
        });
    });

    describe("#keyLift()", function () {
        it("should be able to ignore an unknown key", () => {
            assert.doesNotThrow(() => buildCore().keyLift("Unknown"));
        });

        it("should be able to ignore a key with no instance", () => {
            assert.doesNotThrow(() => buildCore().keyLift("Start"));
        });
    });

    describe("#pauseVideo()", function () {
        it("should be able to disable the PPU", async () => {
            const core = buildCore();
            await core.boot({ loadRom: true, romPath: NONE_ROM_URL });
            core.pauseVideo();
            assert.strictEqual(core.getVideoState(), false);
        });
    });

    describe("#resumeVideo()", function () {
        it("should be able to enable the PPU", async () => {
            const core = buildCore();
            await core.boot({ loadRom: true, romPath: NONE_ROM_URL });
            core.pauseVideo();
            core.resumeVideo();
            assert.strictEqual(core.getVideoState(), true);
        });
    });

    describe("#getVideoState()", function () {
        it("should be able to report a disabled PPU when idle", () => {
            assert.strictEqual(buildCore().getVideoState(), false);
        });
    });

    describe("#pauseAudio()", function () {
        it("should be able to disable the APU", async () => {
            const core = buildCore();
            await core.boot({ loadRom: true, romPath: NONE_ROM_URL });
            core.pauseAudio();
            assert.strictEqual(core.getAudioState(), false);
        });
    });

    describe("#resumeAudio()", function () {
        it("should be able to enable the APU", async () => {
            const core = buildCore();
            await core.boot({ loadRom: true, romPath: NONE_ROM_URL });
            core.pauseAudio();
            core.resumeAudio();
            assert.strictEqual(core.getAudioState(), true);
        });
    });

    describe("#getAudioState()", function () {
        it("should be able to report a disabled APU when idle", () => {
            assert.strictEqual(buildCore().getAudioState(), false);
        });
    });
});
