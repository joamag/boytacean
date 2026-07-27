import { useCallback, useRef, useSyncExternalStore } from "react";

import { useBoytacean } from "./boytacean-context";

/**
 * The rate at which the emulation statistics are sampled, in
 * milliseconds, keeping the re-render frequency independent from
 * the frame rate of the emulator.
 */
const STATS_INTERVAL = 500;

/**
 * The status of the emulator, meaning the set of values that change
 * rarely enough to be safely kept in the React state.
 */
export type BoytaceanStatus = {
    /**
     * If a ROM has already been booted in the emulator.
     */
    booted: boolean;

    /**
     * The name of the ROM that is currently loaded.
     */
    romName?: string;
};

/**
 * The emulation statistics, sampled at a fixed interval instead of
 * being updated once per frame.
 */
export type BoytaceanStats = {
    framerate: number;
    cyclerate: number;
    emulationSpeed: number;
};

/**
 * Subscribes to the status of the emulator, re-rendering the caller
 * only when a ROM is booted and not on every frame.
 *
 * @returns The current status of the emulator.
 */
export const useBoytaceanStatus = (): BoytaceanStatus => {
    const { core } = useBoytacean();

    // the snapshot is cached and only replaced when one of its
    // values actually changes, as `useSyncExternalStore` requires a
    // stable reference to avoid an infinite render loop
    const snapshot = useRef<BoytaceanStatus>({ booted: false });

    const subscribe = useCallback(
        (onChange: () => void) => {
            core.bind("booted", onChange);
            return () => core.unbind("booted", onChange);
        },
        [core]
    );

    const getSnapshot = useCallback(() => {
        const romName = core.loadedRomName ?? undefined;
        const booted = core.loaded;
        const current = snapshot.current;
        if (current.booted !== booted || current.romName !== romName) {
            snapshot.current = { booted: booted, romName: romName };
        }
        return snapshot.current;
    }, [core]);

    return useSyncExternalStore(subscribe, getSnapshot, () => snapshot.current);
};

/**
 * Subscribes to the emulation statistics, sampling them at a fixed
 * interval so that the caller is not re-rendered once per frame.
 *
 * @param interval The sampling interval in milliseconds.
 * @returns The current emulation statistics.
 */
export const useBoytaceanStats = (
    interval = STATS_INTERVAL
): BoytaceanStats => {
    const { core } = useBoytacean();

    const snapshot = useRef<BoytaceanStats>({
        framerate: 0,
        cyclerate: 0,
        emulationSpeed: 0
    });

    const subscribe = useCallback(
        (onChange: () => void) => {
            const handle = setInterval(onChange, interval);
            return () => clearInterval(handle);
        },
        [interval]
    );

    const getSnapshot = useCallback(() => {
        const current = snapshot.current;
        const framerate = core.framerate;
        const cyclerate = core.cyclerate;
        const emulationSpeed = core.emulationSpeed;
        if (
            current.framerate !== framerate ||
            current.cyclerate !== cyclerate ||
            current.emulationSpeed !== emulationSpeed
        ) {
            snapshot.current = {
                framerate: framerate,
                cyclerate: cyclerate,
                emulationSpeed: emulationSpeed
            };
        }
        return snapshot.current;
    }, [core]);

    return useSyncExternalStore(subscribe, getSnapshot, () => snapshot.current);
};
