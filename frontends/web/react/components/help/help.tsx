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
                    <span className="key">Enter</span> - Start
                </li>
                <li>
                    <span className="key">Space</span> - Select
                </li>
                <li>
                    <span className="key">A</span> - A
                </li>
                <li>
                    <span className="key">S</span> - B
                </li>
                <li>
                    <span className="key">Escape</span> - Exit fullscreen
                </li>
                <li>
                    <span className="key">Ctrl + D</span> - Turbo speed
                </li>
                <li>
                    <span className="key">Ctrl + F</span> - Toggle fullscreen
                </li>
                <li>
                    <span className="key">Ctrl + K</span> - Toggle on-screen
                    keyboard
                </li>
            </ul>
        </div>
    );
};

export default Help;
