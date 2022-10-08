import React, { FC, useState } from "react";
import Button from "../button/button";

import "./button-increment.css";

type ButtonIncrementProps = {
    value: number;
    delta?: number;
    min?: number;
    max?: number;
    prefix?: string;
    suffix?: string;
    size?: string;
    style?: string[];
    onClick?: () => void;
    onBeforeChange?: (value: number) => boolean;
    onChange?: (value: number) => void;
};

export const ButtonIncrement: FC<ButtonIncrementProps> = ({
    value,
    delta = 1,
    min,
    max,
    prefix,
    suffix,
    size = "medium",
    style = ["simple", "border"],
    onClick,
    onBeforeChange,
    onChange
}) => {
    const [valueState, setValue] = useState(value);
    const classes = () => ["button-increment", size, ...style].join(" ");
    const _onClick = () => {
        if (onClick) onClick();
    };
    const _onMinusClick = () => {
        const valueNew = valueState - delta;
        if (onBeforeChange) {
            if (!onBeforeChange(valueNew)) return;
        }
        if (min !== undefined && valueNew < min) return;
        setValue(valueNew);
        if (onChange) onChange(valueNew);
    };
    const _onPlusClick = () => {
        const valueNew = valueState + delta;
        if (onBeforeChange) {
            if (!onBeforeChange(valueNew)) return;
        }
        if (max !== undefined && valueNew > max) return;
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
            {prefix && <span className="prefix">{prefix}</span>}
            <span className="value">{valueState}</span>
            {suffix && <span className="suffix">{suffix}</span>}
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
