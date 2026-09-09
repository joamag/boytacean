import { GameBoyCore, GbaCore } from "boytacean-core";
import { createContext, useContext } from "react";

/**
 * Enumeration with the complete set of systems that can be emulated
 * by the component, used to select the core to be built.
 */
export enum BoytaceanSystem {
    Auto = "auto",
    GameBoy = "gb",
    GameBoyAdvance = "gba"
}

/**
 * The stable set of values exposed by the Boytacean context, the
 * identity of this structure is kept stable for the complete
 * lifetime of the provider, meaning that consumers of it are never
 * re-rendered because of emulation activity.
 *
 * High frequency information (eg: the frame buffer) is intentionally
 * not part of this structure and should be consumed by binding
 * directly to the events of the underlying core.
 */
export type BoytaceanContextValue = {
    /**
     * The core instance that is driving the emulation, provides
     * access to the complete set of low level operations.
     */
    core: GameBoyCore | GbaCore;

    /**
     * The system that is currently being emulated by the core, as
     * resolved from the `system` property and the ROM.
     */
    system: BoytaceanSystem.GameBoy | BoytaceanSystem.GameBoyAdvance;

    /**
     * Resumes the emulation in case it's currently paused.
     */
    play: () => Promise<void>;

    /**
     * Pauses the emulation, keeping the current machine state.
     */
    pause: () => Promise<void>;

    /**
     * Resets the machine, restarting the currently loaded ROM.
     */
    reset: () => Promise<void>;

    /**
     * Loads the ROM with the provided data into the machine,
     * booting it in the process.
     *
     * @param name The name of the ROM to be loaded.
     * @param data The binary contents of the ROM to be loaded.
     */
    loadRom: (name: string, data: Uint8Array) => Promise<void>;

    /**
     * Presses the provided key in the emulated game pad.
     *
     * @param key The name of the key to be pressed.
     */
    press: (key: string) => void;

    /**
     * Releases the provided key in the emulated game pad.
     *
     * @param key The name of the key to be released.
     */
    release: (key: string) => void;
};

export const BoytaceanContext = createContext<BoytaceanContextValue | null>(
    null
);

/**
 * Obtains the Boytacean context of the closest provider, allowing
 * the control of the emulator from any component under it.
 *
 * @returns The context value of the closest Boytacean provider.
 */
export const useBoytacean = (): BoytaceanContextValue => {
    const context = useContext(BoytaceanContext);
    if (!context) {
        throw new Error("useBoytacean() requires a <Boytacean.Provider>");
    }
    return context;
};
