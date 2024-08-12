import React, { FC, useEffect, useRef } from "react";
import { Canvas, CanvasStructure, PixelFormat } from "emukit";

import "./tiles-gb.css";

type TilesGBProps = {
    getTile: (index: number) => Uint8Array;
    tileCount: number;
    width?: number | string;
    contentBox?: boolean;
    interval?: number;
    style?: string[];
};

export const TilesGB: FC<TilesGBProps> = ({
    getTile,
    tileCount,
    width,
    contentBox = true,
    interval = 1000,
    style = []
}) => {
    const classes = () =>
        ["tiles-gb", contentBox ? "content-box" : "", ...style].join(" ");
    const intervalsRef = useRef<number>();
    const canvasRef = useRef<HTMLCanvasElement>(null);
    useEffect(() => {
        return () => {
            if (intervalsRef.current) {
                clearInterval(intervalsRef.current);
            }
        };
    }, []);
    const onCanvas = (structure: CanvasStructure) => {
        const drawTiles = () => {
            for (let index = 0; index < tileCount; index++) {
                const pixels = getTile(index);
                drawTile(index, pixels, structure);
            }
        };
        drawTiles();
        intervalsRef.current = setInterval(() => drawTiles(), interval);
    };
    return (
        <div className={classes()}>
            <Canvas
                width={128}
                height={192}
                canvasRef={canvasRef}
                scale={2}
                scaledWidth={width}
                onCanvas={onCanvas}
            />
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
        (line * structure.canvasOffScreen.width * 8 + column * 8) *
        PixelFormat.RGBA;
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
            offset += (structure.canvasOffScreen.width - 7) * PixelFormat.RGBA;
        } else {
            offset += PixelFormat.RGBA;
        }
    }
    structure.canvasOffScreenContext.putImageData(structure.canvasImage, 0, 0);
    structure.canvasContext.drawImage(structure.canvasOffScreen, 0, 0);
};

export default TilesGB;
