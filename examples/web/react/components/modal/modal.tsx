import React, { FC, useEffect } from "react";
import Button from "../button/button";

import "./modal.css";

declare const require: any;

type ModalProps = {
    title?: string;
    text?: string;
    visible?: boolean;
    overlayClose?: boolean;
    style?: string[];
    onConfirm?: () => void;
    onCancel?: () => void;
};

export const Modal: FC<ModalProps> = ({
    title = "Alert",
    text = "Do you confirm the following operation?",
    visible = false,
    overlayClose = true,
    style = [],
    onConfirm,
    onCancel
}) => {
    const classes = () =>
        ["modal", visible ? "visible" : "", ...style].join(" ");
    useEffect(() => {
        const onKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                onCancel && onCancel();
            }
        };
        document.addEventListener("keydown", onKeyDown);
        return () => {
            document.removeEventListener("keydown", onKeyDown);
        };
    }, []);
    const getTextHtml = (separator = /\n/g) => ({
        __html: text.replace(separator, "<br/>")
    });
    const onWindowClick = (
        event: React.MouseEvent<HTMLDivElement, MouseEvent>
    ) => {
        if (!overlayClose) return;
        event.stopPropagation();
    };
    return (
        <div className={classes()} onClick={onCancel}>
            <div className="modal-window" onClick={onWindowClick}>
                <div className="modal-top-buttons">
                    <Button
                        text={""}
                        size={"medium"}
                        style={["simple", "rounded", "no-text"]}
                        image={require("./close.svg")}
                        imageAlt="close"
                        onClick={onCancel}
                    />
                </div>
                <h2 className="modal-title">{title}</h2>
                <p
                    className="modal-text"
                    dangerouslySetInnerHTML={getTextHtml()}
                ></p>
                <div className="modal-buttons">
                    <Button
                        text={"Cancel"}
                        size={"medium"}
                        style={["simple", "red", "border", "padded-large"]}
                        onClick={onCancel}
                    />
                    <Button
                        text={"Confirm"}
                        size={"medium"}
                        style={["simple", "border", "padded-large"]}
                        onClick={onConfirm}
                    />
                </div>
            </div>
        </div>
    );
};

export default Modal;
