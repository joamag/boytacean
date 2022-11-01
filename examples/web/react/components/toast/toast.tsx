import React, { FC } from "react";

import "./toast.css";

type ToastProps = {
    text?: string;
    error?: boolean;
    visible?: boolean;
    style?: string[];
    onCancel?: () => void;
};

export const Toast: FC<ToastProps> = ({
    text = "",
    error = false,
    visible = false,
    style = [],
    onCancel
}) => {
    const classes = () =>
        [
            "toast",
            error ? "error" : "",
            visible ? "visible" : "",
            ...style
        ].join(" ");
    return (
        <div className={classes()}>
            <div className="toast-text" onClick={onCancel}>
                {text}
            </div>
        </div>
    );
};

export default Toast;
