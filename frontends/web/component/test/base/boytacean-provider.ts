import assert from "assert";
import { createRequire } from "module";

import { GameBoyCore, GbaCore, NullStorageAdapter } from "boytacean-core";
import { ComponentProps, ReactElement } from "react";

import {
    BoytaceanContextValue,
    BoytaceanSystem
} from "../../react/boytacean-context";
import {
    BoytaceanProvider,
    resolveSystem
} from "../../react/boytacean-provider";

const react = createRequire(__filename)("react") as Record<string, unknown>;

/**
 * Runs the provider with the same state, memo, and effect lifetime as
 * a renderer, while leaving the emulator loop under test control.
 *
 * @returns The render and unmount operations of the provider.
 */
const buildProvider = () => {
    const states: unknown[] = [];
    const refs: { current: unknown }[] = [];
    const memos: { deps: unknown[]; value: unknown }[] = [];
    const effects: { deps: unknown[]; cleanup?: () => void }[] = [];
    const storage = new NullStorageAdapter();
    const same = (a: unknown[], b: unknown[]) =>
        a.length === b.length &&
        a.every((value, index) => Object.is(value, b[index]));

    return {
        render: (props: ComponentProps<typeof BoytaceanProvider> = {}) => {
            let stateIndex = 0;
            let refIndex = 0;
            let memoIndex = 0;
            let effectIndex = 0;
            const pending: (() => void)[] = [];
            const original = { ...react };
            react.useState = (initial: unknown) => {
                const index = stateIndex++;
                if (!(index in states)) states[index] = initial;
                return [
                    states[index],
                    (value: unknown) => {
                        states[index] = value;
                    }
                ];
            };
            react.useRef = (initial: unknown) => {
                const index = refIndex++;
                if (!refs[index]) refs[index] = { current: initial };
                return refs[index];
            };
            react.useMemo = (factory: () => unknown, deps: unknown[]) => {
                const index = memoIndex++;
                if (!memos[index] || !same(memos[index].deps, deps)) {
                    memos[index] = { deps, value: factory() };
                }
                return memos[index].value;
            };
            react.useEffect = (
                effect: () => (() => void) | undefined,
                deps: unknown[]
            ) => {
                const index = effectIndex++;
                if (!effects[index] || !same(effects[index].deps, deps)) {
                    pending.push(() => {
                        effects[index]?.cleanup?.();
                        effects[index] = { deps, cleanup: effect() };
                    });
                }
            };
            try {
                const result = BoytaceanProvider({
                    storage,
                    ...props
                }) as ReactElement<{ value: BoytaceanContextValue }>;
                pending.forEach((effect) => effect());
                return result.props.value;
            } finally {
                Object.assign(react, original);
            }
        },
        unmount: () => effects.forEach((effect) => effect.cleanup?.())
    };
};

