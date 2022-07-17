import React, { FC } from "react";

import "./button.css";

export const Button: FC<{ text: string, style?: string[] }> = ({ text, style = ["tiny", "border"] }) => {
    const onClick = () => {
        alert("Hello World");
    }

    const classes = () => ["button", ...style].join(" ")

    return <span className={classes()} onClick={onClick}>{ text }</span>;
}
