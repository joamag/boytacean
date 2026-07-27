import React, { CSSProperties, FC, ReactNode } from "react";
import { StorageAdapter } from "boytacean-core";

import { BoytaceanGamepad } from "./boytacean-gamepad";
import { BoytaceanKeyboard } from "./boytacean-keyboard";
import { BoytaceanProvider } from "./boytacean-provider";
import { BoytaceanScreen } from "./boytacean-screen";

type BoytaceanProps = {
    /**
     * The URL of the ROM to be loaded at boot time.
     */
    rom?: string;

    /**
     * The path from which the WASM binary is going to be loaded.
     */
    wasmPath?: string;

    /**
     * The storage adapter to be used in the persistence of both the
     * battery backed RAM and the settings of the emulator.
     */
    storage?: StorageAdapter;

    /**
     * The name of the palette to be set at startup.
     */
    palette?: string;

    /**
     * The scale factor to be applied to the display, ignored in case
     * custom children are provided.
     */
    scale?: number;

    /**
     * If the physical keyboard should be bound to the emulated
     * game pad, ignored in case custom children are provided.
     */
    keyboard?: boolean;

    /**
     * If the on screen game pad should be shown, ignored in case
     * custom children are provided.
     */
    gamepad?: boolean;

    className?: string;
    style?: CSSProperties;

    /**
     * The custom layout to be rendered under the provider, in case
     * it's provided none of the default components are rendered and
     * the complete presentation is delegated to the caller.
     */
    children?: ReactNode;
};

/**
 * Drop-in Game Boy emulator, rendering the display, the on screen
 * game pad and binding the physical keyboard out of the box.
 *
 * Whenever children are provided the default presentation is skipped
 * and only the provider is rendered, allowing the caller to build its
 * own layout using the `useBoytacean()` hook.
 */
export const Boytacean: FC<BoytaceanProps> & {
    Provider: typeof BoytaceanProvider;
    Screen: typeof BoytaceanScreen;
    Gamepad: typeof BoytaceanGamepad;
    Keyboard: typeof BoytaceanKeyboard;
} = ({
    rom,
    wasmPath,
    storage,
    palette,
    scale = 3,
    keyboard = true,
    gamepad = true,
    className,
    style,
    children
}) => (
    <BoytaceanProvider
        rom={rom}
        wasmPath={wasmPath}
        storage={storage}
        palette={palette}
    >
        {children ?? (
            <div className={className} style={style}>
                <BoytaceanScreen scale={scale} />
                {keyboard && <BoytaceanKeyboard />}
                {gamepad && <BoytaceanGamepad />}
            </div>
        )}
    </BoytaceanProvider>
);

Boytacean.Provider = BoytaceanProvider;
Boytacean.Screen = BoytaceanScreen;
Boytacean.Gamepad = BoytaceanGamepad;
Boytacean.Keyboard = BoytaceanKeyboard;
