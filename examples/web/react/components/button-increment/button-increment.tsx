import React, { FC, useState } from "react";
import Button from "../button/button";

import "./button-increment.css";

type ButtonIncrementProps = {
    value: number;
    delta?: number;
    size?: string;
    style?: string[];
    onClick?: () => void;
    onChange?: (value: number) => void;
};

export const ButtonIncrement: FC<ButtonIncrementProps> = ({
    value,
    delta = 1,
    size = "medium",
    style = ["simple", "border"],
    onClick,
    onChange
}) => {
    const [valueState, setValue] = useState(value);
    const classes = () => ["button-increment", size, ...style].join(" ");
    const _onClick = () => {
        if (onClick) onClick();
    };
    const _onMinusClick = () => {
        const valueNew = valueState - delta;
        setValue(valueNew);
        if (onChange) onChange(valueNew);
    };
    const _onPlusClick = () => {
        const valueNew = valueState + delta;
        setValue(valueNew);
        if (onChange) onChange(valueNew);
    };
    return (
        <span className={classes()} onClick={_onClick}>
            <Button
                text={"-"}
                size={size}
                style={["simple"]}
                onClick={_onMinusClick}
            />
            <span className="value">{valueState}</span>
            <Button
                text={"+"}
                size={size}
                style={["simple"]}
                onClick={_onPlusClick}
            />
        </span>
    );
};

export default ButtonIncrement;
