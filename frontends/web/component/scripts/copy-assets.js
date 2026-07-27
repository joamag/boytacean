/* eslint-disable @typescript-eslint/no-var-requires, no-undef */

const { copyFileSync, mkdirSync, readdirSync, statSync } = require("fs");
const { dirname, join, relative } = require("path");

/**
 * The set of file extensions that are considered assets, meaning
 * that they are not handled by the TypeScript compiler and have to
 * be copied into the build directory as they are.
 */
const EXTENSIONS = [".css"];

/**
 * The directories whose assets are going to be copied, relative to
 * the root of the package.
 */
const SOURCES = ["react"];

/**
 * The directory into which the assets are going to be copied.
 */
const TARGET = "dist";

/**
 * Copies every asset of the provided directory into the target one,
 * preserving the relative structure of the source.
 *
 * @param {string} path The directory to be walked.
 */
const copyAssets = (path) => {
    for (const entry of readdirSync(path)) {
        const source = join(path, entry);
        if (statSync(source).isDirectory()) {
            copyAssets(source);
            continue;
        }
        if (!EXTENSIONS.some((extension) => entry.endsWith(extension))) {
            continue;
        }
        const target = join(TARGET, relative(".", source));
        mkdirSync(dirname(target), { recursive: true });
        copyFileSync(source, target);
        console.info(`Copied ${source} to ${target}`);
    }
};

SOURCES.forEach((source) => copyAssets(source));
