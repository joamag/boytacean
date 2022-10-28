import React, { FC, useRef, useEffect } from "react";
import { PixelFormat } from "../../app";

import "./display.css";

declare const require: any;

/**
 * Function that handles a draw operation into a
 * certain drawing context.
 */
export type DrawHandler = (pixels: Uint8Array, format: PixelFormat) => void;

type DisplayOptions = {
    width: number;
    height: number;
    logicWidth: number;
    logicHeight: number;
    scale?: number;
};

type DisplayProps = {
    options?: DisplayOptions;
    size?: string;
    fullscreen?: boolean,
    style?: string[];
    onDrawHandler?: (caller: DrawHandler) => void;
};

type CanvasContents = {
    canvasCtx: CanvasRenderingContext2D;
    canvasBuffer: HTMLCanvasElement;
    canvasBufferCtx: CanvasRenderingContext2D;
    imageData: ImageData;
    videoBuffer: DataView;
};

export const Display: FC<DisplayProps> = ({
    options = { width: 320, height: 288, logicWidth: 160, logicHeight: 144 },
    size = "small",
    fullscreen = false,
    style = [],
    onDrawHandler
}) => {
    options = {
        ...options,
        ...{ width: 320, height: 288, logicWidth: 160, logicHeight: 144 }
    };
    if (!options.scale) {
        options.scale = window.devicePixelRatio ? window.devicePixelRatio : 1;
    }

    let canvasContents: CanvasContents | null = null;
    const classes = () => ["display", fullscreen ? "fullscreen" : null, size, ...style].join(" ");

    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        if (canvasRef.current && !canvasContents) {
            canvasContents = initCanvas(
                options.logicWidth,
                options.logicHeight,
                canvasRef.current
            );
        }
    });

    if (onDrawHandler) {
        onDrawHandler((pixels: Uint8Array, format: PixelFormat) => {
            if (!canvasContents) return;
            updateCanvas(canvasContents, pixels, format);
        });
    }

    return (
        <div id="display" className={classes()}>
            <span id="display-close" className="magnify-button display-close">
                <img
                    className="large"
                    src={require("./minimise.svg")}
                    alt="minimise"
                />
            </span>
            <div className="display-frame">
                <canvas
                    ref={canvasRef}
                    id="display-canvas"
                    className="display-canvas"
                    width={options.width * options.scale}
                    height={options.height * options.scale}
                ></canvas>
            </div>
        </div>
    );
};

const initCanvas = (
    width: number,
    height: number,
    canvas: HTMLCanvasElement
): CanvasContents => {
    // initializes the off-screen canvas that is going to be
    // used in the drawing process, this is used essentially for
    // performance reasons as it provides a way to draw pixels
    // in the original size instead of the target one
    const canvasBuffer = document.createElement("canvas");
    canvasBuffer.width = width;
    canvasBuffer.height = height;
    const canvasBufferCtx = canvasBuffer.getContext("2d")!;
    const imageData = canvasBufferCtx.createImageData(
        canvasBuffer.width,
        canvasBuffer.height
    );
    const videoBuffer = new DataView(imageData.data.buffer);

    const canvasCtx = canvas.getContext("2d")!;
    canvasCtx.scale(
        canvas.width / canvasBuffer.width,
        canvas.height / canvasBuffer.height
    );
    canvasCtx.imageSmoothingEnabled = false;

    return {
        canvasCtx: canvasCtx,
        canvasBuffer: canvasBuffer,
        canvasBufferCtx: canvasBufferCtx,
        imageData: imageData,
        videoBuffer: videoBuffer
    };
};

const updateCanvas = (
    canvasContents: CanvasContents,
    pixels: Uint8Array,
    format: PixelFormat = PixelFormat.RGB
) => {
    let offset = 0;
    for (let index = 0; index < pixels.length; index += format) {
        const color =
            (pixels[index] << 24) |
            (pixels[index + 1] << 16) |
            (pixels[index + 2] << 8) |
            (format == PixelFormat.RGBA ? pixels[index + 3] : 0xff);
        canvasContents.videoBuffer.setUint32(offset, color);
        offset += PixelFormat.RGBA;
    }
    canvasContents.canvasBufferCtx.putImageData(canvasContents.imageData, 0, 0);
    canvasContents.canvasCtx.drawImage(canvasContents.canvasBuffer, 0, 0);
};

export default Display;
