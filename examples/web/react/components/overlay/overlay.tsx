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
        const onDrop = async (event: DragEvent) => {
            if (!event.dataTransfer!.items) return;
            if (event.dataTransfer!.items[0].type) return;

            setVisible(false);

            const file = event.dataTransfer!.files[0];
            onFile && onFile(file);

            event.preventDefault();
            event.stopPropagation();
        };
        const onDragOver = async (event: DragEvent) => {
            if (!event.dataTransfer!.items) return;
            if (event.dataTransfer!.items[0].type) return;
            setVisible(true);
            event.preventDefault();
        };
        const onDragEnter = async (event: DragEvent) => {
            if (!event.dataTransfer!.items) return;
            if (event.dataTransfer!.items[0].type) return;
            setVisible(true);
        };
        const onDragLeave = async (event: DragEvent) => {
            if (!event.dataTransfer!.items) return;
            if (event.dataTransfer!.items[0].type) return;
            setVisible(false);
        };
        document.addEventListener("drop", onDrop);
        document.addEventListener("dragover", onDragOver);
        document.addEventListener("dragenter", onDragEnter);
        document.addEventListener("dragleave", onDragLeave);
        return () => {
            document.removeEventListener("drop", onDrop);
            document.removeEventListener("dragover", onDragOver);
            document.removeEventListener("dragenter", onDragEnter);
            document.removeEventListener("dragleave", onDragLeave);
        };
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
