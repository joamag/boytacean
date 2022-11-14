import React, { FC, useEffect, useRef, useState } from "react";

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

const PREVENT_KEYS: Record<string, boolean> = {
    ArrowUp: true,
    ArrowDown: true,
    ArrowLeft: true,
    ArrowRight: true,
    " ": true
};

declare const require: any;

type KeyboardGBProps = {
    focusable?: boolean;
    fullscreen?: boolean;
    physical?: boolean;
    selectedKeys?: string[];
    style?: string[];
    onKeyDown?: (key: string) => void;
    onKeyUp?: (key: string) => void;
};

export const KeyboardGB: FC<KeyboardGBProps> = ({
    focusable = true,
    fullscreen = false,
    physical = true,
    selectedKeys = [],
    style = [],
    onKeyDown,
    onKeyUp
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
            
            console.log(
                "Gamepad connected at index %d: %s. %d buttons, %d axes.",
                event.gamepad.index,
                event.gamepad.id,
                event.gamepad.buttons.length,
                event.gamepad.axes.length
            );

            const updateStatus = () => {
                event.gamepad.buttons.forEach((button, index) => {
                    if (button.pressed) {
                        console.info(`${index} => ${button.pressed}`);
                    }
                });
                requestAnimationFrame(updateStatus);
            };

            requestAnimationFrame(updateStatus);
        };
        document.addEventListener("keydown", _onKeyDown);
        document.addEventListener("keyup", _onKeyUp);
        window.addEventListener("gamepadconnected", onGamepadConnected);
        return () => {
            document.removeEventListener("keydown", _onKeyDown);
            document.removeEventListener("keyup", _onKeyUp);
            window.removeEventListener("gamepadconnected", onGamepadConnected);
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
                            "▲",
                            "ArrowUp",
                            selectedKeys.includes("ArrowUp"),
                            ["up"]
                        )}
                    </div>
                    <div>
                        {renderKey(
                            "◄",
                            "ArrowLeft",
                            selectedKeys.includes("ArrowLeft"),
                            ["left"]
                        )}
                        {renderKey(
                            "►",
                            "ArrowRight",
                            selectedKeys.includes("ArrowRight"),
                            ["right"]
                        )}
                    </div>
                    <div className="dpad-bottom">
                        {renderKey(
                            "▼",
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
