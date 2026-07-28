import assert from "assert";

import { BoytaceanSystem } from "../../react/boytacean-context";
import { resolveSystem } from "../../react/boytacean-provider";

describe("BoytaceanProvider", function () {
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
});
