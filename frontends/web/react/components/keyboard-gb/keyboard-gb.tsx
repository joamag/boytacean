import React, { FC, useEffect, useRef, useState } from "react";
import { isAndroid } from "../../util";

import "./keyboard-gb.css";

const KEYS: Record<string, string> = {
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    Enter: "Start",
    " ": "Select",
    a: "A",
    s: "B"
};

const KEYS_XBOX: Record<number, string> = {
    12: "ArrowUp",
    102: "ArrowUp",
    13: "ArrowDown",
    103: "ArrowDown",
    14: "ArrowLeft",
    100: "ArrowLeft",
    15: "ArrowRight",
    101: "ArrowRight",
    9: "Start",
    8: "Select",
    1: "A",
    0: "B"
};

const PREVENT_KEYS: Record<string, boolean> = {
    ArrowUp: true,
    ArrowDown: true,
    ArrowLeft: true,
    ArrowRight: true,
    " ": true
};

declare const require: any;

enum Gamepad {
    Unknown = 1,
    Xbox,
    Playstation,
    Switch
}

type KeyboardGBProps = {
    focusable?: boolean;
    fullscreen?: boolean;
    physical?: boolean;
    selectedKeys?: string[];
    style?: string[];
    onKeyDown?: (key: string) => void;
    onKeyUp?: (key: string) => void;
    onGamepad?: (id: string, isValid: boolean, connected?: boolean) => void;
};

/**
 * The sequence of game pads that are considered
 * supported by the current implementation.
 */
const SUPPORTED_PADS = [Gamepad.Xbox];

