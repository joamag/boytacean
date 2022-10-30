import React, { ReactNode, FC, ButtonHTMLAttributes, useEffect } from "react";
import Button from "../button/button";

import "./modal.css";

declare const require: any;

type ModalProps = {
    title?: string;
    text?: string;
    visible?: boolean;
    style?: string[];
    onConfirm?: () => void;
    onCancel?: () => void;
};

export const Modal: FC<ModalProps> = ({
    title = "Alert",
    text = "Do you confirm the following operation?",
    visible = false,
    style = [],
    onConfirm,
    onCancel
}) => {
    const classes = () =>
        ["modal", visible ? "visible" : "", ...style].join(" ");
    useEffect(() => {
        document.addEventListener("keydown", (event) => {
            if (event.key === "Escape") {
                onCancel && onCancel();
            }
        });
    }, []);
    const getTextHtml = (separator = /\n/g) => ({
        __html: text.replace(separator, "<br/>")
    });
    return (
        <div className={classes()}>
            <div className="modal-window">
                <div className="modal-top-buttons">
                    <Button
                        text={""}
                        size={"medium"}
                        style={["simple", "rounded", "no-text"]}
                        image={require("./close.svg")}
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