describe("BoytaceanProvider", function () {
    let provider: ReturnType<typeof buildProvider>;
    let starts: (GameBoyCore | GbaCore)[];
    let stops: (GameBoyCore | GbaCore)[];
    let errors: Error[];
    let initialize: (core: GameBoyCore | GbaCore) => Promise<void>;
    let start: (core: GameBoyCore | GbaCore) => Promise<void>;
    let restore: (() => void)[];

    beforeEach(() => {
        provider = buildProvider();
        starts = [];
        stops = [];
        errors = [];
        initialize = async () => {};
        start = async (core) => {
            core.trigger("booted");
        };
        const originalError = console.error;
        console.error = (error: Error) => errors.push(error);
        restore = [
            () => {
                console.error = originalError;
            }
        ];
        for (const prototype of [GameBoyCore.prototype, GbaCore.prototype]) {
            const original = {
                init: prototype.init,
                start: prototype.start,
                stop: prototype.stop
            };
            prototype.init = async function () {
                await initialize(this);
            };
            prototype.start = async function () {
                starts.push(this);
                await start(this);
            };
            prototype.stop = async function () {
                stops.push(this);
            };
            restore.push(() => Object.assign(prototype, original));
        }
    });

    afterEach(() => {
        provider.unmount();
        restore.forEach((callback) => callback());
    });

    describe("#resolveSystem()", function () {
        it("should be able to fall back to the Game Boy system", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto),
                BoytaceanSystem.GameBoy
            );
        });

        it("should be able to infer the Game Boy system from the ROM", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto, "/roms/pocket.gb"),
                BoytaceanSystem.GameBoy
            );
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto, "/roms/pocket.gbc"),
                BoytaceanSystem.GameBoy
            );
        });

        it("should be able to infer the GBA system from the ROM", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto, "/roms/pocket.gba"),
                BoytaceanSystem.GameBoyAdvance
            );
        });

        it("should be able to ignore the case of the ROM extension", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto, "/roms/POCKET.GBA"),
                BoytaceanSystem.GameBoyAdvance
            );
        });

        it("should be able to ignore the query string of the ROM", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.Auto, "/roms/pocket.gba?v=1"),
                BoytaceanSystem.GameBoyAdvance
            );
        });

        it("should be able to honour an explicit system", () => {
            assert.strictEqual(
                resolveSystem(BoytaceanSystem.GameBoy, "/roms/pocket.gba"),
                BoytaceanSystem.GameBoy
            );
            assert.strictEqual(
                resolveSystem(
                    BoytaceanSystem.GameBoyAdvance,
                    "/roms/pocket.gb"
                ),
                BoytaceanSystem.GameBoyAdvance
            );
        });
    });

    describe("#BoytaceanProvider()", function () {
        it("should be able to remain idle without a ROM", async () => {
            const value = provider.render();
            await new Promise((resolve) => setImmediate(resolve));
            assert.ok(value.core instanceof GameBoyCore);
            assert.strictEqual(starts.length, 0);
            assert.deepStrictEqual(errors, []);
            assert.strictEqual(provider.render(), value);
        });

        it("should be able to select the initial core and palette", async () => {
            const gb = provider.render({ rom: "pocket.gb", palette: "basic" });
            await new Promise((resolve) => setImmediate(resolve));
            assert.ok(gb.core instanceof GameBoyCore);
            assert.strictEqual(gb.core.palette, "basic");
            const gba = provider.render({ rom: "game.gba", palette: "basic" });
            await new Promise((resolve) => setImmediate(resolve));
            assert.ok(gba.core instanceof GbaCore);
            assert.deepStrictEqual(starts, [gb.core, gba.core]);
            assert.ok(stops.includes(gb.core));
        });

        it("should be able to report initial startup errors", async () => {
            const error = new Error("Unable to initialize WASM");
            initialize = async () => {
                throw error;
            };
            provider.render({ rom: "pocket.gb" });
            await new Promise((resolve) => setImmediate(resolve));
            assert.deepStrictEqual(errors, [error]);
            assert.strictEqual(starts.length, 0);
        });

        it("should be able to stop initialization on unmount", async () => {
            let ready!: () => void;
            initialize = () =>
                new Promise((resolve) => {
                    ready = resolve;
                });
            const value = provider.render({ rom: "pocket.gb" });
            provider.unmount();
            ready();
            await new Promise((resolve) => setImmediate(resolve));
            assert.ok(stops.includes(value.core));
            assert.strictEqual(starts.length, 0);
        });
    });

    describe("#loadRom()", function () {
        it("should be able to switch between GB and GBA at runtime", async () => {
            let value = provider.render();
            for (const name of ["first.gb", "second.gba", "third.gb"]) {
                const previous = value.core;
                const loaded = value.loadRom(name, new Uint8Array([1, 2, 3]));
                value = provider.render();
                await loaded;
                assert.strictEqual(
                    value.core instanceof GbaCore,
                    name.endsWith(".gba")
                );
                assert.strictEqual(
                    value.system,
                    name.endsWith(".gba")
                        ? BoytaceanSystem.GameBoyAdvance
                        : BoytaceanSystem.GameBoy
                );
                assert.strictEqual(value.core.loadedRomName, name);
                assert.strictEqual(value.core.loadedRomSize, 3);
                assert.ok(stops.includes(previous));
            }
            assert.strictEqual(starts.length, 3);
        });

        it("should be able to honour the explicit system on runtime loads", async () => {
            const props = { system: BoytaceanSystem.GameBoy };
            let value = provider.render(props);
            const loaded = value.loadRom("game.gba", new Uint8Array([1]));
            value = provider.render(props);
            await loaded;
            assert.ok(value.core instanceof GameBoyCore);
            assert.strictEqual(value.system, BoytaceanSystem.GameBoy);
        });

        it("should be able to reject superseded loads before rendering", async () => {
            const value = provider.render();
            const first = assert.rejects(
                value.loadRom("first.gba", new Uint8Array([1])),
                /cancelled/
            );
            const second = value.loadRom("second.gb", new Uint8Array([2]));
            const next = provider.render();
            await Promise.all([first, second]);
            assert.strictEqual(next.core.loadedRomName, "second.gb");
            assert.deepStrictEqual(starts, [next.core]);
        });

        it("should be able to reject a load when unmounted before rendering", async () => {
            const value = provider.render();
            const loaded = assert.rejects(
                value.loadRom("game.gba", new Uint8Array([1])),
                /cancelled/
            );
            provider.unmount();
            await loaded;
            assert.strictEqual(starts.length, 0);
        });

        it("should be able to ignore stale initialization after another load", async () => {
            let ready!: () => void;
            const value = provider.render();
            await new Promise((resolve) => setImmediate(resolve));
            initialize = (core) =>
                core instanceof GbaCore
                    ? new Promise((resolve) => {
                          ready = resolve;
                      })
                    : Promise.resolve();
            const first = assert.rejects(
                value.loadRom("first.gba", new Uint8Array([1])),
                /cancelled/
            );
            const pending = provider.render();
            const second = pending.loadRom("second.gb", new Uint8Array([2]));
            const next = provider.render();
            ready();
            await Promise.all([first, second]);
            assert.deepStrictEqual(starts, [next.core]);
            assert.ok(stops.includes(pending.core));
        });

        it("should be able to reject a failed runtime boot", async () => {
            const error = new Error("Invalid ROM");
            start = async () => {
                throw error;
            };
            const value = provider.render();
            const loaded = assert.rejects(
                value.loadRom("game.gba", new Uint8Array([1])),
                error
            );
            provider.render();
            await loaded;
            assert.deepStrictEqual(errors, []);
        });

        it("should be able to replace a pending runtime load with a ROM prop", async () => {
            start = async () => {};
            const value = provider.render();
            const loaded = assert.rejects(
                value.loadRom("game.gba", new Uint8Array([1])),
                /cancelled/
            );
            const pending = provider.render();
            await new Promise((resolve) => setImmediate(resolve));
            const next = provider.render({ rom: "pocket.gb" });
            pending.core.trigger("booted");
            await loaded;
            assert.ok(next.core instanceof GameBoyCore);
            assert.strictEqual(next.core.loadedRomName, null);
        });

        it("should be able to return to an earlier ROM prop without reviving a runtime load", async () => {
            const props = { rom: "pocket.gb" };
            const value = provider.render(props);
            const loaded = value.loadRom("game.gba", new Uint8Array([1]));
            provider.render(props);
            await loaded;
            provider.render({ rom: "another.gb" });
            const next = provider.render(props);
            await new Promise((resolve) => setImmediate(resolve));
            assert.ok(next.core instanceof GameBoyCore);
            assert.strictEqual(next.core.loadedRomName, null);
        });

        it("should be able to ignore a boot error after the provider is unmounted", async () => {
            let fail!: (error: Error) => void;
            start = () =>
                new Promise((_resolve, reject) => {
                    fail = reject;
                });
            const value = provider.render();
            const loaded = assert.rejects(
                value.loadRom("game.gba", new Uint8Array([1])),
                /cancelled/
            );
            provider.render();
            await new Promise((resolve) => setImmediate(resolve));
            provider.unmount();
            fail(new Error("Late boot failure"));
            await loaded;
            await new Promise((resolve) => setImmediate(resolve));
            assert.deepStrictEqual(errors, []);
        });
    });
});