export const KeyboardGB: FC<KeyboardGBProps> = ({
    focusable = true,
    fullscreen = false,
    physical = true,
    selectedKeys = [],
    style = [],
    onKeyDown,
    onKeyUp,
    onGamepad
}) => {
    const containerClasses = () =>
        ["keyboard-container", fullscreen ? "fullscreen" : ""].join(" ");
    const recordRef =
        useRef<Record<string, React.Dispatch<React.SetStateAction<boolean>>>>();
    const classes = () =>
        [
            "keyboard",
            "keyboard-gb",
            fullscreen ? "fullscreen" : "",
            ...style
        ].join(" ");
    useEffect(() => {
        if (!physical) return;
        const getGamepadType = (gamepad: globalThis.Gamepad): Gamepad => {
            let gamepadType = Gamepad.Unknown;
            const isXbox = gamepad.id.includes("Xbox");
            if (isXbox) gamepadType = Gamepad.Xbox;
            return gamepadType;
        };
        const _onKeyDown = (event: KeyboardEvent) => {
            const keyCode = KEYS[event.key];
            const isPrevent = PREVENT_KEYS[event.key] ?? false;
            if (isPrevent) event.preventDefault();
            if (keyCode !== undefined) {
                const records = recordRef.current ?? {};
                const setter = records[keyCode];
                setter(true);
                onKeyDown && onKeyDown(keyCode);
                return;
            }
        };
        const _onKeyUp = (event: KeyboardEvent) => {
            const keyCode = KEYS[event.key];
            const isPrevent = PREVENT_KEYS[event.key] ?? false;
            if (isPrevent) event.preventDefault();
            if (keyCode !== undefined) {
                const records = recordRef.current ?? {};
                const setter = records[keyCode];
                setter(false);
                onKeyUp && onKeyUp(keyCode);
                return;
            }
        };
        const onGamepadConnected = (event: GamepadEvent) => {
            const gamepad = event.gamepad;
            let gamepadType = getGamepadType(gamepad);
            const isValid = SUPPORTED_PADS.includes(gamepadType);

            onGamepad && onGamepad(gamepad.id, isValid);

            let keySolver: Record<number, string>;
            switch (gamepadType) {
                case Gamepad.Xbox:
                    keySolver = KEYS_XBOX;
                    break;
            }

            const buttonStates: Record<number, boolean> = {};

            const updateStatus = () => {
                const _gamepad = navigator.getGamepads()[gamepad.index];
                if (!_gamepad) return;

                handleButton(100, _gamepad.axes[0] < -0.5);
                handleButton(101, _gamepad.axes[0] > 0.5);
                handleButton(102, _gamepad.axes[1] < -0.5);
                handleButton(103, _gamepad.axes[1] > 0.5);

                _gamepad.buttons.forEach((button, index) => {
                    const pressed = button.pressed;
                    handleButton(index, pressed);
                });

                requestAnimationFrame(updateStatus);
            };

            const handleButton = (index: number, pressed: boolean) => {
                const keyCode = keySolver[index];
                if (keyCode === undefined) return;
                const state = buttonStates[index] ?? false;
                const keyDown = pressed && !state;
                const keyUp = !pressed && state;

                if (keyDown) {
                    const records = recordRef.current ?? {};
                    const setter = records[keyCode];
                    setter(true);
                    onKeyDown && onKeyDown(keyCode);
                }

                if (keyUp) {
                    const records = recordRef.current ?? {};
                    const setter = records[keyCode];
                    setter(false);
                    onKeyUp && onKeyUp(keyCode);
                }

                buttonStates[index] = pressed;
            };

            requestAnimationFrame(updateStatus);
        };
        const onGamepadDisconnected = (event: GamepadEvent) => {
            const gamepad = event.gamepad;
            let gamepadType = getGamepadType(gamepad);
            const isValid = SUPPORTED_PADS.includes(gamepadType);

            onGamepad && onGamepad(gamepad.id, isValid, false);
        };
        document.addEventListener("keydown", _onKeyDown);
        document.addEventListener("keyup", _onKeyUp);
        window.addEventListener("gamepadconnected", onGamepadConnected);
        window.addEventListener("gamepaddisconnected", onGamepadDisconnected);
        return () => {
            document.removeEventListener("keydown", _onKeyDown);
            document.removeEventListener("keyup", _onKeyUp);
            window.removeEventListener("gamepadconnected", onGamepadConnected);
            window.removeEventListener(
                "gamepadconnected",
                onGamepadDisconnected
            );
        };
    }, []);
    const renderKey = (
        key: string,
        keyName?: string,
        selected = false,
        styles: string[] = []
    ) => {
        const [pressed, setPressed] = useState(selected);
        const classes = ["key", pressed ? "pressed" : "", ...styles].join(" ");
        const records = recordRef.current ?? {};
        records[keyName ?? key ?? "undefined"] = setPressed;
        recordRef.current = records;
        return (
            <span
                className={classes}
                key={keyName ?? key}
                tabIndex={focusable ? 0 : undefined}
                onKeyDown={(event) => {
                    if (event.key !== "Enter") return;
                    setPressed(true);
                    onKeyDown && onKeyDown(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onKeyUp={(event) => {
                    if (event.key !== "Enter") return;
                    setPressed(false);
                    onKeyUp && onKeyUp(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onBlur={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                }}
                onMouseDown={(event) => {
                    setPressed(true);
                    onKeyDown && onKeyDown(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onMouseUp={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onMouseLeave={(event) => {
                    if (!pressed) return;
                    setPressed(false);
                    onKeyUp && onKeyUp(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onTouchStart={(event) => {
                    setPressed(true);
                    onKeyDown && onKeyDown(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onTouchEnd={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(keyName ?? key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
            >
                {key}
            </span>
        );
    };
    return (
        <div className={containerClasses()}>
            <div
                className={classes()}
                onTouchStart={(e) => e.preventDefault()}
                onTouchEnd={(e) => e.preventDefault()}
            >
                <div className="dpad">
                    <div className="dpad-top">
                        {renderKey(
                            isAndroid() ? "▲" : "▲",
                            "ArrowUp",
                            selectedKeys.includes("ArrowUp"),
                            ["up"]
                        )}
                    </div>
                    <div>
                        {renderKey(
                            isAndroid() ? "◀" : "◄",
                            "ArrowLeft",
                            selectedKeys.includes("ArrowLeft"),
                            ["left"]
                        )}
                        {renderKey(
                            isAndroid() ? "▶" : "►",
                            "ArrowRight",
                            selectedKeys.includes("ArrowRight"),
                            ["right"]
                        )}
                    </div>
                    <div className="dpad-bottom">
                        {renderKey(
                            isAndroid() ? "▼" : "▼",
                            "ArrowDown",
                            selectedKeys.includes("ArrowDown"),
                            ["down"]
                        )}
                    </div>
                </div>
                <div className="action">
                    {renderKey("B", "B", selectedKeys.includes("B"), ["b"])}
                    {renderKey("A", "A", selectedKeys.includes("A"), ["a"])}
                </div>
                <div className="break"></div>
                <div className="options">
                    {renderKey(
                        "SELECT",
                        "Select",
                        selectedKeys.includes("Select"),
                        ["select"]
                    )}
                    {renderKey(
                        "START",
                        "Start",
                        selectedKeys.includes("Start"),
                        ["start"]
                    )}
                </div>
            </div>
        </div>
    );
};

export default KeyboardGB;
