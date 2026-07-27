/**
 * Abstract interface for the persistence of opaque values, used
 * by the core to store both the battery backed RAM and the
 * settings of the emulator.
 *
 * Allows the embedder to provide its own persistence strategy
 * instead of being bound to the Web Storage API.
 */
export interface StorageAdapter {
    /**
     * Obtains the value currently stored under the provided key.
     *
     * @param key The key of the value to be retrieved.
     * @returns The value stored under the key or null in case no
     * value is currently stored for it.
     */
    getItem(key: string): string | null;

    /**
     * Stores the provided value under the provided key, replacing
     * any value that may have been previously stored.
     *
     * @param key The key under which the value is stored.
     * @param value The value to be stored.
     */
    setItem(key: string, value: string): void;
}

/**
 * Storage adapter backed by the Web Storage API, using the
 * `localStorage` of the current window as the persistence layer.
 *
 * This is the default adapter used whenever the core runs in a
 * browser environment that exposes the `localStorage` object.
 */
export class LocalStorageAdapter implements StorageAdapter {
    getItem(key: string): string | null {
        return localStorage.getItem(key);
    }

    setItem(key: string, value: string) {
        localStorage.setItem(key, value);
    }
}

/**
 * Storage adapter that discards every write operation and never
 * returns a stored value, to be used whenever persistence is
 * either not available or not wanted.
 */
export class NullStorageAdapter implements StorageAdapter {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    getItem(key: string): string | null {
        return null;
    }

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    setItem(key: string, value: string) {}
}

/**
 * Builds the default storage adapter for the current environment,
 * using the Web Storage API whenever it's available and falling
 * back to a null (no-op) adapter otherwise.
 *
 * @returns The storage adapter to be used by default.
 */
export const defaultStorage = (): StorageAdapter =>
    typeof localStorage === "undefined"
        ? new NullStorageAdapter()
        : new LocalStorageAdapter();
