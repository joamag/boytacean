import { FC, useEffect } from "react";

import { useBoytacean } from "./boytacean-context";

/**
 * The default mapping between the keys of the physical keyboard and
 * the keys of the emulated game pad.
 */
export const KEYS_MAP: Record<string, string> = {
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    Enter: "Start",
    " ": "Select",
    a: "A",
    s: "B",
    A: "A",
    S: "B"
};

type BoytaceanKeyboardProps = {
    /**
     * The mapping between the keys of the physical keyboard and the
     * keys of the emulated game pad.
     */
    keys?: Record<string, string>;

    /**
     * If the listeners should be registered against the document
     * instead of the window, both are global in nature.
     */
    target?: EventTarget;
};

/**
 * Binds the physical keyboard to the emulated game pad.
 *
 * This component does not render anything, allowing the keyboard
 * support to be added to any custom layout.
 */
export const BoytaceanKeyboard: FC<BoytaceanKeyboardProps> = ({
    keys = KEYS_MAP,
    target
}) => {
    const { press, release } = useBoytacean();

    useEffect(() => {
        const element = target ?? window;

        const onKeyDown = (event: Event) => {
            const key = keys[(event as KeyboardEvent).key];
            if (!key) return;
            press(key);
            event.preventDefault();
        };
        const onKeyUp = (event: Event) => {
            const key = keys[(event as KeyboardEvent).key];
            if (!key) return;
            release(key);
            event.preventDefault();
        };

        // releases every mapped key whenever the page loses focus, as
        // the key up event is not delivered in that situation and the
        // key would otherwise stay pressed forever
        const onBlur = () => {
            Object.values(keys).forEach((key) => release(key));
        };

        element.addEventListener("keydown", onKeyDown);
        element.addEventListener("keyup", onKeyUp);
        window.addEventListener("blur", onBlur);

        return () => {
            element.removeEventListener("keydown", onKeyDown);
            element.removeEventListener("keyup", onKeyUp);
            window.removeEventListener("blur", onBlur);
        };
    }, [keys, target, press, release]);

    return null;
};
