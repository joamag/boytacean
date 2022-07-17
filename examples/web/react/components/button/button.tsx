import React, { FC } from "react";

import "./button.css";

type ButtonProps = {
    text: string;
    size?: string;
    style?: string[];
    onClick?: () => void;
};

export const Button: FC<ButtonProps> = ({
    text,
    size = "small",
    style = ["simple", "border"],
    onClick
}) => {
    const classes = () => ["button", size, ...style].join(" ");
    const _onClick = () => (onClick ? onClick() : undefined);
    return (
        <span className={classes()} onClick={_onClick}>
            {text}
        </span>
    );
};

export default Button;
