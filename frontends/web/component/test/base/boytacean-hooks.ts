import assert from "assert";
import { createRequire } from "module";

import { Cartridge } from "boytacean";
import { GameBoyCore, NullStorageAdapter, releaseCore } from "boytacean-core";

/**
 * The React module as a mutable record, so that its exports can be
 * replaced by the stubs of the hook runner.
 *
 * The CommonJS module is used explicitly, as the bindings of an ES
 * module are read only and cannot be reassigned.
 */
const react = createRequire(__filename)("react") as Record<string, unknown>;

import {
    BoytaceanStats,
    BoytaceanStatus,
    useBoytaceanStats,
    useBoytaceanStatus
} from "../../react/boytacean-hooks";

/**
 * The result of a hook execution, exposing the arguments that the
 * hook handed over to `useSyncExternalStore`.
 */
type HookResult<T> = {
    subscribe: (onChange: () => void) => () => void;
    getSnapshot: () => T;
};

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
 * Loads a synthetic ROM into the provided core, so that it reports
 * itself as loaded without requiring the WASM module.
 *
 * @param core The core to load the ROM into.
 * @param name The name of the ROM to be loaded.
 */
const loadRom = (core: GameBoyCore, name = "POCKET-DEMO") => {
    core.setRom(name, new Uint8Array([1, 2]), null as unknown as Cartridge);
};

/**
 * Runs the provided hook outside of a renderer, by replaying the
 * calls that `useSyncExternalStore` would perform and capturing the
 * arguments that the hook provides to it.
 *
 * This allows the verification of the snapshot caching, which is the
 * behaviour that prevents an infinite render loop, without pulling a
 * full renderer into the test suite.
 *
 * @param hook The hook to be executed.
 * @param core The core to be provided through the context.
 * @returns The subscribe and snapshot functions of the hook.
 */
const runHook = <T>(hook: () => T, core: GameBoyCore): HookResult<T> => {
    const refs: { current: unknown }[] = [];
    let index = 0;
    let result: HookResult<T> | null = null;

    const original = { ...react };

    const mutable = react;
    mutable.useCallback = (callback: unknown) => callback;
    mutable.useContext = () => ({ core: core });
    mutable.useRef = (initial: unknown) => {
        if (refs[index] === undefined) refs[index] = { current: initial };
        return refs[index++];
    };
    mutable.useSyncExternalStore = (
        subscribe: (onChange: () => void) => () => void,
        getSnapshot: () => T
    ) => {
        result = { subscribe: subscribe, getSnapshot: getSnapshot };
        return getSnapshot();
    };

    try {
        index = 0;
        hook();
        return result as unknown as HookResult<T>;
    } finally {
        Object.assign(mutable, original);
    }
};

describe("BoytaceanHooks", function () {
    beforeEach(() => {
        cores = [];
    });

    afterEach(() => {
        cores.forEach((core) => releaseCore(core));
    });

    describe("#useBoytaceanStatus()", function () {
        it("should be able to report an unloaded core", () => {
            const core = buildCore();
            const { getSnapshot } = runHook<BoytaceanStatus>(
                () => useBoytaceanStatus(),
                core
            );
            const status = getSnapshot();
            assert.strictEqual(status.booted, false);
            assert.strictEqual(status.romName, undefined);
        });

        it("should be able to report a loaded core", () => {
            const core = buildCore();
            loadRom(core);
            const { getSnapshot } = runHook<BoytaceanStatus>(
                () => useBoytaceanStatus(),
                core
            );
            const status = getSnapshot();
            assert.strictEqual(status.booted, true);
            assert.strictEqual(status.romName, "POCKET-DEMO");
        });

        it("should be able to keep a stable snapshot reference", () => {
            const core = buildCore();
            const { getSnapshot } = runHook<BoytaceanStatus>(
                () => useBoytaceanStatus(),
                core
            );
            assert.strictEqual(getSnapshot(), getSnapshot());
        });

        it("should be able to replace the snapshot on a change", () => {
            const core = buildCore();
            const { getSnapshot } = runHook<BoytaceanStatus>(
                () => useBoytaceanStatus(),
                core
            );
            const before = getSnapshot();
            loadRom(core);
            const after = getSnapshot();
            assert.notStrictEqual(before, after);
            assert.strictEqual(after.romName, "POCKET-DEMO");
        });

        it("should be able to subscribe to the booted event", () => {
            const core = buildCore();
            const { subscribe } = runHook<BoytaceanStatus>(
                () => useBoytaceanStatus(),
                core
            );
            let changes = 0;
            const unsubscribe = subscribe(() => changes++);
            core.trigger("booted");
            assert.strictEqual(changes, 1);
            unsubscribe();
            core.trigger("booted");
            assert.strictEqual(changes, 1);
        });
    });

    describe("#useBoytaceanStats()", function () {
        it("should be able to report the emulation statistics", () => {
            const core = buildCore();
            const { getSnapshot } = runHook<BoytaceanStats>(
                () => useBoytaceanStats(),
                core
            );
            const stats = getSnapshot();
            assert.strictEqual(stats.framerate, 0);
            assert.strictEqual(stats.cyclerate, 0);
            assert.strictEqual(stats.emulationSpeed, 0);
        });

        it("should be able to keep a stable snapshot reference", () => {
            const core = buildCore();
            const { getSnapshot } = runHook<BoytaceanStats>(
                () => useBoytaceanStats(),
                core
            );
            assert.strictEqual(getSnapshot(), getSnapshot());
        });
    });
});
