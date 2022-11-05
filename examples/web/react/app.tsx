import React, { FC, useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";

declare const require: any;

import {
    Button,
    ButtonContainer,
    ButtonIncrement,
    ButtonSwitch,
    ClearHandler,
    Display,
    DrawHandler,
    Footer,
    Info,
    KeyboardGB,
    Link,
    Modal,
    Overlay,
    Pair,
    PanelSplit,
    Paragraph,
    Section,
    Tiles,
    Title,
    Toast
} from "./components";

import "./app.css";

export type Callback<T> = (owner: T, params?: any) => void;

/**
 * Abstract class that implements the basic functionality
 * part of the definition of the Observer pattern.
 *
 * @see {@link https://en.wikipedia.org/wiki/Observer_pattern}
 */
export class Observable {
    private events: Record<string, [Callback<this>]> = {};

    bind(event: string, callback: Callback<this>) {
        const callbacks = this.events[event] ?? [];
        if (callbacks.includes(callback)) return;
        callbacks.push(callback);
        this.events[event] = callbacks;
    }

    unbind(event: string, callback: Callback<this>) {
        const callbacks = this.events[event] ?? [];
        if (!callbacks.includes(callback)) return;
        const index = callbacks.indexOf(callback);
        callbacks.splice(index, 1);
        this.events[event] = callbacks;
    }

    trigger(event: string, params?: any) {
        const callbacks = this.events[event] ?? [];
        callbacks.forEach((c) => c(this, params));
    }
}

export type RomInfo = {
    name?: string;
    data?: Uint8Array;
    size?: number;
    extra?: Record<string, string | undefined>;
};

export type BenchmarkResult = {
    delta: number;
    count: number;
    cycles: number;
    frequency_mhz: number;
};

export interface ObservableI {
    bind(event: string, callback: Callback<this>): void;
    unbind(event: string, callback: Callback<this>): void;
    trigger(event: string): void;
}

/**
 * Top level interface that declares the main abstract
 * interface of an emulator structured entity.
 * Should allow typical hardware operations to be performed.
 */
export interface Emulator extends ObservableI {
    /**
     * The descriptive name of the emulator.
     */
    get name(): string;

    /**
     * The name of the the hardware that is being emulated
     * by the emulator (eg: Super Nintendo).
     */
    get device(): string;

    get deviceUrl(): string | undefined;

    /**
     * A semantic version string for the current version
     * of the emulator.
     *
     * @see {@link https://semver.org}
     */
    get version(): string;

    /**
     * The URL to the page describing the current version
     * of the emulator.
     */
    get versionUrl(): string;

    /**
     * The complete set of engine names that can be used
     * in the re-boot operation.
     */
    get engines(): string[];

    /**
     * The name of the current execution engine being used
     * by the emulator.
     */
    get engine(): string | null;

    /**
     * The pixel format of the emulator's display
     * image buffer (eg: RGB).
     */
    get pixelFormat(): PixelFormat;

    /**
     * Gets the complete image buffer as a sequence of
     * bytes that respects the current pixel format from
     * `getPixelFormat()`. This method returns an in memory
     * pointer to the heap and not a copy.
     */
    get imageBuffer(): Uint8Array;

    /**
     * Gets information about the ROM that is currently
     * loaded in the emulator, using a structure containing
     * the information about the ROM that is currently
     * loaded in the emulator.
     */
    get romInfo(): RomInfo;

    /**
     * The current CPU frequency (logic) of the emulator,
     * should impact other elements of the emulator.
     */
    get frequency(): number;
    set frequency(value: number);

    /**
     * The current logic framerate of the running emulator.
     */
    get framerate(): number;

    getTile(index: number): Uint8Array;

    /**
     * Boot (or reboots) the emulator according to the provided
     * set of options.
     *
     * @param options The options that are going to be used for
     * the booting operation of the emulator.
     */
    boot(options: any): void;

    /**
     * Toggle the running state of the emulator between paused
     * and running, prevents consumers from the need to access
     * the current running state of the emulator to implement
     * a logic toggle.
     */
    toggleRunning(): void;
    pause(): void;
    resume(): void;

    /**
     * Resets the emulator machine to the start state and
     * re-loads the ROM that is currently set in the emulator.
     */
    reset(): void;

    /**
     * Runs a benchmark operation in the emulator, effectively
     * measuring the performance of it.
     *
     * @param count The number of benchmark iterations to be
     * run, increasing this value will make the benchmark take
     * more time to be executed.
     * @returns The result metrics from the benchmark run.
     */
    benchmark(count?: number): BenchmarkResult;
}

export class EmulatorBase extends Observable {
    get deviceUrl(): string | undefined {
        return undefined;
    }

    get versionUrl(): string | undefined {
        return undefined;
    }
}

/**
 * Enumeration that describes the multiple pixel
 * formats and the associated size in bytes.
 */
export enum PixelFormat {
    RGB = 3,
    RGBA = 4
}

type AppProps = {
    emulator: Emulator;
    backgrounds?: string[];
};

export const App: FC<AppProps> = ({ emulator, backgrounds = ["264653"] }) => {
    const [paused, setPaused] = useState(false);
    const [fullscreen, setFullscreen] = useState(false);
    const [backgroundIndex, setBackgroundIndex] = useState(0);
    const [romInfo, setRomInfo] = useState<RomInfo>({});
    const [framerate, setFramerate] = useState(0);
    const [keyaction, setKeyaction] = useState<string>();
    const [modalTitle, setModalTitle] = useState<string>();
    const [modalText, setModalText] = useState<string>();
    const [modalVisible, setModalVisible] = useState(false);
    const [toastText, setToastText] = useState<string>();
    const [toastError, setToastError] = useState(false);
    const [toastVisible, setToastVisible] = useState(false);
    const [keyboardVisible, setKeyboardVisible] = useState(false);
    const [infoVisible, setInfoVisible] = useState(true);
    const [debugVisible, setDebugVisible] = useState(false);

    const toastCounterRef = useRef(0);
    const frameRef = useRef<boolean>(false);
    const errorRef = useRef<boolean>(false);
    const modalCallbackRef =
        useRef<(value: boolean | PromiseLike<boolean>) => void>();

    useEffect(() => {
        document.body.style.backgroundColor = `#${getBackground()}`;
    }, [backgroundIndex]);
    useEffect(() => {
        switch (keyaction) {
            case "Escape":
                setFullscreen(false);
                setKeyaction(undefined);
                break;
            case "Fullscreen":
                setFullscreen(!fullscreen);
                setKeyaction(undefined);
                break;
        }
    }, [keyaction]);
    useEffect(() => {
        const onKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                setKeyaction("Escape");
                event.stopPropagation();
                event.preventDefault();
            }
            if (event.key === "f" && event.ctrlKey === true) {
                setKeyaction("Fullscreen");
                event.stopPropagation();
                event.preventDefault();
            }
        };
        const onBooted = () => {
            setRomInfo(emulator.romInfo);
            setPaused(false);
        };
        const onMessage = (
            emulator: Emulator,
            params: Record<string, any> = {}
        ) => {
            showToast(params.text, params.error, params.timeout);
        };
        document.addEventListener("keydown", onKeyDown);
        emulator.bind("booted", onBooted);
        emulator.bind("message", onMessage);
        return () => {
            document.removeEventListener("keydown", onKeyDown);
            emulator.unbind("booted", onBooted);
            emulator.unbind("message", onMessage);
        };
    }, []);

    const getPauseText = () => (paused ? "Resume" : "Pause");
    const getPauseIcon = () =>
        paused ? require("../res/play.svg") : require("../res/pause.svg");
    const getBackground = () => backgrounds[backgroundIndex];

    const showModal = async (
        text: string,
        title = "Alert"
    ): Promise<boolean> => {
        setModalText(text);
        setModalTitle(title);
        setModalVisible(true);
        const result = (await new Promise((resolve) => {
            modalCallbackRef.current = resolve;
        })) as boolean;
        return result;
    };
    const showToast = async (text: string, error = false, timeout = 3500) => {
        setToastText(text);
        setToastError(error);
        setToastVisible(true);
        toastCounterRef.current++;
        const counter = toastCounterRef.current;
        await new Promise((resolve) => {
            setTimeout(() => {
                if (counter !== toastCounterRef.current) return;
                setToastVisible(false);
                resolve(true);
            }, timeout);
        });
    };

    const onFile = async (file: File) => {
        // @todo must make this more flexible and not just
        // Game Boy only (using the emulator interface)
        if (!file.name.endsWith(".gb")) {
            showToast(
                `This is probably not a ${emulator.device} ROM file!`,
                true
            );
            return;
        }

        const arrayBuffer = await file.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);

        emulator.boot({ engine: null, romName: file.name, romData: romData });

        showToast(`Loaded ${file.name} ROM successfully!`);
    };
    const onModalConfirm = () => {
        if (modalCallbackRef.current) {
            modalCallbackRef.current(true);
            modalCallbackRef.current = undefined;
        }
        setModalVisible(false);
    };
    const onModalCancel = () => {
        if (modalCallbackRef.current) {
            modalCallbackRef.current(false);
            modalCallbackRef.current = undefined;
        }
        setModalVisible(false);
    };
    const onToastCancel = () => {
        setToastVisible(false);
    };
    const onPauseClick = () => {
        emulator.toggleRunning();
        setPaused(!paused);
    };
    const onResetClick = () => {
        emulator.reset();
    };
    const onBenchmarkClick = async () => {
        const result = await showModal(
            "Are you sure you want to start a benchmark?\nThe benchmark is considered an expensive operation!",
            "Confirm"
        );
        if (!result) return;
        const { delta, count, frequency_mhz } = emulator.benchmark();
        await showToast(
            `Took ${delta.toFixed(
                2
            )} seconds to run ${count} ticks (${frequency_mhz.toFixed(
                2
            )} Mhz)!`,
            undefined,
            7500
        );
    };
    const onFullscreenClick = () => {
        setFullscreen(!fullscreen);
    };
    const onKeyboardClick = () => {
        setKeyboardVisible(!keyboardVisible);
    };
    const onInformationClick = () => {
        setInfoVisible(!infoVisible);
    };
    const onDebugClick = () => {
        setDebugVisible(!debugVisible);
    };
    const onThemeClick = () => {
        setBackgroundIndex((backgroundIndex + 1) % backgrounds.length);
    };
    const onUploadFile = async (file: File) => {
        const arrayBuffer = await file.arrayBuffer();
        const romData = new Uint8Array(arrayBuffer);
        emulator.boot({ engine: null, romName: file.name, romData: romData });
        showToast(`Loaded ${file.name} ROM successfully!`);
    };
    const onEngineChange = (engine: string) => {
        emulator.boot({ engine: engine.toLowerCase() });
        showToast(
            `${emulator.device} running in engine "${engine}" from now on!`
        );
    };
    const onFrequencyChange = (value: number) => {
        emulator.frequency = value * 1000 * 1000;
    };
    const onFrequencyReady = (handler: (value: number) => void) => {
        emulator.bind("frequency", (emulator: Emulator, frequency: number) => {
            handler(frequency / 1000000);
        });
    };
    const onMinimize = () => {
        setFullscreen(!fullscreen);
    };
    const onDrawHandler = (handler: DrawHandler) => {
        if (frameRef.current) return;
        frameRef.current = true;
        emulator.bind("frame", () => {
            handler(emulator.imageBuffer, PixelFormat.RGB);
            setFramerate(emulator.framerate);
        });
    };
    const onClearHandler = (handler: ClearHandler) => {
        if (errorRef.current) return;
        errorRef.current = true;
        emulator.bind("error", async () => {
            await handler(undefined, require("../res/storm.png"), 0.2);
        });
    };

    return (
        <div className="app">
            <Overlay text={"Drag to load ROM"} onFile={onFile} />
            <Modal
                title={modalTitle}
                text={modalText}
                visible={modalVisible}
                onConfirm={onModalConfirm}
                onCancel={onModalCancel}
            />
            <Toast
                text={toastText}
                error={toastError}
                visible={toastVisible}
                onCancel={onToastCancel}
            />
            <Footer color={getBackground()}>
                Built with ❤️ by{" "}
                <Link href="https://joao.me" target="_blank">
                    João Magalhães
                </Link>
            </Footer>
            <PanelSplit
                left={
                    <div className="display-container">
                        <Display
                            fullscreen={fullscreen}
                            onDrawHandler={onDrawHandler}
                            onClearHandler={onClearHandler}
                            onMinimize={onMinimize}
                        />
                    </div>
                }
            >
                <Title
                    text={emulator.name}
                    version={emulator.version}
                    versionUrl={
                        emulator.versionUrl ? emulator.versionUrl : undefined
                    }
                    iconSrc={require("../res/thunder.png")}
                ></Title>
                <Section>
                    <Paragraph>
                        This is a{" "}
                        {emulator.deviceUrl ? (
                            <Link href={emulator.deviceUrl} target="_blank">
                                {emulator.device}
                            </Link>
                        ) : (
                            emulator.device
                        )}{" "}
                        emulator built using the{" "}
                        <Link href="https://www.rust-lang.org" target="_blank">
                            Rust Programming Language
                        </Link>{" "}
                        and is running inside this browser with the help of{" "}
                        <Link href="https://webassembly.org/" target="_blank">
                            WebAssembly
                        </Link>
                        .
                    </Paragraph>
                    <Paragraph>
                        You can check the source code of it at{" "}
                        <Link
                            href="https://gitlab.stage.hive.pt/joamag/boytacean"
                            target="_blank"
                        >
                            GitLab
                        </Link>
                        .
                    </Paragraph>
                    <Paragraph>
                        TIP: Drag and Drop ROM files to the Browser to load the
                        ROM.
                    </Paragraph>
                </Section>
                {keyboardVisible && (
                    <Section>
                        <KeyboardGB />
                    </Section>
                )}
                {debugVisible && (
                    <Section>
                        <h3>VRAM Tiles</h3>
                        <Tiles
                            getTile={(index) => emulator.getTile(index)}
                            tileCount={384}
                        />
                    </Section>
                )}
                {infoVisible && (
                    <Section>
                        <Info>
                            <Pair
                                key="button-engine"
                                name={"Engine"}
                                valueNode={
                                    <ButtonSwitch
                                        options={emulator.engines.map((e) =>
                                            e.toUpperCase()
                                        )}
                                        size={"large"}
                                        style={["simple"]}
                                        onChange={onEngineChange}
                                    />
                                }
                            />
                            <Pair
                                key="rom"
                                name={"ROM"}
                                value={romInfo.name ?? "-"}
                            />
                            <Pair
                                key="rom-size"
                                name={"ROM Size"}
                                value={
                                    romInfo.size
                                        ? `${new Intl.NumberFormat().format(
                                              romInfo.size
                                          )} bytes`
                                        : "-"
                                }
                            />
                            <Pair
                                key="button-frequency"
                                name={"CPU Frequency"}
                                valueNode={
                                    <ButtonIncrement
                                        value={emulator.frequency / 1000 / 1000}
                                        delta={0.4}
                                        min={0}
                                        suffix={"MHz"}
                                        decimalPlaces={2}
                                        onChange={onFrequencyChange}
                                        onReady={onFrequencyReady}
                                    />
                                }
                            />
                            <Pair
                                key="rom-type"
                                name={"ROM Type"}
                                value={
                                    romInfo.extra?.romType
                                        ? `${romInfo.extra?.romType}`
                                        : "-"
                                }
                            />
                            <Pair
                                key="framerate"
                                name={"Framerate"}
                                value={`${framerate} fps`}
                            />
                        </Info>
                    </Section>
                )}
                <Section>
                    <ButtonContainer>
                        <Button
                            text={getPauseText()}
                            image={getPauseIcon()}
                            imageAlt="pause"
                            enabled={paused}
                            style={["simple", "border", "padded"]}
                            onClick={onPauseClick}
                        />
                        <Button
                            text={"Reset"}
                            image={require("../res/reset.svg")}
                            imageAlt="reset"
                            style={["simple", "border", "padded"]}
                            onClick={onResetClick}
                        />
                        <Button
                            text={"Benchmark"}
                            image={require("../res/bolt.svg")}
                            imageAlt="benchmark"
                            style={["simple", "border", "padded"]}
                            onClick={onBenchmarkClick}
                        />
                        <Button
                            text={"Fullscreen"}
                            image={require("../res/maximise.svg")}
                            imageAlt="maximise"
                            style={["simple", "border", "padded"]}
                            onClick={onFullscreenClick}
                        />
                        <Button
                            text={"Keyboard"}
                            image={require("../res/dialpad.svg")}
                            imageAlt="keyboard"
                            enabled={keyboardVisible}
                            style={["simple", "border", "padded"]}
                            onClick={onKeyboardClick}
                        />
                        <Button
                            text={"Information"}
                            image={require("../res/info.svg")}
                            imageAlt="information"
                            enabled={infoVisible}
                            style={["simple", "border", "padded"]}
                            onClick={onInformationClick}
                        />
                        <Button
                            text={"Debug"}
                            image={require("../res/bug.svg")}
                            imageAlt="debug"
                            enabled={debugVisible}
                            style={["simple", "border", "padded"]}
                            onClick={onDebugClick}
                        />
                        <Button
                            text={"Theme"}
                            image={require("../res/marker.svg")}
                            imageAlt="theme"
                            style={["simple", "border", "padded"]}
                            onClick={onThemeClick}
                        />
                        <Button
                            text={"Load ROM"}
                            image={require("../res/upload.svg")}
                            imageAlt="upload"
                            file={true}
                            accept={".gb"}
                            style={["simple", "border", "padded"]}
                            onFile={onUploadFile}
                        />
                    </ButtonContainer>
                </Section>
            </PanelSplit>
        </div>
    );
};

export const startApp = (
    element: string,
    emulator: Emulator,
    backgrounds: string[]
) => {
    const elementRef = document.getElementById(element);
    if (!elementRef) {
        return;
    }

    const root = ReactDOM.createRoot(elementRef);
    root.render(<App emulator={emulator} backgrounds={backgrounds} />);
};

export default App;
