import React, { FC, useState } from "react";

import "./keyboard-chip8.css";

type KeyboardChip8Props = {
    focusable?: boolean;
    style?: string[];
    onKeyDown?: (key: string) => void;
    onKeyUp?: (key: string) => void;
};

export const KeyboardChip8: FC<KeyboardChip8Props> = ({
    focusable = true,
    style = [],
    onKeyDown,
    onKeyUp
}) => {
    const classes = () => ["keyboard", "keyboard-chip8", ...style].join(" ");
    const renderKey = (key: string, styles: string[] = []) => {
        const [pressed, setPressed] = useState(false);
        const classes = ["key", pressed ? "pressed" : "", ...styles].join(" ");
        return (
            <span
                className={classes}
                key={key}
                tabIndex={focusable ? 0 : undefined}
                onKeyDown={(event) => {
                    if (event.key !== "Enter") return;
                    setPressed(true);
                    onKeyDown && onKeyDown(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onKeyUp={(event) => {
                    if (event.key !== "Enter") return;
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onBlur={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                }}
                onMouseDown={(event) => {
                    setPressed(true);
                    onKeyDown && onKeyDown(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onMouseUp={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onMouseLeave={(event) => {
                    if (!pressed) return;
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onTouchStart={(event) => {
                    setPressed(true);
                    onKeyDown && onKeyDown(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
                onTouchEnd={(event) => {
                    setPressed(false);
                    onKeyUp && onKeyUp(key);
                    event.stopPropagation();
                    event.preventDefault();
                }}
            >
                {key}
            </span>
        );
    };
    return (
        <div className={classes()}>
            <div className="keyboard-line">
                {["1", "2", "3", "4"].map((k) => renderKey(k))}
            </div>
            <div className="keyboard-line">
                {["Q", "W", "E", "R"].map((k) => renderKey(k))}
            </div>
            <div className="keyboard-line">
                {["A", "S", "D", "F"].map((k) => renderKey(k))}
            </div>
            <div className="keyboard-line">
                {["Z", "X", "C", "V"].map((k) => renderKey(k))}
            </div>
        </div>
    );
};

export default KeyboardChip8;
