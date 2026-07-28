import {
    GameBoyCore,
    GbaCore,
    releaseCore,
    releaseGbaCore,
    StorageAdapter
} from "boytacean-core";
import React, { FC, ReactNode, useEffect, useMemo, useState } from "react";

import {
    BoytaceanContext,
    BoytaceanContextValue,
    BoytaceanSystem
} from "./boytacean-context";

/**
 * Resolves the system to be emulated from the requested one and the
 * ROM that is going to be loaded, inferring it from the extension of
 * the ROM whenever the automatic mode is used.
 *
 * @param system The system that has been requested by the caller.
 * @param rom The URL of the ROM that is going to be loaded.
 * @returns The concrete system to be emulated.
 */
export const resolveSystem = (
    system: BoytaceanSystem,
    rom?: string
): BoytaceanSystem.GameBoy | BoytaceanSystem.GameBoyAdvance => {
    if (system !== BoytaceanSystem.Auto) return system;
    const path = rom?.split("?")[0].toLowerCase() ?? "";
    return path.endsWith(".gba")
        ? BoytaceanSystem.GameBoyAdvance
        : BoytaceanSystem.GameBoy;
};

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
     * The name of the palette to be set at startup, only used by the
     * Game Boy system as the GBA has no palette support.
     */
    palette?: string;

    /**
     * The system to be emulated, defaults to the automatic mode in
     * which it's inferred from the extension of the ROM.
     */
    system?: BoytaceanSystem;

    children?: ReactNode;
};

/**
 * Provides an emulator core to the components under it, taking care
 * of the complete lifecycle of the emulator, meaning that the core
 * is booted on mount and stopped on unmount.
 *
 * The core that is built depends on the system that is being
 * emulated, meaning that a new one is created whenever the system
 * changes, either explicitly or through the ROM that is loaded.
 */
export const BoytaceanProvider: FC<BoytaceanProviderProps> = ({
    rom,
    wasmPath,
    storage,
    palette,
    system = BoytaceanSystem.Auto,
    children
}) => {
    const resolved = resolveSystem(system, rom);
    const [core, setCore] = useState<GameBoyCore | GbaCore>(() =>
        resolved === BoytaceanSystem.GameBoyAdvance
            ? new GbaCore({ wasmPath: wasmPath, storage: storage })
            : new GameBoyCore({ wasmPath: wasmPath, storage: storage })
    );

    // replaces the core whenever the resolved system no longer
    // matches the one of the current core, so that a ROM of another
    // system can be loaded into the same component
    useEffect(() => {
        const isGba = core instanceof GbaCore;
        if (isGba === (resolved === BoytaceanSystem.GameBoyAdvance)) return;
        setCore(
            resolved === BoytaceanSystem.GameBoyAdvance
                ? new GbaCore({ wasmPath: wasmPath, storage: storage })
                : new GameBoyCore({ wasmPath: wasmPath, storage: storage })
        );
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [core, resolved]);

    useEffect(() => {
        let disposed = false;

        const boot = async () => {
            await core.init();

            // in case the component has been unmounted while the WASM
            // module was still loading then avoids the boot operation
            if (disposed) return;

            // the palette is only supported by the Game Boy system,
            // as the GBA drives the colors from the ROM itself
            if (palette && core instanceof GameBoyCore) {
                core.palette = palette;
            }

            // starts the main loop of the emulator, notice that this
            // promise is only settled once the emulator is stopped
            await core.start({ romUrl: rom });
        };
        boot();

        return () => {
            disposed = true;
            core.stop();
            if (core instanceof GbaCore) releaseGbaCore(core);
            else releaseCore(core);
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [core, rom]);

    const value = useMemo<BoytaceanContextValue>(
        () => ({
            core: core,
            system: resolved,
            play: () => core.resume(),
            pause: () => core.pause(),
            reset: () => core.reset(),
            loadRom: (name: string, data: Uint8Array) =>
                core.boot({ romName: name, romData: data, reuse: false }),
            press: (key: string) => core.keyPress(key),
            release: (key: string) => core.keyLift(key)
        }),
        [core, resolved]
    );

    return (
        <BoytaceanContext.Provider value={value}>
            {children}
        </BoytaceanContext.Provider>
    );
};
