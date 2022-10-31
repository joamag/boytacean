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
    Pair,
    PanelSplit,
    Paragraph,
    Section,
    Title,
    Toast
} from "./components";

import "./app.css";

export type Callback<T> = (owner: T, params?: Record<string, any>) => void;

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

    trigger(event: string, params?: Record<string, any>) {
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

export interface ObservableI {
    bind(event: string, callback: Callback<this>): void;
    trigger(event: string): void;
}

/**
 * Top level interface that declares the main abstract
 * interface of an emulator structured entity.
 * Should allow typical hardware operations to be performed.
 */
export interface Emulator extends ObservableI {
    /**
     * Obtains the descriptive name of the emulator.
     *
     * @returns The descriptive name of the emulator.
     */
    getName(): string;

    /**
     * Obtains a semantic version string for the current
     * version of the emulator.
     *
     * @returns The semantic version string.
     * @see {@link https://semver.org}
     */
    getVersion(): string;

    /**
     * Obtains a URL to the page describing the current version
     * of the emulator.
     *
     * @returns A URL to the page describing the current version
     * of the emulator.
     */
    getVersionUrl(): string;

    /**
     * Obtains the pixel format of the emulator's display
     * image buffer (eg: RGB).
     *
     * @returns The pixel format used for the emulator's
     * image buffer.
     */
    getPixelFormat(): PixelFormat;

    /**
     * Obtains the complete image buffer as a sequence of
     * bytes that respects the current pixel format from
     * `getPixelFormat()`. This method returns an in memory
     * pointer to the heap and not a copy.
     *
     * @returns The byte based image buffer that respects
     * the emulator's pixel format.
     */
    getImageBuffer(): Uint8Array;

    /**
     * Obtains information about the ROM that is currently
     * loaded in the emulator.
     *
     * @returns Structure containing the information about
     * the ROM that is currently loaded in the emulator.
     */
    getRomInfo(): RomInfo;

    /**
     * Returns the current logic framerate of the running
     * emulator.
     *
     * @return The current logic framerate of the running
     * emulator.
     */
    getFramerate(): number;

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
     * Boot (or reboots) the emulator according to the provided
     * set of options.
     *
     * @param options The options that are going to be used for
     * the booting operation of the emulator.
     */
    boot(options: any): void;
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
        document.addEventListener("keydown", (event) => {
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
        });
        emulator.bind("booted", () => {
            const romInfo = emulator.getRomInfo();
            setRomInfo(romInfo);
        });
        emulator.bind("message", (_, params = {}) => {
            showToast(params.text, params.error, params.timeout);
        });
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
        await showToast(
            result
                ? "Will run the benchmark as fast as possible"
                : "Will not run the benchmark",
            !result
        );
    };
    const onFullscreenClick = () => {
        setFullscreen(!fullscreen);
    };
    const onKeyboardClick = () => {
        showToast("Keyboard click");
    };
    const onInformationClick = () => {
        showToast("Information click");
    };
    const onThemeClick = () => {
        setBackgroundIndex((backgroundIndex + 1) % backgrounds.length);
    };
    const onEngineChange = (engine: string) => {
        emulator.boot({ engine: engine.toLowerCase() });
        showToast(`Game Boy running in engine "${engine}" from now on!`);
    };
    const onMinimize = () => {
        setFullscreen(!fullscreen);
    };
    const onDrawHandler = (handler: DrawHandler) => {
        if (frameRef.current) return;
        frameRef.current = true;
        emulator.bind("frame", () => {
            handler(emulator.getImageBuffer(), PixelFormat.RGB);
            setFramerate(emulator.getFramerate());
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
                        <KeyboardGB />
                    </div>
                }
            >
                <Title
                    text={emulator.getName()}
                    version={emulator.getVersion()}
                    versionUrl={emulator.getVersionUrl()}
                    iconSrc={require("../res/thunder.png")}
                ></Title>
                <Section>
                    <Paragraph>
                        This is a{" "}
                        <Link
                            href="https://en.wikipedia.org/wiki/Game_Boy"
                            target="_blank"
                        >
                            Game Boy
                        </Link>{" "}
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
                <Section>
                    <ButtonContainer>
                        <Button
                            text={getPauseText()}
                            image={getPauseIcon()}
                            imageAlt="pause"
                            onClick={onPauseClick}
                        />
                        <Button
                            text={"Reset"}
                            image={require("../res/reset.svg")}
                            imageAlt="reset"
                            onClick={onResetClick}
                        />
                        <Button
                            text={"Benchmark"}
                            image={require("../res/bolt.svg")}
                            imageAlt="benchmark"
                            onClick={onBenchmarkClick}
                        />
                        <Button
                            text={"Fullscreen"}
                            image={require("../res/maximise.svg")}
                            imageAlt="maximise"
                            onClick={onFullscreenClick}
                        />
                        <Button
                            text={"Keyboard"}
                            image={require("../res/dialpad.svg")}
                            imageAlt="keyboard"
                            onClick={onKeyboardClick}
                        />
                        <Button
                            text={"Information"}
                            image={require("../res/info.svg")}
                            imageAlt="iformation"
                            onClick={onInformationClick}
                        />
                        <Button
                            text={"Theme"}
                            image={require("../res/marker.svg")}
                            imageAlt="theme"
                            onClick={onThemeClick}
                        />
                    </ButtonContainer>
                    <Info>
                        <Pair
                            key="button-engine"
                            name={"Engine"}
                            valueNode={
                                <ButtonSwitch
                                    options={["NEO", "CLASSIC"]}
                                    size={"large"}
                                    style={["simple"]}
                                    onChange={onEngineChange}
                                />
                            }
                        />
                        <Pair
                            key="button-frequency"
                            name={"CPU Frequency"}
                            valueNode={
                                <ButtonIncrement
                                    value={200}
                                    delta={100}
                                    min={0}
                                    suffix={"Hz"}
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
                            value={romInfo.name ? `${romInfo.size} bytes` : "-"}
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
