import React, { FC, useState } from "react";
import ReactDOM from "react-dom/client";

import {
    Boytacean,
    useBoytacean,
    useBoytaceanStats,
    useBoytaceanStatus
} from "../component";

import "./index.css";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
declare const require: any;

const ROM_PATH = require("../../../res/roms/demo/pocket.gb");

/**
 * Custom mapping between the physical keyboard and the emulated
 * game pad, replacing the default action keys.
 */
const CUSTOM_KEYS: Record<string, string> = {
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    Enter: "Start",
    " ": "Select",
    z: "A",
    x: "B"
};

/**
 * Custom status bar built on top of the context, showing that the
 * emulator can be controlled from any component under the provider.
 */
const Status: FC = () => {
    const { play, pause } = useBoytacean();
    const { framerate } = useBoytaceanStats();
    const { romName } = useBoytaceanStatus();
    const [paused, setPaused] = useState(false);

    return (
        <div className="status">
            <button
                type="button"
                className="status-button"
                onClick={async () => {
                    if (paused) {
                        await play();
                    } else {
                        await pause();
                    }
                    setPaused(!paused);
                }}
            >
                {paused ? "Resume" : "Pause"}
            </button>
            <span className="status-label">
                {romName ?? "no ROM"} · {framerate} FPS
            </span>
        </div>
    );
};

/**
 * Completely custom game pad, built without any of the components
 * of the package by driving the press and release actions that are
 * exposed by the context.
 */
const CustomKeys: FC = () => {
    const { press, release } = useBoytacean();
    return (
        <div className="keys">
            {["ArrowLeft", "ArrowRight", "A", "B", "Start"].map((key) => (
                <button
                    key={key}
                    type="button"
                    className="key"
                    onMouseDown={() => press(key)}
                    onMouseUp={() => release(key)}
                    onMouseLeave={() => release(key)}
                >
                    {key}
                </button>
            ))}
        </div>
    );
};

const App: FC = () => (
    <div className="app">
        <h1>Boytacean React</h1>

        <h2>Out of the box</h2>
        <p>
            A single element renders the display, binds the keyboard and shows
            the on screen game pad.
        </p>
        <Boytacean rom={ROM_PATH} palette="christmas" className="emulator" />

        <h2>Custom layout</h2>
        <p>
            Providing children switches the component into provider only mode,
            the layout is built with the exposed components and the
            <code> useBoytacean()</code> hook. The physical keyboard is remapped
            here, using <code>Z</code> and <code>X</code> for the action keys
            instead of the defaults.
        </p>
        <Boytacean.Provider rom={ROM_PATH}>
            <div className="emulator">
                <Boytacean.Screen scale={2} className="screen" />
                <Status />
                <Boytacean.Keyboard keys={CUSTOM_KEYS} />
            </div>
        </Boytacean.Provider>

        <h2>Custom keyboard</h2>
        <p>
            No component of the package is used for the input here, the keys are
            driven directly through the <code>press()</code> and
            <code> release()</code> actions of the context.
        </p>
        <Boytacean.Provider rom={ROM_PATH}>
            <div className="emulator">
                <Boytacean.Screen scale={2} className="screen" />
                <CustomKeys />
            </div>
        </Boytacean.Provider>
    </div>
);

const element = document.getElementById("app");
if (element) {
    ReactDOM.createRoot(element).render(<App />);
}
