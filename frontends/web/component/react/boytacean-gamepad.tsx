import React, { CSSProperties, FC } from "react";

import { useBoytacean } from "./boytacean-context";

import "./boytacean-gamepad.css";

type BoytaceanGamepadProps = {
    className?: string;
    style?: CSSProperties;
};

type GamepadButtonProps = {
    /**
     * The name of the game pad key that is going to be pressed
     * while the button is held down.
     */
    button: string;

    label: string;
    className: string;
};

const GamepadButton: FC<GamepadButtonProps> = ({
    button,
    label,
    className
}) => {
    const { press, release } = useBoytacean();
    return (
        <button
            type="button"
            className={className}
            onMouseDown={() => press(button)}
            onMouseUp={() => release(button)}
            onMouseLeave={() => release(button)}
            onTouchStart={(event) => {
                event.preventDefault();
                press(button);
            }}
            onTouchEnd={(event) => {
                event.preventDefault();
                release(button);
            }}
        >
            {label}
        </button>
    );
};

/**
 * On screen game pad that drives the emulated keys, meant to be used
 * as the out of the box input method for touch devices.
 */
export const BoytaceanGamepad: FC<BoytaceanGamepadProps> = ({
    className,
    style
}) => (
    <div className={["gamepad", className].join(" ")} style={style}>
        <div className="gamepad-dpad">
            <GamepadButton
                button="ArrowUp"
                label="▲"
                className="gamepad-button up"
            />
            <GamepadButton
                button="ArrowLeft"
                label="◀"
                className="gamepad-button left"
            />
            <GamepadButton
                button="ArrowRight"
                label="▶"
                className="gamepad-button right"
            />
            <GamepadButton
                button="ArrowDown"
                label="▼"
                className="gamepad-button down"
            />
        </div>
        <div className="gamepad-actions">
            <GamepadButton
                button="B"
                label="B"
                className="gamepad-button action"
            />
            <GamepadButton
                button="A"
                label="A"
                className="gamepad-button action"
            />
        </div>
        <div className="gamepad-menu">
            <GamepadButton
                button="Select"
                label="SELECT"
                className="gamepad-button menu"
            />
            <GamepadButton
                button="Start"
                label="START"
                className="gamepad-button menu"
            />
        </div>
    </div>
);
