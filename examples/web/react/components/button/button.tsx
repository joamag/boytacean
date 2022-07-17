import React, { FC } from "react";

import "./button.css";

export const Button: FC<{ text: string; size?: string; style?: string[] }> = ({
    text,
    size = "small",
    style = ["simple", "border"]
}) => {
    const onClick = () => {
        alert("Hello World");
    };

    const classes = () => ["button", size, ...style].join(" ");

    return (
        <span className={classes()} onClick={onClick}>
            {text}
        </span>
    );
};
