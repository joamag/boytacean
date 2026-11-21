import assert from "assert";

import { KEYS_MAP } from "../../react/boytacean-keyboard";

/**
 * The complete set of game pad keys that the emulator is able to
 * receive, used to validate the default key mapping.
 *
 * The shoulder keys are only present in the Game Boy Advance, but
 * are still mapped as the same mapping drives both systems.
 */
const PAD_KEYS = [
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Start",
    "Select",
    "A",
    "B",
    "L",
    "R"
];

describe("BoytaceanKeyboard", function () {
    describe("#KEYS_MAP", function () {
        it("should be able to map only valid game pad keys", () => {
            Object.values(KEYS_MAP).forEach((key) => {
                assert.ok(PAD_KEYS.includes(key));
            });
        });

        it("should be able to cover every game pad key", () => {
            const mapped = new Set(Object.values(KEYS_MAP));
            PAD_KEYS.forEach((key) => {
                assert.ok(mapped.has(key));
            });
        });

        it("should be able to map the directional keys directly", () => {
            assert.strictEqual(KEYS_MAP["ArrowUp"], "ArrowUp");
            assert.strictEqual(KEYS_MAP["ArrowDown"], "ArrowDown");
            assert.strictEqual(KEYS_MAP["ArrowLeft"], "ArrowLeft");
            assert.strictEqual(KEYS_MAP["ArrowRight"], "ArrowRight");
        });

        it("should be able to map both cases of the action keys", () => {
            assert.strictEqual(KEYS_MAP["a"], "A");
            assert.strictEqual(KEYS_MAP["A"], "A");
            assert.strictEqual(KEYS_MAP["s"], "B");
            assert.strictEqual(KEYS_MAP["S"], "B");
        });

        it("should be able to map both cases of the shoulder keys", () => {
            assert.strictEqual(KEYS_MAP["q"], "L");
            assert.strictEqual(KEYS_MAP["Q"], "L");
            assert.strictEqual(KEYS_MAP["w"], "R");
            assert.strictEqual(KEYS_MAP["W"], "R");
        });
    });
});
