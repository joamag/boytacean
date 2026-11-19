import { Dump, Dumper } from "boytacean";

/**
 * The baud rate of the serial connection to a GBxCart RW.
 */
export const CARTRIDGE_BAUD_RATE = 1000000;

/**
 * The time (in milliseconds) to wait for a reply of the device,
 * which never comes when it runs another firmware.
 */
export const CARTRIDGE_TIMEOUT = 2000;

/**
 * The USB IDs of the serial converter of a GBxCart RW, used so
 * that only these devices are offered to the user.
 */
export const CARTRIDGE_FILTERS = [
    { usbVendorId: 0x1a86, usbProductId: 0x7523 }
];

/**
 * The subset of a Web Serial API port that is used to read a
 * cartridge, so that any compatible port can be used.
 */
export type CartridgePort = {
    readable: ReadableStream<Uint8Array> | null;
    writable: WritableStream<Uint8Array> | null;
    open(options: { baudRate: number }): Promise<void>;
    close(): Promise<void>;
};

/**
 * The subset of an exchange of the dumper (from the WASM package)
 * that is used to talk to the device.
 */
export type CartridgeExchange = {
    send_eager(): Uint8Array;
    expect(): number;
    free(): void;
};

/**
 * The subset of the dumper (from the WASM package) that is used to
 * read a cartridge, which holds the complete dumping logic.
 */
export type CartridgeDumper = {
    next_exchange(): CartridgeExchange | undefined;
    feed_wa(data: Uint8Array): void;
    progress(): number;
    device(): string | undefined;
    finish_wa(): Dump;
    free(): void;
};

/**
 * The result of reading a cartridge, the dump and the description
 * of the device that read it.
 */
export type CartridgeRead = {
    dump: Dump;
    device: string | undefined;
};

/**
 * Enumeration with the values for the save that is used for a game
 * loaded from a cartridge.
 */
export enum CartridgeSave {
    Cartridge = "cartridge",
    Storage = "storage",
    None = "none"
}

/**
 * The information on a cartridge that has been loaded into the
 * emulator, including the save that is used for its game.
 */
export type CartridgeInfo = {
    title: string;
    device: string | undefined;
    checksumValid: boolean;
    save: CartridgeSave;
};

/**
 * The bytes received from the device that have not been consumed
 * yet, as replies can be split or merged by the serial port.
 */
type SerialBuffer = {
    data: Uint8Array;
};

/**
 * Checks if the current environment supports the Web Serial API,
 * required to read cartridges.
 *
 * @returns If cartridges can be read in the current environment.
 */
export const hasCartridgeSupport = (): boolean => {
    return typeof navigator !== "undefined" && "serial" in navigator;
};

/**
 * Asks the user to select the serial port of a GBxCart RW, which
 * must be called as the result of a user gesture.
 *
 * @returns The serial port selected by the user.
 */
export const requestCartridgePort = async (): Promise<CartridgePort> => {
    const serial = (
        navigator as unknown as {
            serial: {
                requestPort(options: {
                    filters: typeof CARTRIDGE_FILTERS;
                }): Promise<CartridgePort>;
            };
        }
    ).serial;
    return await serial.requestPort({ filters: CARTRIDGE_FILTERS });
};

/**
 * Reads the cartridge in the GBxCart RW at the provided serial port,
 * the port is opened for the read and always closed afterwards.
 *
 * @param port The serial port of the GBxCart RW.
 * @param options The dumper to be used (defaults to the one in the
 * WASM package), which is released after the read, a callback for the progress (from 0 to 1) and the
 * timeout for each reply of the device.
 * @returns The dump of the cartridge and the device that read it.
 */
export const readCartridge = async (
    port: CartridgePort,
    {
        dumper = undefined,
        onProgress = undefined,
        timeout = CARTRIDGE_TIMEOUT
    }: {
        dumper?: CartridgeDumper;
        onProgress?: (progress: number) => void;
        timeout?: number;
    } = {}
): Promise<CartridgeRead> => {
    let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
    let writer: WritableStreamDefaultWriter<Uint8Array> | undefined;
    let active: CartridgeDumper | undefined = dumper;
    try {
        await port.open({ baudRate: CARTRIDGE_BAUD_RATE });
        reader = port.readable?.getReader();
        writer = port.writable?.getWriter();
        if (!reader || !writer) {
            throw new Error("Unable to access the serial port of the device");
        }

        // the dumper (always owned and released here) is only built
        // once the port is open, so that nothing is built in vain
        active = active ?? new Dumper();
        const buffer: SerialBuffer = { data: new Uint8Array(0) };
        let exchange: CartridgeExchange | undefined;
        while ((exchange = active.next_exchange())) {
            try {
                await writer.write(exchange.send_eager());
                const data = await readExact(
                    reader,
                    buffer,
                    exchange.expect(),
                    timeout
                );
                active.feed_wa(data);
            } finally {
                exchange.free();
            }
            onProgress?.(active.progress());
        }

        // waits for every byte (eg: the final release of the cartridge
        // bus) to reach the device, as closing the port discards the
        // bytes that are still buffered
        await writer.close();
        return { dump: active.finish_wa(), device: active.device() };
    } finally {
        // cancels any pending read before releasing the stream locks,
        // so that the port can always be closed
        await reader?.cancel().catch(() => undefined);
        reader?.releaseLock();
        writer?.releaseLock();
        await port.close().catch(() => undefined);
        active?.free();
    }
};

/**
 * Reads exactly the provided number of bytes from the device, keeping
 * any extra bytes received for the next read.
 *
 * @param reader The reader of the serial port.
 * @param buffer The bytes received and not consumed yet.
 * @param count The number of bytes to be read.
 * @param timeout The time (in milliseconds) to wait for each chunk.
 * @returns The bytes read from the device.
 */
const readExact = async (
    reader: ReadableStreamDefaultReader<Uint8Array>,
    buffer: SerialBuffer,
    count: number,
    timeout: number
): Promise<Uint8Array> => {
    while (buffer.data.length < count) {
        const { value, done } = await readTimeout(reader, timeout);
        if (done || !value) {
            throw new Error("The connection to the device was closed");
        }
        const data = new Uint8Array(buffer.data.length + value.length);
        data.set(buffer.data);
        data.set(value, buffer.data.length);
        buffer.data = data;
    }
    const data = buffer.data.slice(0, count);
    buffer.data = buffer.data.slice(count);
    return data;
};

/**
 * Reads the next chunk from the device, failing in case it does not
 * reply within the provided timeout.
 *
 * @param reader The reader of the serial port.
 * @param timeout The time (in milliseconds) to wait for the chunk.
 * @returns The result of the read operation.
 */
const readTimeout = async (
    reader: ReadableStreamDefaultReader<Uint8Array>,
    timeout: number
): Promise<ReadableStreamReadResult<Uint8Array>> => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([
            reader.read(),
            new Promise<never>((_, reject) => {
                timer = setTimeout(
                    () =>
                        reject(
                            new Error(
                                "The device is not responding, check that it runs the L firmware"
                            )
                        ),
                    timeout
                );
            })
        ]);
    } finally {
        clearTimeout(timer);
    }
};
