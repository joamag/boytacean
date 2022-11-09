import React, { ChangeEvent, FC } from "react";

import "./button.css";

type ButtonProps = {
    text: string;
    image?: string;
    imageAlt?: string;
    enabled?: boolean;
    focusable?: boolean;
    file?: boolean;
    accept?: string;
    size?: string;
    style?: string[];
    onClick?: () => void;
    onFile?: (file: File) => void;
};

export const Button: FC<ButtonProps> = ({
    text,
    image,
    imageAlt,
    enabled = false,
    focusable = false,
    file = false,
    accept = ".txt",
    size = "small",
    style = ["simple", "border"],
    onClick,
    onFile
}) => {
    const classes = () =>
        [
            "button",
            size,
            enabled ? "enabled" : "",
            file ? "file" : "",
            ...style
        ].join(" ");
    const onFileChange = (event: ChangeEvent<HTMLInputElement>) => {
        if (!event.target.files || event.target.files.length === 0) {
            return;
        }
        const file = event.target.files[0];
        onFile && onFile(file);
        event.target.value = "";
    };
    const onKeyPress = (event: React.KeyboardEvent) => {
        if (event.key !== "Enter") return;
        onClick && onClick();
    };
    const renderSimple = () => (
        <span
            className={classes()}
            onClick={onClick}
            onKeyPress={onKeyPress}
            tabIndex={focusable ? 0 : undefined}
        >
            {text}
        </span>
    );
    const renderComplex = () => (
        <span
            className={classes()}
            onClick={onClick}
            onKeyPress={onKeyPress}
            tabIndex={focusable ? 0 : undefined}
        >
            {image && <img src={image} alt={imageAlt || text || "button"} />}
            {file && (
                <input type="file" accept={accept} onChange={onFileChange} />
            )}
            <span>{text}</span>
        </span>
    );
    return image ? renderComplex() : renderSimple();
};

export default Button;
