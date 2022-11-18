import React, { FC } from "react";

import "./help.css";

type HelpProps = {
    style?: string[];
};

export const Help: FC<HelpProps> = ({ style = [] }) => {
    const classes = () => ["help", ...style].join(" ");
    return (
        <div className={classes()}>
            <ul>
                <li>
                    <span className="key-container">
                        <span className="key">Enter</span>
                    </span>
                    Start button
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">Space</span>
                    </span>
                    Select button
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">A</span>
                    </span>
                    A button
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">S</span>
                    </span>
                    B button
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">Escape</span>
                    </span>
                    Exit fullscreen
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">Ctrl + D</span>
                    </span>
                    Turbo speed
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">Ctrl + F</span>
                    </span>
                    Toggle fullscreen
                </li>
                <li>
                    <span className="key-container">
                        <span className="key">Ctrl + K</span>
                    </span>
                    Toggle on-screen keyboard
                </li>
            </ul>
        </div>
    );
};

export default Help;
