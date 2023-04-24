import React, { FC, useEffect, useRef, useState } from "react";
import { Canvas, CanvasStructure, PixelFormat } from "emukit";
import { WebglPlot, WebglLine, ColorRGBA } from "webgl-plot";

import "./audio-gb.css";

type AudioGBProps = {
    getAudioOutput: () => Record<string, number>;
    interval?: number;
    drawInterval?: number;
    color?: number;
    range?: number;
    rangeVolume?: number;
    engine?: "webgl" | "canvas";
    style?: string[];
    renderWave?: (name: string, key: string, styles?: string[]) => JSX.Element;
};

export const AudioGB: FC<AudioGBProps> = ({
    getAudioOutput,
    interval = 1,
    drawInterval = 1000 / 60,
    color = 0x50cb93ff,
    range = 128,
    rangeVolume = 32,
    engine = "webgl",
    style = [],
    renderWave
}) => {
    const classes = () => ["audio-gb", ...style].join(" ");
    const [audioOutput, setAudioOutput] = useState<Record<string, number[]>>(
        {}
    );
    const intervalsRef = useRef<number>();
    const intervalsExtraRef = useRef<number>();

    useEffect(() => {
        const updateAudioOutput = () => {
            const _audioOutput = getAudioOutput();
            for (const [key, value] of Object.entries(_audioOutput)) {
                const values = audioOutput[key] ?? new Array(range).fill(0);
                values.push(value);
                if (values.length > range) {
                    values.shift();
                }
                audioOutput[key] = values;
            }
            setAudioOutput(audioOutput);
        };
        setInterval(() => updateAudioOutput(), interval);
        updateAudioOutput();
        return () => {
            if (intervalsRef.current) {
                clearInterval(intervalsRef.current);
            }
            if (intervalsExtraRef.current) {
                clearInterval(intervalsExtraRef.current);
            }
        };
    }, []);
    const renderAudioWave = (
        name: string,
        key: string,
        styles: string[] = []
    ) => {
        const classes = ["audio-wave", ...styles].join(" ");
        const onCanvas = (structure: CanvasStructure) => {
            const drawWave = () => {
                const values = audioOutput[key];
                if (!values) {
                    return;
                }
                structure.canvasImage.data.fill(0);
                values.forEach((value, index) => {
                    const valueN = Math.min(value, rangeVolume - 1);
                    const line = rangeVolume - 1 - valueN;
                    const offset = (line * range + index) * PixelFormat.RGBA;
                    structure.canvasBuffer.setUint32(offset, color);
                });
                structure.canvasOffScreenContext.putImageData(
                    structure.canvasImage,
                    0,
                    0
                );
                structure.canvasContext.clearRect(0, 0, range, rangeVolume);
                structure.canvasContext.drawImage(
                    structure.canvasOffScreen,
                    0,
                    0
                );
            };
            drawWave();
            intervalsExtraRef.current = setInterval(
                () => drawWave(),
                drawInterval
            );
        };
        return (
            <div className={classes}>
                <h4>{name}</h4>
                <Canvas
                    width={range}
                    height={rangeVolume}
                    onCanvas={onCanvas}
                />
            </div>
        );
    };
    const renderAudioWaveWgl = (
        name: string,
        key: string,
        styles: string[] = []
    ) => {
        const canvasRef = useRef<HTMLCanvasElement>(null);
        const classes = ["audio-wave", ...styles].join(" ");
        useEffect(() => {
            if (!canvasRef.current) return;

            // converts the canvas to the expected size according
            // to the device pixel ratio value
            const devicePixelRatio = window.devicePixelRatio || 1;
            canvasRef.current.width = range * devicePixelRatio;
            canvasRef.current.height = rangeVolume * devicePixelRatio;

            // creates the WGL Plot object with the canvas element
            // that is associated with the current audio wave
            const wglPlot = new WebglPlot(canvasRef.current);

            const colorRgba = new ColorRGBA(...intToColor2(color));
            const line = new WebglLine(colorRgba, range);

            line.arrangeX();
            wglPlot.addLine(line);

            const drawWave = () => {
                const values = audioOutput[key];
                if (!values) {
                    return;
                }

                values.forEach((value, index) => {
                    const valueN = Math.min(value, rangeVolume - 1);
                    line.setY(index, valueN / rangeVolume - 1);
                });

                wglPlot.update();
            };
            drawWave();
            intervalsExtraRef.current = setInterval(
                () => drawWave(),
                drawInterval
            );
        }, [canvasRef]);
        return (
            <div className={classes}>
                <h4>{name}</h4>
                <Canvas
                    canvasRef={canvasRef}
                    width={range}
                    height={rangeVolume}
                    init={false}
                />
            </div>
        );
    };
    let renderMethod =
        engine === "webgl" ? renderAudioWaveWgl : renderAudioWave;
    renderMethod = renderWave ?? renderMethod;
    return (
        <div className={classes()}>
            <div className="section">
                {renderMethod("Master", "master")}
                {renderMethod("CH1", "ch1")}
                {renderMethod("CH2", "ch2")}
                {renderMethod("CH3", "ch3")}
                {renderMethod("CH4", "ch4")}
            </div>
        </div>
    );
};

const intToColor = (int: number): [number, number, number, number] => {
    const r = (int >> 24) & 0xff;
    const g = (int >> 16) & 0xff;
    const b = (int >> 8) & 0xff;
    const a = int & 0xff;
    return [r, g, b, a];
};

const intToColor2 = (int: number): [number, number, number, number] => {
    const color = intToColor(int);
    return color.map((v) => v / 255) as [number, number, number, number];
};

export default AudioGB;
