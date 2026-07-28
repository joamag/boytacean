import assert from "assert";

import { GbaRomInfo } from "boytacean";

import { GbaCore, releaseGbaCore } from "../../ts/gba";
import { NullStorageAdapter } from "../../ts/storage";

/**
 * The set of cores that have been built by a test, released after it
 * so that the active core does not leak into the following ones.
 */
let cores: GbaCore[] = [];

/**
 * Builds a core instance backed by a null storage adapter, so that
 * no Web Storage API is required to run the tests.
 *
 * @returns The core instance to be used in testing.
 */
const buildCore = (): GbaCore => {
    const core = new GbaCore({ storage: new NullStorageAdapter() });
    cores.push(core);
    return core;
};

describe("GbaCore", function () {
    beforeEach(() => {
        cores = [];
    });

    afterEach(() => {
        cores.forEach((core) => releaseGbaCore(core));
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
            core.setRom(
                "POCKET-DEMO",
                new Uint8Array([1, 2, 3]),
                null as unknown as GbaRomInfo
            );
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

    describe("#getVideoState()", function () {
        it("should be able to report a disabled PPU when idle", () => {
            assert.strictEqual(buildCore().getVideoState(), false);
        });
    });

    describe("#getAudioState()", function () {
        it("should be able to report a disabled APU when idle", () => {
            assert.strictEqual(buildCore().getAudioState(), false);
        });
    });
});
