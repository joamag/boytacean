import assert from "assert";

import { Dump } from "boytacean";

import {
    CARTRIDGE_BAUD_RATE,
    CARTRIDGE_FILTERS,
    CartridgeDumper,
    CartridgeExchange,
    CartridgePort,
    hasCartridgeSupport,
    readCartridge,
    requestCartridgePort
} from "../../ts/cartridge";

/**
 * A step of the scripted conversation with the fake device, the bytes
 * the dumper sends and the chunks the device replies with.
 */
type Step = {
    send: number[];
    expect: number;
    reply: number[][];
};

/**
 * A fake dumper that runs a scripted sequence of exchanges, recording
 * the replies it is fed and which of its objects have been freed.
 */
class FakeDumper implements CartridgeDumper {
    fed: number[][] = [];
    freed = false;
    exchangesFreed = 0;
    failFeed = false;
    private index = 0;

    constructor(private steps: Step[]) {}

    next_exchange(): CartridgeExchange | undefined {
        const step = this.steps[this.index];
        if (!step) return undefined;
        this.index++;
        return {
            send_eager: () => new Uint8Array(step.send),
            expect: () => step.expect,
            free: () => {
                this.exchangesFreed++;
            }
        };
    }

    feed_wa(data: Uint8Array) {
        if (this.failFeed)
            throw new Error("Invalid acknowledgement from the device");
        this.fed.push(Array.from(data));
    }

    progress(): number {
        return this.index / this.steps.length;
    }

    device(): string | undefined {
        return "GBxCart RW v1.3 (firmware L1)";
    }

    finish_wa(): Dump {
        return { title: () => "PETAL LANES" } as unknown as Dump;
    }

    free() {
        this.freed = true;
    }
}

/**
 * A fake Web Serial API port that replies to each write with the chunks
 * of the next step, recording what was written and its state.
 */
type FakePort = CartridgePort & {
    written: number[][];
    baudRate?: number;
    flushed: boolean;
    closed: boolean;
    end: () => void;
};

/**
 * Builds a fake serial port that replies with the chunks of each one
 * of the provided steps, in order, as data is written to it.
 *
 * @param steps The steps with the chunks of the replies.
 * @returns The fake serial port.
 */
const buildPort = (steps: Step[]): FakePort => {
    let controller: ReadableStreamDefaultController<Uint8Array>;
    let index = 0;
    const port: FakePort = {
        written: [],
        flushed: false,
        closed: false,
        readable: new ReadableStream<Uint8Array>({
            start: (value) => {
                controller = value;
            }
        }),
        writable: new WritableStream<Uint8Array>({
            write: (chunk) => {
                port.written.push(Array.from(chunk));
                const step = steps[index++];
                step?.reply.forEach((data) =>
                    controller.enqueue(new Uint8Array(data))
                );
            },
            close: () => {
                port.flushed = true;
            }
        }),
        open: async ({ baudRate }) => {
            port.baudRate = baudRate;
        },
        close: async () => {
            port.closed = true;
        },
        end: () => controller.close()
    };
    return port;
};

/**
 * The conversation of a short dump, with a firmware query, a command
 * without reply and a read of four bytes.
 */
const STEPS: Step[] = [
    {
        send: [0xa1],
        expect: 9,
        reply: [[0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea]]
    },
    { send: [0xa8], expect: 0, reply: [] },
    { send: [0xb1], expect: 4, reply: [[0x50, 0x4c, 0x67, 0x00]] }
];

