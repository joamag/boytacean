import React, { ReactNode, FC, ButtonHTMLAttributes } from "react";
import Button from "../button/button";

import "./modal.css";

declare const require: any;

type ModalProps = {
    title?: string;
    text?: string;
    visible?: boolean;
    style?: string[];
};

export const Modal: FC<ModalProps> = ({
    title = "Alert",
    text = "Do you confirm the following operation?",
    visible = false,
    style = []
}) => {
    const classes = () =>
        ["modal", visible ? "visible" : "", ...style].join(" ");
    return (
        <div className={classes()}>
            <div className="modal-window">
                <div className="modal-top-buttons">
                    <Button
                        text={""}
                        size={"medium"}
                        style={["simple", "rounded", "no-text"]}
                        image={require("./close.svg")}
                    />
                </div>
                <h2 className="modal-title">{title}</h2>
                <p className="modal-text">{text}</p>
                <div className="modal-buttons">
                    <Button
                        text={"Cancel"}
                        size={"medium"}
                        style={["simple", "red", "border", "padded-large"]}
                    />
                    <Button
                        text={"Confirm"}
                        size={"medium"}
                        style={["simple", "border", "padded-large"]}
                    />
                </div>
            </div>
        </div>
    );
};

export default Modal;
