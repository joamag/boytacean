import React, { FC, useState } from "react";

import "./display.css";

declare const require: any;

type DisplayOptions = {
    width: number;
    height: number;
    scale?: number;
};

type DisplayProps = {
    options?: DisplayOptions;
    size?: string;
    style?: string[];
};

export const Display: FC<DisplayProps> = ({
    options = {},
    size = "small",
    style = []
}) => {
    options = { ...options, ...{ width: 320, height: 288 } };
    const classes = () => ["display", size, ...style].join(" ");
    if (!options.scale) {
        options.scale = window.devicePixelRatio ? window.devicePixelRatio : 1;
    }
    return (
        <div id="display" className={classes()}>
            <span id="display-close" className="magnify-button canvas-close">
                <img
                    className="large"
                    src={require("./minimise.svg")}
                    alt="minimise"
                />
            </span>
            <div className="display-frame">
                <canvas
                    id="display-canvas"
                    className="display-canvas"
                    width={options.width * options.scale}
                    height={options.height * options.scale}
                ></canvas>
            </div>
        </div>
    );
};

export default Display;
