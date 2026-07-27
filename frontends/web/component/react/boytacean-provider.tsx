import { GameBoyCore, StorageAdapter } from "boytacean-core";
import React, { FC, ReactNode, useEffect, useMemo, useState } from "react";

import { BoytaceanContext, BoytaceanContextValue } from "./boytacean-context";

type BoytaceanProviderProps = {
    /**
     * The URL of the ROM to be loaded at boot time, in case it's
     * not provided no ROM is loaded and the machine stays idle.
     */
    rom?: string;

    /**
     * The path from which the WASM binary is going to be loaded,
     * defaults to the wasm-bindgen resolution strategy.
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

    children?: ReactNode;
};

/**
 * Provides a Game Boy core to the components under it, taking care
 * of the complete lifecycle of the emulator, meaning that the core
 * is booted on mount and stopped on unmount.
 */
export const BoytaceanProvider: FC<BoytaceanProviderProps> = ({
    rom,
    wasmPath,
    storage,
    palette,
    children
}) => {
    const [core] = useState(
        () => new GameBoyCore({ wasmPath: wasmPath, storage: storage })
    );

    useEffect(() => {
        let disposed = false;

        const boot = async () => {
            await core.init();

            // in case the component has been unmounted while the WASM
            // module was still loading then avoids the boot operation
            if (disposed) return;
            if (palette) core.palette = palette;

            // starts the main loop of the emulator, notice that this
            // promise is only settled once the emulator is stopped
            await core.start({ romUrl: rom });
        };
        boot();

        return () => {
            disposed = true;
            core.stop();
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [core, rom]);

    const value = useMemo<BoytaceanContextValue>(
        () => ({
            core: core,
            play: () => core.resume(),
            pause: () => core.pause(),
            reset: () => core.reset(),
            loadRom: (name: string, data: Uint8Array) =>
                core.boot({ romName: name, romData: data, reuse: false }),
            press: (key: string) => core.keyPress(key),
            release: (key: string) => core.keyLift(key)
        }),
        [core]
    );

    return (
        <BoytaceanContext.Provider value={value}>
            {children}
        </BoytaceanContext.Provider>
    );
};
