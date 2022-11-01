import React, { FC } from "react";

import "./overlay.css";

declare const require: any;

type OverlayProps = {
    text?: string;
    style?: string[];
};

export const Overlay: FC<OverlayProps> = ({ text, style = [] }) => {
    const classes = () => ["overlay", ...style].join(" ");
    return (
        <div className={classes()}>
            <div className="overlay-container">
                {text && <div className="overlay-text">{text}</div>}
                <div className="overlay-image">
                    <img alt="sunglasses" src={require("./sunglasses.png")} />
                </div>
            </div>
        </div>
    );
};

export default Overlay;
