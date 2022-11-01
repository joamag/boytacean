import React, { FC } from "react";
import { PixelFormat } from "../../app";
import Canvas, { CanvasStructure } from "../canvas/canvas";

import "./tiles.css";

type TilesProps = {
    getTile: (index: number) => Uint8Array;
    tileCount: number;
    style?: string[];
};

export const Tiles: FC<TilesProps> = ({ getTile, tileCount, style = [] }) => {
    const classes = () => ["title", ...style].join(" ");
    const onCanvas = (structure: CanvasStructure) => {
        setInterval(() => {
            for (let index = 0; index < 384; index++) {
                const pixels = getTile(index);
                drawTile(index, pixels, structure);
            }
        }, 1000);
    };
    return (
        <div className={classes()}>
            <Canvas width={128} height={192} scale={2} onCanvas={onCanvas} />
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
 * @param structure The canvas context to which the tile is
 * growing to be drawn.
 * @param format The pixel format of the sprite.
 */
const drawTile = (
    index: number,
    pixels: Uint8Array,
    structure: CanvasStructure,
    format: PixelFormat = PixelFormat.RGB
) => {
    const line = Math.floor(index / 16);
    const column = index % 16;
    let offset =
        (line * structure.canvas.width * 8 + column * 8) * PixelFormat.RGBA;
    let counter = 0;
    for (let i = 0; i < pixels.length; i += format) {
        const color =
            (pixels[i] << 24) |
            (pixels[i + 1] << 16) |
            (pixels[i + 2] << 8) |
            (format === PixelFormat.RGBA ? pixels[i + 3] : 0xff);
        structure.canvasBuffer.setUint32(offset, color);

        counter++;
        if (counter === 8) {
            counter = 0;
            offset += (structure.canvas.width - 7) * PixelFormat.RGBA;
        } else {
            offset += PixelFormat.RGBA;
        }
    }
    structure.canvasContext.putImageData(structure.canvasImage, 0, 0);
};

export default Tiles;
