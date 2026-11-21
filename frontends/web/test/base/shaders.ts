import assert from "assert";
import { readFile } from "fs/promises";
import { resolve } from "path";
import { fileURLToPath, pathToFileURL } from "url";

import type { loadShader as LoadShader } from "../../ts/shaders";

/**
 * The directory of the master shader of the web front-end.
 */
const WEB_SHADERS_PATH = resolve(__dirname, "../../res/shaders");

/**
 * The directory of the filters, shared with the SDL front-end.
 */
const SDL_SHADERS_PATH = resolve(__dirname, "../../../sdl/res/shaders");

/**
 * The names of the filters that can be used by the shader.
 */
const FILTERS = ["bilinear", "crt", "passthrough", "smooth_bilinear"];

/**
 * Fetches the file at the provided URL from the local file system,
 * standing in for the Fetch API of Node.js, that does not support
 * file URLs, so that the sources of the shaders are loaded.
 *
 * @param input The URL of the file to be fetched.
 * @returns The response with the contents of the file or a not found
 * one in case the file does not exist.
 */
const fetchFile = async (input: string | URL | Request): Promise<Response> => {
    try {
        const data = await readFile(new URL(String(input)), "utf-8");
        return new Response(data);
    } catch (err) {
        return new Response(null, { status: 404 });
    }
};

/**
 * Reads the source of the shader file at the provided path.
 *
 * @param path The path of the shader file to be read.
 * @returns The source of the shader file.
 */
const readShader = async (path: string): Promise<string> => {
    return await readFile(path, "utf-8");
};

describe("Shaders", function () {
    let loadShader: typeof LoadShader;
    let fetch: typeof global.fetch;
    let warn: typeof console.warn;
    let fetched: string[];
    let warnings: string[];

    before(() => {
        // the shader files are bundled as raw files (resolved to their
        // URL) by Parcel, which is simulated with their file URL, the
        // module is only loaded afterwards as it requires the files
        require.extensions[".frag"] = (module, filename) => {
            module.exports = pathToFileURL(filename).href;
        };
        ({ loadShader } = require("../../ts/shaders"));
    });

    beforeEach(() => {
        fetched = [];
        warnings = [];
        fetch = global.fetch;
        warn = console.warn;
        global.fetch = (async (input: string | URL | Request) => {
            fetched.push(String(input));
            return await fetchFile(input);
        }) as typeof global.fetch;
        console.warn = (message: string) => {
            warnings.push(message);
        };
    });

    afterEach(() => {
        global.fetch = fetch;
        console.warn = warn;
    });

    after(() => {
        delete require.extensions[".frag"];
    });

    describe("#loadShader()", function () {
        it("should be able to place the filter in the master shader", async () => {
            const master = await readShader(
                resolve(WEB_SHADERS_PATH, "master.frag")
            );
            const filter = await readShader(
                resolve(SDL_SHADERS_PATH, "crt.frag")
            );
            const shader = await loadShader("crt");
            assert.ok(shader);
            assert.deepStrictEqual(shader, {
                fragment: master.replace("{filter}", () => filter)
            });
            assert.ok(shader.fragment.startsWith("#version 300 es"));
            assert.ok(!shader.fragment.includes("{filter}"));
            assert.deepStrictEqual(warnings, []);
        });

        it("should be able to load every filter", async () => {
            for (const name of FILTERS) {
                const filter = await readShader(
                    resolve(SDL_SHADERS_PATH, `${name}.frag`)
                );
                const shader = await loadShader(name);
                assert.ok(shader?.fragment.includes(filter), name);
            }
            assert.deepStrictEqual(warnings, []);
        });

        it("should be able to fetch the master shader and the filter", async () => {
            await loadShader("bilinear");
            assert.deepStrictEqual(
                fetched.map((url) => fileURLToPath(url)),
                [
                    resolve(WEB_SHADERS_PATH, "master.frag"),
                    resolve(SDL_SHADERS_PATH, "bilinear.frag")
                ]
            );
        });

        it("should be able to keep the replacement patterns of the filter", async () => {
            // the filter is placed as it is, the replacement patterns
            // of a string (eg: `$&`) must not be expanded
            global.fetch = (async (input: string | URL | Request) => {
                if (String(input).endsWith("master.frag")) {
                    return await fetchFile(input);
                }
                return new Response("vec4 scale() { return $& + $$ + $1; }");
            }) as typeof global.fetch;
            const shader = await loadShader("crt");
            assert.ok(
                shader?.fragment.includes(
                    "vec4 scale() { return $& + $$ + $1; }"
                )
            );
        });

        it("should not load an unknown shader", async () => {
            assert.strictEqual(await loadShader("unknown"), undefined);
            assert.deepStrictEqual(warnings, ["Unknown shader unknown"]);
            assert.deepStrictEqual(fetched, []);
        });

        it("should not load an inherited property as a shader", async () => {
            for (const name of ["constructor", "toString", "__proto__"]) {
                assert.strictEqual(await loadShader(name), undefined);
            }
            assert.deepStrictEqual(warnings, [
                "Unknown shader constructor",
                "Unknown shader toString",
                "Unknown shader __proto__"
            ]);
            assert.deepStrictEqual(fetched, []);
        });

        it("should not load the shader when a source can not be retrieved", async () => {
            global.fetch = (async (input: string | URL | Request) => {
                if (String(input).endsWith("crt.frag")) {
                    return new Response(null, { status: 404 });
                }
                return await fetchFile(input);
            }) as typeof global.fetch;
            assert.strictEqual(await loadShader("crt"), undefined);
            assert.deepStrictEqual(warnings, [
                "Problem loading shader crt: Error: Problem retrieving shader (404)"
            ]);
        });

        it("should not load the shader when a request fails", async () => {
            global.fetch = (async () => {
                throw new TypeError("Failed to fetch");
            }) as typeof global.fetch;
            assert.strictEqual(await loadShader("crt"), undefined);
            assert.deepStrictEqual(warnings, [
                "Problem loading shader crt: TypeError: Failed to fetch"
            ]);
        });
    });
});
