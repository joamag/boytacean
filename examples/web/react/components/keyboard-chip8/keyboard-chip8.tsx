import React, { FC } from "react";

import "./keyboard-chip8.css";

type KeyboardChip8Props = {
    style?: string[];
    onKeyDown?: (key: string) => void;
};

export const KeyboardChip8: FC<KeyboardChip8Props> = ({
    style = [],
    onKeyDown
}) => {
    const classes = () => ["keyboard", "keyboard-chip8", ...style].join(" ");
    const renderKey = (key: string) => {
        return (
            <span
                className="key"
                key={key}
                onKeyDown={() => onKeyDown && onKeyDown(key)}
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
