import React, { FC, useState, useRef, useEffect } from "react";
import { PixelFormat } from "../../app";

import "./display.css";

const PIXEL_UNSET_COLOR = 0x1b1a17ff;

declare const require: any;

/**
 * Function that handles a draw operation into a
 * certain drawing context.
 */
export type DrawHandler = (pixels: Uint8Array, format: PixelFormat) => void;

export type ClearHandler = (
    color?: number,
    image?: string,
    imageScale?: number
) => Promise<void>;

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
    fullscreen?: boolean;
    nativeFullscreen?: boolean;
    style?: string[];
    onDrawHandler?: (caller: DrawHandler) => void;
    onClearHandler?: (caller: ClearHandler) => void;
    onMinimize?: () => void;
};

type CanvasContents = {
    canvas: HTMLCanvasElement;
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
    nativeFullscreen = true,
    style = [],
    onDrawHandler,
    onClearHandler,
    onMinimize
}) => {
    options = {
        ...options,
        ...{ width: 320, height: 288, logicWidth: 160, logicHeight: 144 }
    };
    if (!options.scale) {
        options.scale = window.devicePixelRatio ? window.devicePixelRatio : 1;
    }

    const classes = () =>
        ["display", fullscreen ? "fullscreen" : null, size, ...style].join(" ");

    const [width, setWidth] = useState<number>();
    const [height, setHeight] = useState<number>();
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const canvasContentsRef = useRef<CanvasContents>();
    const resizeRef = useRef(() => {
        const [fullWidth, fullHeight] = crop(options.width / options.height);
        setWidth(fullWidth);
        setHeight(fullHeight);
    });

    useEffect(() => {
        if (canvasRef.current) {
            canvasContentsRef.current = initCanvas(
                options.logicWidth,
                options.logicHeight,
                canvasRef.current
            );
        }
    }, [canvasRef, options.scale]);

    useEffect(() => {
        if (fullscreen) {
            canvasRef.current?.focus();
            resizeRef.current();
            document.getElementsByTagName("body")[0].style.overflow = "hidden";
            window.addEventListener("resize", resizeRef.current);

            // requests the browser to go fullscreen using the
            // body of the document as the entry HTML element
            if (nativeFullscreen && document.body.requestFullscreen) {
                document.body.requestFullscreen().catch(() => {});
            } else if (
                nativeFullscreen &&
                (document.body as any).webkitRequestFullscreen
            ) {
                (document.body as any).webkitRequestFullscreen();
            }
        } else {
            setWidth(undefined);
            setHeight(undefined);
            document
                .getElementsByTagName("body")[0]
                .style.removeProperty("overflow");
            window.removeEventListener("resize", resizeRef.current);

            // restores the window mode, returning from the
            // fullscreen browser
            if (nativeFullscreen && document.exitFullscreen) {
                document.exitFullscreen().catch(() => {});
            } else if (
                nativeFullscreen &&
                (document as any).webkitExitFullscreen
            ) {
                (document as any).webkitExitFullscreen();
            }
        }
        return () => {
            window.removeEventListener("resize", resizeRef.current);
        };
    }, [fullscreen]);

    if (onDrawHandler) {
        onDrawHandler((pixels, format) => {
            if (!canvasContentsRef.current) return;
            updateCanvas(canvasContentsRef.current, pixels, format);
        });
    }

    if (onClearHandler) {
        onClearHandler(async (color, image, imageScale) => {
            if (!canvasContentsRef.current) return;
            await clearCanvas(canvasContentsRef.current, color, {
                image: image,
                imageScale: imageScale
            });
        });
    }

    return (
        <div className={classes()}>
            <span
                className="magnify-button display-minimize"
                onClick={onMinimize}
            >
                <img
                    className="large"
                    src={require("./minimise.svg")}
                    alt="minimise"
                />
            </span>
            <div
                className="display-frame"
                style={{ width: width ?? options.width, height: height }}
            >
                <canvas
                    ref={canvasRef}
                    tabIndex={-1}
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

    // initializes the visual canvas (where data is going to be written)
    // with, resetting the transform vector to the identity and re-calculating
    // the scale of the drawing properly
    const canvasCtx = canvas.getContext("2d")!;
    canvasCtx.setTransform(1, 0, 0, 1, 0, 0);
    canvasCtx.scale(
        canvas.width / canvasBuffer.width,
        canvas.height / canvasBuffer.height
    );
    canvasCtx.imageSmoothingEnabled = false;

    return {
        canvas: canvas,
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

const clearCanvas = async (
    canvasContents: CanvasContents,
    color = PIXEL_UNSET_COLOR,
    {
        image = null,
        imageScale = 1
    }: { image?: string | null; imageScale?: number } = {}
) => {
    // uses the "clear" color to fill a rectangle with the complete
    // size of the canvas contents
    canvasContents.canvasCtx.fillStyle = `#${color.toString(16).toUpperCase()}`;
    canvasContents.canvasCtx.fillRect(
        0,
        0,
        canvasContents.canvas.width,
        canvasContents.canvas.height
    );

    // in case an image was requested then uses that to load
    // an image at the center of the screen properly scaled
    if (image) {
        await drawImageCanvas(canvasContents, image, imageScale);
    }
};

const drawImageCanvas = async (
    canvasContents: CanvasContents,
    image: string,
    imageScale = 1.0
) => {
    const img = await new Promise<HTMLImageElement>((resolve) => {
        const img = new Image();
        img.onload = () => {
            resolve(img);
        };
        img.src = image;
    });
    const [imgWidth, imgHeight] = [
        img.width * imageScale * window.devicePixelRatio,
        img.height * imageScale * window.devicePixelRatio
    ];
    const [x0, y0] = [
        canvasContents.canvas.width / 2 - imgWidth / 2,
        canvasContents.canvas.height / 2 - imgHeight / 2
    ];
    canvasContents.canvasCtx.setTransform(1, 0, 0, 1, 0, 0);
    try {
        canvasContents.canvasCtx.drawImage(img, x0, y0, imgWidth, imgHeight);
    } finally {
        canvasContents.canvasCtx.scale(
            canvasContents.canvas.width / canvasContents.canvasBuffer.width,
            canvasContents.canvas.height / canvasContents.canvasBuffer.height
        );
    }
};

const crop = (ratio: number): [number, number] => {
    // calculates the window ratio as this is fundamental to
    // determine the proper way to crop the fullscreen
    const windowRatio = window.innerWidth / window.innerHeight;

    // in case the window is wider (more horizontal than the base ratio)
    // this means that we must crop horizontally
    if (windowRatio > ratio) {
        return [window.innerWidth * (ratio / windowRatio), window.innerHeight];
    } else {
        return [window.innerWidth, window.innerHeight * (windowRatio / ratio)];
    }
};

export default Display;
