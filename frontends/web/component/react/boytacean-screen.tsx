import React, { CSSProperties, FC, useEffect, useRef } from "react";

import { useBoytacean } from "./boytacean-context";

type BoytaceanScreenProps = {
    /**
     * The scale factor to be applied to the logical dimensions of
     * the Game Boy display.
     */
    scale?: number;

    className?: string;
    style?: CSSProperties;
};

/**
 * Draws the frame buffer of the emulator into a canvas element.
 *
 * The frame buffer is drawn imperatively from the frame event of the
 * core, meaning that it never goes through the React state and that
 * no re-render is triggered by the emulation.
 */
export const BoytaceanScreen: FC<BoytaceanScreenProps> = ({
    scale = 3,
    className,
    style
}) => {
    const { core } = useBoytacean();
    const canvasRef = useRef<HTMLCanvasElement>(null);

    // the dimensions are obtained from the core itself, so that the
    // display of both the Game Boy and the GBA is properly sized
    const { width, height } = core.dimensions;

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const context = canvas.getContext("2d");
        if (!context) return;

        const imageData = context.createImageData(width, height);
        const buffer = imageData.data;

        const onFrame = () => {
            // converts the RGB frame buffer of the emulator into the
            // RGBA buffer that is required by the canvas image data
            const frameBuffer = core.imageBuffer;
            for (
                let index = 0, offset = 0;
                index < frameBuffer.length;
                index += 3, offset += 4
            ) {
                buffer[offset] = frameBuffer[index];
                buffer[offset + 1] = frameBuffer[index + 1];
                buffer[offset + 2] = frameBuffer[index + 2];
                buffer[offset + 3] = 0xff;
            }
            context.putImageData(imageData, 0, 0);
        };
        core.bind("frame", onFrame);

        return () => {
            core.unbind("frame", onFrame);
        };
    }, [core, width, height]);

    return (
        <canvas
            ref={canvasRef}
            className={className}
            width={width}
            height={height}
            style={{
                width: width * scale,
                height: height * scale,
                imageRendering: "pixelated",
                ...style
            }}
        />
    );
};
