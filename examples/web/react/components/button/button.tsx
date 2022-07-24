import React, { FC } from "react";

import "./button.css";

type ButtonProps = {
    text: string;
    image?: string;
    imageAlt?: string;
    size?: string;
    style?: string[];
    onClick?: () => void;
};

export const Button: FC<ButtonProps> = ({
    text,
    image,
    imageAlt,
    size = "small",
    style = ["simple", "border"],
    onClick
}) => {
    const classes = () => ["button", size, ...style].join(" ");
    const _onClick = () => (onClick ? onClick() : undefined);
    const buttonSimple = () => (
        <span className={classes()} onClick={_onClick}>
            {text}
        </span>
    );
    const buttonImage = () => (
        <span className={classes()} onClick={_onClick}>
            <img src={image} alt={imageAlt} />
            <span>{text}</span>
        </span>
    );
    return image ? buttonImage() : buttonSimple();
};

export default Button;
