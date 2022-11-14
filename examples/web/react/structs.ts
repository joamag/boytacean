export const FREQUENCY_DELTA = 100000;

export type Callback<T> = (owner: T, params?: any) => void;

/**
 * Abstract class that implements the basic functionality
 * part of the definition of the Observer pattern.
 *
 * @see {@link https://en.wikipedia.org/wiki/Observer_pattern}
 */
export class Observable {
    private events: Record<string, [Callback<this>]> = {};

    bind(event: string, callback: Callback<this>) {
        const callbacks = this.events[event] ?? [];
        if (callbacks.includes(callback)) return;
        callbacks.push(callback);
        this.events[event] = callbacks;
    }

    unbind(event: string, callback: Callback<this>) {
        const callbacks = this.events[event] ?? [];
        if (!callbacks.includes(callback)) return;
        const index = callbacks.indexOf(callback);
        callbacks.splice(index, 1);
        this.events[event] = callbacks;
    }

    trigger(event: string, params?: any) {
        const callbacks = this.events[event] ?? [];
        callbacks.forEach((c) => c(this, params));
    }
}

export type RomInfo = {
    name?: string;
    data?: Uint8Array;
    size?: number;
    extra?: Record<string, string | undefined>;
};

export type BenchmarkResult = {
    delta: number;
    count: number;
    cycles: number;
    frequency_mhz: number;
};

export enum Feature {
    Debug = 1,
    Palettes,
    Benchmark,
    Keyboard,
    KeyboardChip8,
    KeyboardGB
}

/**
 * Enumeration that describes the multiple pixel
 * formats and the associated size in bytes.
 */
export enum PixelFormat {
    RGB = 3,
    RGBA = 4
}

export interface ObservableI {
    bind(event: string, callback: Callback<this>): void;
    unbind(event: string, callback: Callback<this>): void;
    trigger(event: string): void;
}

/**
 * Top level interface that declares the main abstract
 * interface of an emulator structured entity.
 * Should allow typical hardware operations to be performed.
 */
export interface Emulator extends ObservableI {
    /**
     * The descriptive name of the emulator.
     */
    get name(): string;

    /**
     * The name of the the hardware that is being emulated
     * by the emulator (eg: Super Nintendo).
     */
    get device(): string;

    /**
     * A URL to a website that describes the device that is
     * being emulated by the emulator (eg: Wikipedia link).
     */
    get deviceUrl(): string | undefined;

    /**
     * A semantic version string for the current version
     * of the emulator.
     *
     * @see {@link https://semver.org}
     */
    get version(): string;

    /**
     * The URL to the page describing the current version
     * of the emulator.
     */
    get versionUrl(): string | undefined;

    get repository(): string | undefined;

    get repositoryUrl(): string | undefined;

    /**
     * The features available and compatible with the emulator,
     * these values will influence the associated GUIs.
     */
    get features(): Feature[];

    /**
     * The complete set of engine names that can be used
     * in the re-boot operation.
     */
    get engines(): string[];

    /**
     * The name of the current execution engine being used
     * by the emulator.
     */
    get engine(): string | null;

    /**
     * The complete set of file extensions that this emulator
     * supports.
     */
    get romExts(): string[];

    /**
     * The pixel format of the emulator's display
     * image buffer (eg: RGB).
     */
    get pixelFormat(): PixelFormat;

    /**
     * Gets the complete image buffer as a sequence of
     * bytes that respects the current pixel format from
     * `getPixelFormat()`. This method returns an in memory
     * pointer to the heap and not a copy.
     */
    get imageBuffer(): Uint8Array;

    /**
     * Gets information about the ROM that is currently
     * loaded in the emulator, using a structure containing
     * the information about the ROM that is currently
     * loaded in the emulator.
     */
    get romInfo(): RomInfo;

    /**
     * The current CPU frequency (logic) of the emulator,
     * should impact other elements of the emulator.
     */
    get frequency(): number;
    set frequency(value: number);

    /**
     * The recommended frequency delta in hertz for scale up
     * and scale down operations in the CPU frequency.
     */
    get frequencyDelta(): number | null;

    /**
     * The current logic framerate of the running emulator.
     */
    get framerate(): number;

    /**
     * A dictionary that contains the register names associated
     * with their value either as strings or numbers.
     */
    get registers(): Record<string, string | number>;

    /**
     * Obtains the pixel buffer for the VRAM tile at the given
     * index.
     *
     * @param index The index of the tile to obtain pixel buffer.
     * @returns The pixel buffer of the tile at the given index.
     */
    getTile(index: number): Uint8Array;

    /**
     * Boot (or reboots) the emulator according to the provided
     * set of options.
     *
     * @param options The options that are going to be used for
     * the booting operation of the emulator.
     */
    boot(options: any): void;

    /**
     * Toggle the running state of the emulator between paused
     * and running, prevents consumers from the need to access
     * the current running state of the emulator to implement
     * a logic toggle.
     */
    toggleRunning(): void;
    pause(): void;
    resume(): void;

    /**
     * Resets the emulator machine to the start state and
     * re-loads the ROM that is currently set in the emulator.
     */
    reset(): void;

    keyPress(key: string): void;

    keyLift(key: string): void;

    /**
     * Changes the palette of the emulator to the "next" one.
     */
    changePalette?: { (): void };

    /**
     * Runs a benchmark operation in the emulator, effectively
     * measuring the performance of it.
     *
     * @param count The number of benchmark iterations to be
     * run, increasing this value will make the benchmark take
     * more time to be executed.
     * @returns The result metrics from the benchmark run.
     */
    benchmark?: { (count?: number): BenchmarkResult };
}

export class EmulatorBase extends Observable {
    get deviceUrl(): string | undefined {
        return undefined;
    }

    get versionUrl(): string | undefined {
        return undefined;
    }

    get repository(): string | undefined {
        return undefined;
    }

    get repositoryUrl(): string | undefined {
        return undefined;
    }

    get features(): Feature[] {
        return [];
    }

    get frequencyDelta(): number | null {
        return FREQUENCY_DELTA;
    }
}
