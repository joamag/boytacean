import {
    GameBoyCore,
    GbaCore,
    releaseCore,
    releaseGbaCore,
    StorageAdapter
} from "boytacean-core";
import React, {
    FC,
    ReactNode,
    useEffect,
    useMemo,
    useRef,
    useState
} from "react";

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

type RomRequest = {
    source?: string;
    name: string;
    data: Uint8Array;
    resolve: () => void;
    reject: (error: Error) => void;
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
    const [request, setRequest] = useState<RomRequest | null>(null);
    const pending = useRef<RomRequest | null>(null);
    const currentRequest = request?.source === rom ? request : null;
    const resolved = resolveSystem(system, currentRequest?.name ?? rom);
    const core = useMemo<GameBoyCore | GbaCore>(
        () =>
            resolved === BoytaceanSystem.GameBoyAdvance
                ? new GbaCore({ wasmPath: wasmPath, storage: storage })
                : new GameBoyCore({ wasmPath: wasmPath, storage: storage }),
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [resolved, rom, currentRequest]
    );

    useEffect(() => {
        return () => {
            pending.current?.reject(new Error("ROM loading was cancelled"));
            pending.current = null;
        };
    }, []);

    useEffect(() => {
        let disposed = false;
        if (request && !currentRequest) setRequest(null);

        const onBooted = () => {
            if (disposed) return;
            currentRequest?.resolve();
            if (pending.current === currentRequest) pending.current = null;
        };

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

            if (currentRequest) {
                core.setRom(currentRequest.name, currentRequest.data, null);
                core.bind("booted", onBooted);
            } else if (!rom) {
                return;
            }

            // starts the main loop of the emulator, notice that this
            // promise is only settled once the emulator is stopped
            await core.start({ romUrl: currentRequest ? undefined : rom });
        };
        boot().catch((error: Error) => {
            if (disposed) return;
            if (currentRequest) currentRequest.reject(error);
            else console.error(error);
            if (pending.current === currentRequest) pending.current = null;
        });

        return () => {
            disposed = true;
            core.stop();
            core.unbind("booted", onBooted);
            if (core instanceof GbaCore) releaseGbaCore(core);
            else releaseCore(core);
            if (pending.current === currentRequest) {
                currentRequest?.reject(new Error("ROM loading was cancelled"));
                pending.current = null;
            }
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [core]);

    const value = useMemo<BoytaceanContextValue>(
        () => ({
            core: core,
            system: resolved,
            play: () => core.resume(),
            pause: () => core.pause(),
            reset: () => core.reset(),
            loadRom: (name: string, data: Uint8Array) =>
                new Promise<void>((resolve, reject) => {
                    pending.current?.reject(
                        new Error("ROM loading was cancelled")
                    );
                    const request = {
                        source: rom,
                        name,
                        data,
                        resolve,
                        reject
                    };
                    pending.current = request;
                    setRequest(request);
                }),
            press: (key: string) => core.keyPress(key),
            release: (key: string) => core.keyLift(key)
        }),
        [core, resolved, rom]
    );

    return (
        <BoytaceanContext.Provider value={value}>
            {children}
        </BoytaceanContext.Provider>
    );
};
