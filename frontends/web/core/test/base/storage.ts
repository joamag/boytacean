import assert from "assert";

import {
    defaultStorage,
    LocalStorageAdapter,
    NullStorageAdapter
} from "../../ts/storage";

/**
 * The in memory items of the local storage stub, used to verify
 * the operations of the Web Storage API backed adapter.
 */
let items: Record<string, string> = {};

describe("Storage", function () {
    beforeEach(() => {
        items = {};
        global.localStorage = {
            getItem: (key: string) => (key in items ? items[key] : null),
            setItem: (key: string, value: string) => {
                items[key] = value;
            }
        } as unknown as Storage;
    });

    afterEach(() => {
        delete (global as { localStorage?: Storage }).localStorage;
    });

    describe("#getItem()", function () {
        it("should be able to read a stored value", () => {
            items["title"] = "value";
            const storage = new LocalStorageAdapter();
            assert.strictEqual(storage.getItem("title"), "value");
        });

        it("should be able to return null for a missing value", () => {
            const storage = new LocalStorageAdapter();
            assert.strictEqual(storage.getItem("missing"), null);
        });

        it("should be able to always return null when null backed", () => {
            const storage = new NullStorageAdapter();
            storage.setItem("title", "value");
            assert.strictEqual(storage.getItem("title"), null);
        });
    });

    describe("#setItem()", function () {
        it("should be able to store a value", () => {
            const storage = new LocalStorageAdapter();
            storage.setItem("title", "value");
            assert.strictEqual(items["title"], "value");
        });

        it("should be able to replace a stored value", () => {
            const storage = new LocalStorageAdapter();
            storage.setItem("title", "first");
            storage.setItem("title", "second");
            assert.strictEqual(items["title"], "second");
        });

        it("should be able to discard a value when null backed", () => {
            const storage = new NullStorageAdapter();
            storage.setItem("title", "value");
            assert.deepStrictEqual(items, {});
        });
    });

    describe("#defaultStorage()", function () {
        it("should be able to use the Web Storage API when available", () => {
            assert.ok(defaultStorage() instanceof LocalStorageAdapter);
        });

        it("should be able to fall back when no Web Storage API", () => {
            delete (global as { localStorage?: Storage }).localStorage;
            assert.ok(defaultStorage() instanceof NullStorageAdapter);
        });
    });
});
