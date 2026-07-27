import assert from "assert";

import { Cartridge } from "boytacean";

import { GameBoyCore, releaseCore, SerialDevice } from "../../ts/gb";
import { NullStorageAdapter } from "../../ts/storage";

/**
 * The set of cores that have been built by a test, released after it
 * so that the active core does not leak into the following ones.
 */
let cores: GameBoyCore[] = [];

/**
 * Builds a core instance backed by a null storage adapter, so that
 * no Web Storage API is required to run the tests.
 *
 * @returns The core instance to be used in testing.
 */
const buildCore = (): GameBoyCore => {
    const core = new GameBoyCore({ storage: new NullStorageAdapter() });
    cores.push(core);
    return core;
};

/**
 * Runs the provided operation while capturing the warnings that are
 * sent to the console, restoring it afterwards.
 *
 * @param caller The operation to be run under capture.
 * @returns The complete set of captured warning messages.
 */
const captureWarnings = (caller: () => void): string[] => {
    const warnings: string[] = [];
    const warn = console.warn;
    console.warn = (message: string) => warnings.push(message);
    try {
        caller();
    } finally {
        console.warn = warn;
    }
    return warnings;
};

describe("GameBoyCore", function () {
    beforeEach(() => {
        cores = [];
    });

    afterEach(() => {
        cores.forEach((core) => releaseCore(core));
    });

    describe("#instance", function () {
        it("should be able to start without a Game Boy instance", () => {
            assert.strictEqual(buildCore().instance, null);
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
                null as unknown as Cartridge
            );
            assert.strictEqual(core.loaded, true);
            assert.strictEqual(core.loadedRomName, "POCKET-DEMO");
            assert.strictEqual(core.loadedRomSize, 3);
        });
    });

    describe("#engines", function () {
        it("should be able to provide the available engines", () => {
            assert.deepStrictEqual(buildCore().engines, ["auto", "cgb", "dmg"]);
        });
    });

    describe("#engine", function () {
        it("should be able to default to the auto engine", () => {
            assert.strictEqual(buildCore().engine, "auto");
        });
    });

    describe("#dimensions", function () {
        it("should be able to provide the Game Boy dimensions", () => {
            const dimensions = buildCore().dimensions;
            assert.strictEqual(dimensions.width, 160);
            assert.strictEqual(dimensions.height, 144);
        });
    });

    describe("#imageBuffer", function () {
        it("should be able to return an empty buffer when idle", () => {
            assert.strictEqual(buildCore().imageBuffer.length, 0);
        });
    });

    describe("#serialDevice", function () {
        it("should be able to default to the null device", () => {
            assert.strictEqual(buildCore().serialDevice, SerialDevice.Null);
        });

        it("should be able to change the serial device", () => {
            const core = buildCore();
            core.serialDevice = SerialDevice.Printer;
            assert.strictEqual(core.serialDevice, SerialDevice.Printer);
        });
    });

    describe("#keyPress()", function () {
        it("should be able to ignore an unknown key", () => {
            assert.doesNotThrow(() => buildCore().keyPress("Unknown"));
        });
    });

    describe("#keyLift()", function () {
        it("should be able to ignore an unknown key", () => {
            assert.doesNotThrow(() => buildCore().keyLift("Unknown"));
        });
    });

    describe("#boot()", function () {
        it("should be able to reject a boot without a ROM", async () => {
            await assert.rejects(buildCore().boot(), {
                message: "Unable to load initial ROM"
            });
        });
    });

    describe("#setActiveCore()", function () {
        it("should be able to create sequential cores silently", () => {
            const warnings = captureWarnings(() => {
                buildCore();
                buildCore();
            });
            assert.strictEqual(warnings.length, 0);
        });

        it("should be able to warn about a displaced loaded core", () => {
            const core = buildCore();
            core.setRom(
                "POCKET-DEMO",
                new Uint8Array([1]),
                null as unknown as Cartridge
            );
            const warnings = captureWarnings(() => buildCore());
            assert.strictEqual(warnings.length, 1);
            assert.match(warnings[0], /Multiple Game Boy cores/);
        });
    });

    describe("#releaseCore()", function () {
        it("should be able to release the active core", () => {
            const core = buildCore();
            core.setRom(
                "POCKET-DEMO",
                new Uint8Array([1]),
                null as unknown as Cartridge
            );
            releaseCore(core);
            const warnings = captureWarnings(() => buildCore());
            assert.strictEqual(warnings.length, 0);
        });

        it("should be able to ignore a superseded core", () => {
            const first = buildCore();
            const second = buildCore();
            releaseCore(first);
            second.setRom(
                "POCKET-DEMO",
                new Uint8Array([1]),
                null as unknown as Cartridge
            );
            const warnings = captureWarnings(() => buildCore());
            assert.strictEqual(warnings.length, 1);
        });
    });
});