describe("Cartridge", function () {
    describe("#hasCartridgeSupport()", function () {
        const navigator = Object.getOwnPropertyDescriptor(
            globalThis,
            "navigator"
        );

        afterEach(() => {
            if (navigator) {
                Object.defineProperty(globalThis, "navigator", navigator);
            } else {
                delete (globalThis as { navigator?: unknown }).navigator;
            }
        });

        it("should be able to detect the lack of the Web Serial API", () => {
            Object.defineProperty(globalThis, "navigator", {
                value: {},
                configurable: true
            });
            assert.strictEqual(hasCartridgeSupport(), false);
        });

        it("should be able to detect the Web Serial API", () => {
            Object.defineProperty(globalThis, "navigator", {
                value: { serial: {} },
                configurable: true
            });
            assert.strictEqual(hasCartridgeSupport(), true);
        });
    });

    describe("#requestCartridgePort()", function () {
        const navigator = Object.getOwnPropertyDescriptor(
            globalThis,
            "navigator"
        );

        afterEach(() => {
            if (navigator) {
                Object.defineProperty(globalThis, "navigator", navigator);
            } else {
                delete (globalThis as { navigator?: unknown }).navigator;
            }
        });

        it("should be able to request a port of a GBxCart RW", async () => {
            const port = buildPort([]);
            let filters: unknown;
            Object.defineProperty(globalThis, "navigator", {
                value: {
                    serial: {
                        requestPort: async (options: { filters: unknown }) => {
                            filters = options.filters;
                            return port;
                        }
                    }
                },
                configurable: true
            });
            assert.strictEqual(await requestCartridgePort(), port);
            assert.deepStrictEqual(filters, CARTRIDGE_FILTERS);
        });
    });

    describe("#readCartridge()", function () {
        it("should be able to read a cartridge", async () => {
            const port = buildPort(STEPS);
            const dumper = new FakeDumper(STEPS);
            const progress: number[] = [];
            const { dump, device } = await readCartridge(port, {
                dumper: dumper,
                onProgress: (value) => progress.push(value)
            });
            assert.strictEqual(dump.title(), "PETAL LANES");
            assert.strictEqual(device, "GBxCart RW v1.3 (firmware L1)");
            assert.strictEqual(port.baudRate, CARTRIDGE_BAUD_RATE);
            assert.deepStrictEqual(port.written, [[0xa1], [0xa8], [0xb1]]);
            assert.deepStrictEqual(dumper.fed, [
                [0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea],
                [],
                [0x50, 0x4c, 0x67, 0x00]
            ]);
            assert.deepStrictEqual(progress, [1 / 3, 2 / 3, 1]);
            assert.strictEqual(dumper.exchangesFreed, 3);
            assert.strictEqual(dumper.freed, true);
            assert.strictEqual(port.flushed, true);
            assert.strictEqual(port.closed, true);
        });

        it("should be able to join replies split in chunks", async () => {
            const steps: Step[] = [
                {
                    send: [0xa1],
                    expect: 9,
                    reply: [
                        [0x08, 0x4c],
                        [0x00, 0x01, 0x04],
                        [0x65, 0xe2, 0x1d, 0xea]
                    ]
                }
            ];
            const dumper = new FakeDumper(steps);
            await readCartridge(buildPort(steps), { dumper: dumper });
            assert.deepStrictEqual(dumper.fed, [
                [0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea]
            ]);
        });

        it("should be able to keep extra bytes for the next exchange", async () => {
            const steps: Step[] = [
                { send: [0xb4], expect: 1, reply: [[0x01, 0x50, 0x4c]] },
                { send: [0xb1], expect: 2, reply: [] },
                { send: [0xb1], expect: 1, reply: [[0x67]] }
            ];
            const dumper = new FakeDumper(steps);
            await readCartridge(buildPort(steps), { dumper: dumper });
            assert.deepStrictEqual(dumper.fed, [[0x01], [0x50, 0x4c], [0x67]]);
        });

        it("should be able to fail when the device does not reply", async () => {
            const steps: Step[] = [{ send: [0xa1], expect: 9, reply: [] }];
            const port = buildPort(steps);
            const dumper = new FakeDumper(steps);
            await assert.rejects(
                readCartridge(port, { dumper: dumper, timeout: 20 }),
                {
                    message:
                        "The device is not responding, check that it runs the L firmware"
                }
            );
            assert.strictEqual(dumper.exchangesFreed, 1);
            assert.strictEqual(dumper.freed, true);
            assert.strictEqual(port.closed, true);
        });

        it("should be able to fail on a partial reply", async () => {
            const steps: Step[] = [
                { send: [0xa1], expect: 9, reply: [[0x08, 0x4c]] }
            ];
            const port = buildPort(steps);
            await assert.rejects(
                readCartridge(port, {
                    dumper: new FakeDumper(steps),
                    timeout: 20
                }),
                { message: /not responding/ }
            );
            assert.strictEqual(port.closed, true);
        });

        it("should be able to fail when the connection is closed", async () => {
            const steps: Step[] = [{ send: [0xa1], expect: 9, reply: [] }];
            const port = buildPort(steps);
            const dumper = new FakeDumper(steps);
            port.end();
            await assert.rejects(readCartridge(port, { dumper: dumper }), {
                message: "The connection to the device was closed"
            });
            assert.strictEqual(dumper.freed, true);
            assert.strictEqual(port.closed, true);
        });

        it("should be able to fail when a reply is rejected", async () => {
            const port = buildPort(STEPS);
            const dumper = new FakeDumper(STEPS);
            dumper.failFeed = true;
            await assert.rejects(readCartridge(port, { dumper: dumper }), {
                message: "Invalid acknowledgement from the device"
            });
            assert.deepStrictEqual(port.written, [[0xa1]]);
            assert.strictEqual(dumper.exchangesFreed, 1);
            assert.strictEqual(dumper.freed, true);
            assert.strictEqual(port.flushed, false);
            assert.strictEqual(port.closed, true);
        });

        it("should be able to fail without the streams of the port", async () => {
            const port = buildPort(STEPS);
            port.readable = null;
            const dumper = new FakeDumper(STEPS);
            await assert.rejects(readCartridge(port, { dumper: dumper }), {
                message: "Unable to access the serial port of the device"
            });
            assert.strictEqual(port.closed, true);
        });

        it("should be able to fail when the port can not be opened", async () => {
            const port = buildPort(STEPS);
            port.open = async () => {
                throw new Error("Failed to open serial port");
            };
            const dumper = new FakeDumper(STEPS);
            await assert.rejects(readCartridge(port, { dumper: dumper }), {
                message: "Failed to open serial port"
            });
            assert.deepStrictEqual(port.written, []);
            assert.strictEqual(dumper.freed, true);
        });
    });
});
