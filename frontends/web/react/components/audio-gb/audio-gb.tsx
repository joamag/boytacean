import { Canvas, CanvasStructure, PixelFormat } from "emukit";
import React, {
    FC,
    useCallback,
    useEffect,
    useMemo,
    useRef,
    useState
} from "react";
import { ColorRGBA, WebglLine, WebglPlot } from "webgl-plot";

import { GameboyEmulator } from "../../../ts";

import "./audio-gb.css";

type AudioGBProps = {
    emulator: GameboyEmulator;
    getAudioOutput: () => Record<string, number>;
    interval?: number;
    drawInterval?: number;
    color?: number;
    range?: number;
    rangeVolume?: number;
    engine?: "webgl" | "canvas";
    style?: string[];
    renderWave?: (
        name: string,
        key: string,
        styles?: string[],
        onClick?: (key: string) => void
    ) => JSX.Element;
};

export const AudioGB: FC<AudioGBProps> = ({
    emulator,
    getAudioOutput,
    interval = 1,
    drawInterval = 1000 / 60,
    color = 0x58b09cff,
    range = 128,
    rangeVolume = 32,
    engine = "webgl",
    style = [],
    renderWave
}) => {
    const classes = useMemo(() => ["audio-gb", ...style].join(" "), [style]);
    const [audioOutput, setAudioOutput] = useState<Record<string, number[]>>(
        {}
    );
    const [ch1Enabled, setCh1Enabled] = useState(
        emulator.instance?.audio_ch1_enabled() ?? true
    );
    const [ch2Enabled, setCh2Enabled] = useState(
        emulator.instance?.audio_ch2_enabled() ?? true
    );
    const [ch3Enabled, setCh3Enabled] = useState(
        emulator.instance?.audio_ch3_enabled() ?? true
    );
    const [ch4Enabled, setCh4Enabled] = useState(
        emulator.instance?.audio_ch4_enabled() ?? true
    );

    useEffect(
        () => {
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

            updateAudioOutput();

            const audioInterval = setInterval(
                () => updateAudioOutput(),
                interval
            );
            return () => {
                clearInterval(audioInterval);
            };
        },
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [audioOutput, interval, range]
    );

    const renderAudioWave = (
        name: string,
        key: string,
        styles?: string[],
        onClick?: (key: string) => void
    ): JSX.Element => {
        if (renderWave) {
            return renderWave(name, key, styles, onClick);
        }
        if (engine === "canvas") {
            return (
                <AudioWave
                    key={key}
                    name={name}
                    index={key}
                    audioOutput={audioOutput}
                    drawInterval={drawInterval}
                    color={color}
                    range={range}
                    rangeVolume={rangeVolume}
                    styles={styles}
                    onClick={onClick}
                />
            );
        }
        return (
            <AudioWaveWebGL
                key={key}
                name={name}
                index={key}
                audioOutput={audioOutput}
                drawInterval={drawInterval}
                color={color}
                range={range}
                rangeVolume={rangeVolume}
                styles={styles}
                onClick={onClick}
            />
        );
    };

    return (
        <div className={classes}>
            <div className="section">
                {renderAudioWave("Master", "master", ["master"])}
                {renderAudioWave(
                    "CH1",
                    "ch1",
                    ["ch1", "selector", ch1Enabled ? "" : "disabled"],
                    () => {
                        emulator.instance?.set_audio_ch1_enabled(!ch1Enabled);
                        setCh1Enabled(!ch1Enabled);
                    }
                )}
                {renderAudioWave(
                    "CH2",
                    "ch2",
                    ["ch2", "selector", ch2Enabled ? "" : "disabled"],
                    () => {
                        emulator.instance?.set_audio_ch2_enabled(!ch2Enabled);
                        setCh2Enabled(!ch2Enabled);
                    }
                )}
                {renderAudioWave(
                    "CH3",
                    "ch3",
                    ["ch3", "selector", ch3Enabled ? "" : "disabled"],
                    () => {
                        emulator.instance?.set_audio_ch3_enabled(!ch3Enabled);
                        setCh3Enabled(!ch3Enabled);
                    }
                )}
                {renderAudioWave(
                    "CH4",
                    "ch4",
                    ["ch4", "selector", ch4Enabled ? "" : "disabled"],
                    () => {
                        emulator.instance?.set_audio_ch4_enabled(!ch4Enabled);
                        setCh4Enabled(!ch4Enabled);
                    }
                )}
            </div>
        </div>
    );
};

type AudioWaveProps = {
    name: string;
    index: string;
    audioOutput: Record<string, number[]>;
    drawInterval?: number;
    color?: number;
    range?: number;
    rangeVolume?: number;
    styles?: string[];
    onClick?: (key: string) => void;
};

const AudioWave: FC<AudioWaveProps> = ({
    name,
    index,
    audioOutput,
    drawInterval = 1000 / 60,
    color = 0x58b09cff,
    range = 128,
    rangeVolume = 32,
    styles = [],
    onClick
}) => {
    const classes = useMemo(
        () => ["audio-wave", ...styles].join(" "),
        [styles]
    );
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const onCanvas = useCallback(
        (structure: CanvasStructure) => {
            const drawWave = () => {
                const values = audioOutput[index];
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

            const interval = setInterval(() => drawWave(), drawInterval);
            return () => {
                clearInterval(interval);
            };
        },
        [index, audioOutput, drawInterval, color, range, rangeVolume]
    );
    return (
        <div className={classes} onClick={() => onClick && onClick(index)}>
            <h4>{name}</h4>
            <Canvas
                width={range}
                height={rangeVolume}
                canvasRef={canvasRef}
                onCanvas={onCanvas}
            />
        </div>
    );
};

type AudioWaveWebGLProps = {
    name: string;
    index: string;
    audioOutput: Record<string, number[]>;
    drawInterval?: number;
    color?: number;
    range?: number;
    rangeVolume?: number;
    styles?: string[];
    onClick?: (key: string) => void;
};

const AudioWaveWebGL: FC<AudioWaveWebGLProps> = ({
    name,
    index,
    audioOutput,
    drawInterval = 1000 / 60,
    color = 0x58b09cff,
    range = 128,
    rangeVolume = 32,
    styles = [],
    onClick
}) => {
    const classes = useMemo(
        () => ["audio-wave", ...styles].join(" "),
        [styles]
    );
    const canvasRef = useRef<HTMLCanvasElement>(null);
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
            const values = audioOutput[index];
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

        const interval = setInterval(() => drawWave(), drawInterval);
        return () => {
            clearInterval(interval);
        };
    }, [audioOutput, index, drawInterval, color, range, rangeVolume]);
    return (
        <div className={classes} onClick={() => onClick && onClick(index)}>
            <h4>{name}</h4>
            <Canvas
                width={range}
                height={rangeVolume}
                canvasRef={canvasRef}
                init={false}
            />
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
