import { Emulator } from "emukit";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

export async function setupWebGL(emulator: Emulator, shader: string) {
    const canvas = document.querySelector<HTMLCanvasElement>(".display-canvas");
    if (!canvas) {
        console.error("No canvas found");
        return;
    }
    const gl = canvas.getContext("webgl2");
    if (!gl) {
        console.error("No WebGL context found");
        return;
    }

    const vertexSrc = await fetchShader(require("../res/shaders/master.vert"));
    const fragPartial = await fetchShader(getShaderPath(shader));
    const masterFrag = await fetchShader(require("../res/shaders/master.frag"));
    const fragmentSrc = masterFrag.replace("{filter}", fragPartial);

    const vertex = compileShader(gl, gl.VERTEX_SHADER, vertexSrc);
    const fragment = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSrc);
    if (!vertex || !fragment) return;

    const program = createProgram(gl, vertex, fragment);
    if (!program) return;

    const vao = gl.createVertexArray();
    const vbo = gl.createBuffer();
    const vertices = new Float32Array([
        -1, -1, 0, 0, 1, -1, 1, 0, -1, 1, 0, 1, 1, 1, 1, 1
    ]);

    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 16, 0);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 16, 8);
    gl.enableVertexAttribArray(1);
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
    gl.bindVertexArray(null);

    const texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    const locImage = gl.getUniformLocation(program, "image");
    const locOut = gl.getUniformLocation(program, "output_resolution");
    const locOrigin = gl.getUniformLocation(program, "origin");

    gl.useProgram(program);
    gl.uniform1i(locImage, 0);

    const draw = () => {
        const buf = emulator.imageBuffer;
        if (!buf) return;
        const [w, h] = [emulator.dimensions.width, emulator.dimensions.height];
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texImage2D(
            gl.TEXTURE_2D,
            0,
            gl.RGB,
            w,
            h,
            0,
            gl.RGB,
            gl.UNSIGNED_BYTE,
            buf
        );
        gl.viewport(0, 0, canvas.width, canvas.height);
        gl.uniform2f(locOut, canvas.width, canvas.height);
        gl.uniform2f(locOrigin, 0, 0);
        gl.bindVertexArray(vao);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    };

    emulator.bind("frame", draw);
}

function getShaderPath(name: string): string {
    switch (name) {
        case "bilinear":
            return require("../res/shaders/bilinear.frag");
        case "smooth":
        case "smooth_bilinear":
            return require("../res/shaders/smooth_bilinear.frag");
        case "crt":
            return require("../res/shaders/crt.frag");
        case "pass":
        case "passthrough":
        default:
            return require("../res/shaders/passthrough.frag");
    }
}

async function fetchShader(path: string): Promise<string> {
    const response = await fetch(path);
    return await response.text();
}

/**
 * Compiles a WebGL shader from source code, and returns the shader object.
 * If compilation fails, logs the error message to the console and returns null.
 *
 * @param gl The WebGL2 rendering context.
 * @param type The type of shader to compile (gl.VERTEX_SHADER or gl.FRAGMENT_SHADER).
 * @param source The GLSL source code for the shader.
 * @returns The compiled WebGL shader object, or null if compilation failed.
 */
function compileShader(
    gl: WebGL2RenderingContext,
    type: number,
    source: string
) {
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error(gl.getShaderInfoLog(shader));
        return null;
    }
    return shader;
}

function createProgram(
    gl: WebGL2RenderingContext,
    vs: WebGLShader,
    fs: WebGLShader
) {
    const program = gl.createProgram();
    if (!program) return null;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        console.error(gl.getProgramInfoLog(program));
        return null;
    }
    return program;
}
