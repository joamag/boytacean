import React, { FC } from "react";
import { PixelFormat } from "../../app";

import "./tiles.css";

type TilesProps = {
    style?: string[];
};

export const Tiles: FC<TilesProps> = ({ style = [] }) => {
    const classes = () => ["title", ...style].join(" ");
    return (
        <div className={classes()}>
            <canvas className="canvas-tiles" width="128" height="192"></canvas>
        </div>
    );
};

/**
 * Draws the tile at the given index to the proper vertical
 * offset in the given context and buffer.
 *
 * @param index The index of the sprite to be drawn.
 * @param pixels Buffer of pixels that contains the RGB data
 * that is going to be drawn.
 * @param context The canvas context to which the tile is
 * growing to be drawn.
 * @param buffer The data buffer to be used in the drawing
 * process, re-usage of it improves performance.
 * @param format The pixel format of the sprite.
 */
const drawTile = (
    index: number,
    pixels: Uint8Array,
    canvas: HTMLCanvasElement,
    context: CanvasRenderingContext2D,
    canvasImage: ImageData,
    buffer: DataView,
    format: PixelFormat = PixelFormat.RGB
) => {
    const line = Math.floor(index / 16);
    const column = index % 16;
    let offset = (line * canvas.width * 8 + column * 8) * PixelFormat.RGBA;
    let counter = 0;
    for (let i = 0; i < pixels.length; i += format) {
        const color =
            (pixels[i] << 24) |
            (pixels[i + 1] << 16) |
            (pixels[i + 2] << 8) |
            (format === PixelFormat.RGBA ? pixels[i + 3] : 0xff);
        buffer.setUint32(offset, color);

        counter++;
        if (counter === 8) {
            counter = 0;
            offset += (canvas.width - 7) * PixelFormat.RGBA;
        } else {
            offset += PixelFormat.RGBA;
        }
    }
    context.putImageData(canvasImage, 0, 0);
};

export default Tiles;
