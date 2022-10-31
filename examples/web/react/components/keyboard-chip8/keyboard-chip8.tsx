import React, { FC } from "react";

import "./keyboard-chip8.css";

type KeyboardChip8Props = {
    style?: string[];
};

export const KeyboardChip8: FC<KeyboardChip8Props> = ({ style = [] }) => {
    const classes = () => ["keyboard", "keyboard-chip8", ...style].join(" ");
    return (
        <div className={classes()}>
            <div className="keyboard-line">
                <span className="key">1</span>
                <span className="key">2</span>
                <span className="key">3</span>
                <span className="key">4</span>
            </div>
            <div className="keyboard-line">
                <span className="key">Q</span>
                <span className="key">W</span>
                <span className="key">E</span>
                <span className="key">R</span>
            </div>
            <div className="keyboard-line">
                <span className="key">A</span>
                <span className="key">S</span>
                <span className="key">D</span>
                <span className="key">F</span>
            </div>
            <div className="keyboard-line">
                <span className="key">Z</span>
                <span className="key">X</span>
                <span className="key">C</span>
                <span className="key">V</span>
            </div>
        </div>
    );
};

export default KeyboardChip8;
