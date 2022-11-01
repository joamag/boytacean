import React, { FC, useEffect, useRef } from "react";

import "./canvas.css";

export type CanvasStructure = {
    canvas: HTMLCanvasElement;
    canvasContext: CanvasRenderingContext2D;
    canvasImage: ImageData;
    canvasBuffer: DataView;
};

type CanvasProps = {
    width: number;
    height: number;
    scale?: number;
    style?: string[];
    onCanvas?: (structure: CanvasStructure) => void;
};

export const Canvas: FC<CanvasProps> = ({
    width,
    height,
    scale = 1,
    style = [],
    onCanvas
}) => {
    const classes = () => ["canvas", ...style].join(" ");
    const canvasRef = useRef<HTMLCanvasElement>(null);
    useEffect(() => {
        if (canvasRef.current) {
            const structure = initCanvas(
                width,
                height,
                scale,
                canvasRef.current
            );
            onCanvas && onCanvas(structure);
        }
    }, [canvasRef]);
    return (
        <canvas
            ref={canvasRef}
            className={classes()}
            style={{ width: width * scale }}
            width={width}
            height={height}
        />
    );
};

const initCanvas = (
    width: number,
    height: number,
    scale: number,
    canvas: HTMLCanvasElement,
    smoothing = false
): CanvasStructure => {
    const canvasContext = canvas.getContext("2d")!;
    canvasContext.imageSmoothingEnabled = smoothing;

    const canvasImage = canvasContext.createImageData(width, height);
    const canvasBuffer = new DataView(canvasImage.data.buffer);

    return {
        canvas: canvas,
        canvasContext: canvasContext,
        canvasImage: canvasImage,
        canvasBuffer: canvasBuffer
    };
};

export default Canvas;
