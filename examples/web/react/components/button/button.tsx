import React, { ChangeEvent, FC } from "react";

import "./button.css";

type ButtonProps = {
    text: string;
    image?: string;
    imageAlt?: string;
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
    file = false,
    accept = ".txt",
    size = "small",
    style = ["simple", "border"],
    onClick,
    onFile
}) => {
    const classes = () =>
        ["button", size, file ? "file" : "", ...style].join(" ");
    const onFileChange = (event: ChangeEvent<HTMLInputElement>) => {
        if (!event.target.files || event.target.files.length === 0) {
            return;
        }
        const file = event.target.files[0];
        onFile && onFile(file);
        event.target.value = "";
    };
    const renderButtonSimple = () => (
        <span className={classes()} onClick={onClick}>
            {text}
        </span>
    );
    const renderButtonComplex = () => (
        <span className={classes()} onClick={onClick}>
            {image && <img src={image} alt={imageAlt || text || "button"} />}
            {file && (
                <input type="file" accept={accept} onChange={onFileChange} />
            )}
            <span>{text}</span>
        </span>
    );
    return image ? renderButtonComplex() : renderButtonSimple();
};

export default Button;
