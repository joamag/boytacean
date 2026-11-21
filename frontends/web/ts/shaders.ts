import { Shader } from "emukit";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

/**
 * The filters (shared with the SDL front-end) that can be
 * used by the shader, indexed by their name.
 */
const FILTERS: Record<string, string> = {
    bilinear: require("../../sdl/res/shaders/bilinear.frag"),
    crt: require("../../sdl/res/shaders/crt.frag"),
    passthrough: require("../../sdl/res/shaders/passthrough.frag"),
    smooth_bilinear: require("../../sdl/res/shaders/smooth_bilinear.frag")
};

/**
 * Loads the shader that uses the filter with the provided
 * name, placing the filter in the master shader.
 *
 * As the shader is optional, any problem loading it is logged
 * and no shader is returned, so that the display keeps its
 * regular drawing.
 *
 * @param name The name of the filter used by the shader.
 * @returns The shader, or undefined if it can not be loaded.
 */
export const loadShader = async (name: string): Promise<Shader | undefined> => {
    const filter = FILTERS[name];
    if (!filter) {
        console.warn(`Unknown shader ${name}`);
        return undefined;
    }
    try {
        const [master, source] = await Promise.all([
            fetchSource(require("../res/shaders/master.frag")),
            fetchSource(filter)
        ]);
        return { fragment: master.replace("{filter}", () => source) };
    } catch (err) {
        console.warn(`Problem loading shader ${name}: ${err}`);
        return undefined;
    }
};

const fetchSource = async (url: string): Promise<string> => {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Problem retrieving shader (${response.status})`);
    }
    return await response.text();
};
