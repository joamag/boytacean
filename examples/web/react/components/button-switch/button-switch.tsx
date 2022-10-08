import React, { FC, useState } from "react";
import Button from "../button/button";

type ButtonSwitchProps = {
    options: string[];
    size?: string;
    style?: string[];
    onClick?: () => void;
    onChange?: (value: string, index: number) => void;
};

export const ButtonSwitch: FC<ButtonSwitchProps> = ({
    options,
    size = "small",
    style = ["simple", "border"],
    onClick,
    onChange
}) => {
    const [index, setIndex] = useState(0);
    const text = () => options[index];
    const _onClick = () => {
        const indexNew = (index + 1) % options.length;
        const option = options[indexNew];
        setIndex(indexNew);
        if (onClick) onClick();
        if (onChange) onChange(option, indexNew);
    };
    return (
        <Button text={text()} size={size} style={style} onClick={_onClick} />
    );
};

export default ButtonSwitch;
