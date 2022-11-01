import React, { FC, useEffect, useState } from "react";

import "./overlay.css";

declare const require: any;

type OverlayProps = {
    text?: string;
    style?: string[];
    onFile?: (file: File) => void;
};

export const Overlay: FC<OverlayProps> = ({ text, style = [], onFile }) => {
    const [visible, setVisible] = useState(false);
    const classes = () =>
        ["overlay", visible ? "visible" : "", ...style].join(" ");
    useEffect(() => {
        document.addEventListener("drop", async (event) => {
            if (
                !event.dataTransfer!.files ||
                event.dataTransfer!.files.length === 0
            ) {
                return;
            }

            event.preventDefault();
            event.stopPropagation();

            setVisible(false);

            const file = event.dataTransfer!.files[0];
            onFile && onFile(file);
        });
        document.addEventListener("dragover", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;

            event.preventDefault();

            setVisible(true);
        });
        document.addEventListener("dragenter", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;
            setVisible(true);
        });
        document.addEventListener("dragleave", async (event) => {
            if (!event.dataTransfer!.items || event.dataTransfer!.items[0].type)
                return;
            setVisible(false);
        });
    }, []);
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
