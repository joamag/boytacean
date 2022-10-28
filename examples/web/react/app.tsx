import React, { FC, useEffect, useState } from "react";
import ReactDOM from "react-dom/client";

declare const require: any;

import {
    Button,
    ButtonContainer,
    ButtonIncrement,
    ButtonSwitch,
    Display,
    DrawHandler,
    Footer,
    Info,
    Link,
    Pair,
    PanelSplit,
    Paragraph,
    Section,
    Title
} from "./components";

import "./app.css";

/**
 * Top level interface that declares the main abstract
 * interface of an emulator structured entity.
 * Should allow typical hardware operations to be performed.
 */
export interface Emulator {
    getName(): string;
    getVersion(): string;
    getVersionUrl(): string;
    getPixelFormat(): PixelFormat;
    getImageBuffer(): Uint8Array;
    toggleRunning(): void;
    pause(): void;
    resume(): void;
    reset(): void;
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
    const getPauseText = () => (paused ? "Resume" : "Pause");
    const getPauseIcon = () =>
        paused ? require("../res/play.svg") : require("../res/pause.svg");
    const getBackground = () => backgrounds[backgroundIndex];
    const onPauseClick = () => {
        emulator.toggleRunning();
        setPaused(!paused);
    };
    const onResetClick = () => {
        emulator.reset();
    };
    const onFullscreenClick = () => {
        setFullscreen((!fullscreen);
    };
    const onThemeClick = () => {
        setBackgroundIndex((backgroundIndex + 1) % backgrounds.length);
    };
    const onDrawHandler = (handler: DrawHandler) => {
        setInterval(() => {
            handler(emulator.getImageBuffer(), PixelFormat.RGB);
        }, 1000);
    };
    useEffect(() => {
        document.body.style.backgroundColor = `#${getBackground()}`;
    });
    return (
        <div className="app">
            <Footer color={getBackground()}>
                Built with ❤️ by{" "}
                <Link href="https://joao.me" target="_blank">
                    João Magalhães
                </Link>
            </Footer>
            <PanelSplit
                left={
                    <div>
                        <Display fullscreen={fullscreen} onDrawHandler={onDrawHandler} />
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
                            text={"Fullscreen"}
                            image={require("../res/maximise.svg")}
                            imageAlt="maximise"
                            onClick={onFullscreenClick}
                        />
                        <Button
                            text={"Theme"}
                            image={require("../res/marker.svg")}
                            imageAlt="marker"
                            onClick={onThemeClick}
                        />
                    </ButtonContainer>
                    <Info>
                        <Pair key="tobias" name={"Tobias"} value={"Matias"} />
                        <Pair key="matias" name={"Matias"} value={"3"} />
                        <Pair
                            key="button-tobias"
                            name={"Button Increment"}
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
                            key="button-cpu"
                            name={"Button Switch"}
                            valueNode={
                                <ButtonSwitch
                                    options={["NEO", "CLASSIC"]}
                                    size={"large"}
                                    style={["simple"]}
                                    onChange={(v) => alert(v)}
                                />
                            }
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
