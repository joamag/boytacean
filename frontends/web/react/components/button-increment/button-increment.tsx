import React, { FC, useEffect, useState } from "react";
import Button from "../button/button";

import "./button-increment.css";

type ButtonIncrementProps = {
    value: number;
    delta?: number;
    min?: number;
    max?: number;
    prefix?: string;
    suffix?: string;
    decimalPlaces?: number;
    size?: string;
    style?: string[];
    onClick?: () => void;
    onBeforeChange?: (value: number) => boolean;
    onChange?: (value: number) => void;
    onReady?: (setValue: (value: number) => void) => void;
};

export const ButtonIncrement: FC<ButtonIncrementProps> = ({
    value,
    delta = 1,
    min,
    max,
    prefix,
    suffix,
    decimalPlaces,
    size = "medium",
    style = ["simple", "border"],
    onClick,
    onBeforeChange,
    onChange,
    onReady
}) => {
    const [valueState, setValue] = useState(value);
    const classes = () => ["button-increment", size, ...style].join(" ");
    useEffect(() => {
        onReady && onReady((value) => setValue(value));
    }, []);
    const _onMinusClick = () => {
        let valueNew = valueState - delta;
        if (onBeforeChange) {
            if (!onBeforeChange(valueNew)) return;
        }
        if (min !== undefined) valueNew = Math.max(min, valueNew);
        if (valueNew === valueState) return;
        setValue(valueNew);
        if (onChange) onChange(valueNew);
    };
    const _onPlusClick = () => {
        let valueNew = valueState + delta;
        if (onBeforeChange) {
            if (!onBeforeChange(valueNew)) return;
        }
        if (max !== undefined) valueNew = Math.min(max, valueNew);
        if (valueNew === valueState) return;
        setValue(valueNew);
        if (onChange) onChange(valueNew);
    };
    return (
        <span className={classes()} onClick={onClick}>
            <Button
                text={"-"}
                size={size}
                style={["simple"]}
                onClick={_onMinusClick}
            />
            {prefix && <span className="prefix">{prefix}</span>}
            <span className="value">
                {decimalPlaces ? valueState.toFixed(decimalPlaces) : valueState}
            </span>
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
