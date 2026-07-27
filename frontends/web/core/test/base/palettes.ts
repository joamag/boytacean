import assert from "assert";

import { PALETTES, PALETTES_MAP } from "../../ts/palettes";

describe("Palettes", function () {
    describe("#PALETTES", function () {
        it("should be able to provide the complete set of palettes", () => {
            assert.ok(PALETTES.length > 0);
        });

        it("should be able to provide four colors per palette", () => {
            PALETTES.forEach((palette) => {
                assert.strictEqual(palette.colors.length, 4);
            });
        });

        it("should be able to provide unique palette names", () => {
            const names = PALETTES.map((palette) => palette.name);
            assert.strictEqual(new Set(names).size, names.length);
        });

        it("should be able to provide valid hexadecimal colors", () => {
            PALETTES.forEach((palette) => {
                palette.colors.forEach((color) => {
                    assert.match(color, /^[0-9a-f]{6}$/);
                });
            });
        });
    });

    describe("#PALETTES_MAP", function () {
        it("should be able to index every palette by name", () => {
            assert.strictEqual(
                Object.keys(PALETTES_MAP).length,
                PALETTES.length
            );
            PALETTES.forEach((palette) => {
                assert.strictEqual(PALETTES_MAP[palette.name], palette);
            });
        });
    });
});
