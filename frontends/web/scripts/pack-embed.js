/**
 * Builds the publish ready `boytacean-web` package under `pkg`.
 *
 * The package.json used to build the web front-end carries build time
 * configuration (`source`, `alias` and `targets`) that Parcel would also
 * apply when resolving the package from a consumer project, breaking it.
 * This script emits a trimmed manifest next to the built bundles instead
 * of publishing the development one.
 */

const {
    copyFileSync,
    cpSync,
    existsSync,
    mkdirSync,
    rmSync,
    writeFileSync
} = require("fs");
const { join, resolve } = require("path");

const BASE = resolve(__dirname, "..");
const ROOT = resolve(BASE, "..", "..");
const TARGET = join(BASE, "pkg");

/**
 * The directories with the built bundles that are copied into the
 * package, must be built before this script runs.
 */
const BUNDLES = ["component", "embed", "module"];

/**
 * The bundles that are published under a different directory name, the
 * React one is built into `component` to avoid colliding with the
 * `react` source directory, but is published as `react` so that
 * `boytacean-web/react` also resolves in the bundlers that ignore the
 * `exports` map (eg: Parcel).
 */
const RENAMES = { component: "react" };

/**
 * The entry point shims written into the published bundle directories,
 * they are what makes a directory import resolve without relying on the
 * `exports` map.
 */
const INDEXES = { react: "react.js" };

const manifest = require(join(BASE, "package.json"));

for (const bundle of BUNDLES) {
    if (existsSync(join(BASE, bundle))) continue;
    throw new Error(`Missing "${bundle}" bundle, run "npm run build:embed"`);
}

rmSync(TARGET, { force: true, recursive: true });
mkdirSync(TARGET, { recursive: true });

for (const bundle of BUNDLES) {
    cpSync(join(BASE, bundle), join(TARGET, RENAMES[bundle] ?? bundle), {
        recursive: true
    });
}

for (const [directory, entry] of Object.entries(INDEXES)) {
    writeFileSync(
        join(TARGET, directory, "index.js"),
        `export * from "./${entry}";\n`
    );
}

copyFileSync(join(ROOT, "LICENSE"), join(TARGET, "LICENSE"));
copyFileSync(join(ROOT, "doc", "embedding.md"), join(TARGET, "README.md"));

writeFileSync(
    join(TARGET, "package.json"),
    `${JSON.stringify(
        {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            repository: manifest.repository,
            license: manifest.license,
            keywords: manifest.keywords,
            exports: manifest.exports,
            module: manifest.module,
            dependencies: manifest.dependencies,
            peerDependencies: manifest.peerDependencies,
            peerDependenciesMeta: manifest.peerDependenciesMeta,
            sideEffects: true
        },
        null,
        4
    )}\n`
);

console.info(`Packed ${manifest.name}@${manifest.version} into ${TARGET}`);
