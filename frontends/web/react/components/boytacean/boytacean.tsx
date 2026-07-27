import { EmulatorApp } from "emukit";
import React, {
    CSSProperties,
    forwardRef,
    useCallback,
    useEffect,
    useImperativeHandle,
    useMemo,
    useRef,
    useState
} from "react";
import { RecoilRoot } from "recoil";

// the imports are deliberately made against the concrete modules rather
// than the barrel, as `ts/gb.ts` imports the sibling components from the
// react barrel and that would introduce a circular dependency
import {
    BACKGROUNDS,
    containBodyBackground,
    EMBED_STORAGE_PREFIX,
    MOUNT_CLASS
} from "../../../ts/embed";
import { GameboyEmulator } from "../../../ts/gb";

export type BoytaceanProps = {
    /**
     * The URL of the ROM to be loaded at startup, the remote origin
     * must support CORS. In case it's not provided the default ROM
     * that is bundled with the emulator is used instead.
     */
    romUrl?: string;
    /**
     * The URL of a JSON playlist file with a list of ROMs, enables
     * the playlist section in the emulator UI.
     */
    playlistUrl?: string;
    /**
     * The name of the palette to be used at startup (eg: `christmas`,
     * `hogwards`, `mariobros`, etc.).
     */
    palette?: string;
    /**
     * The background color (hexadecimal, without the `#` prefix) to be
     * used in the emulator area.
     */
    background?: string;
    /**
     * The list of background colors made available in the UI.
     */
    backgrounds?: string[];
    /**
     * If the emulator should occupy the complete area of its container.
     */
    fullscreen?: boolean;
    /**
     * If the on-screen keyboard should start visible.
     */
    keyboard?: boolean;
    /**
     * If the debug panels should start visible.
     */
    debug?: boolean;
    /**
     * If information should be logged in verbose mode, implied by
     * {@link BoytaceanProps.debug}.
     */
    verbose?: boolean;
    /**
     * If the informative panels should be shown.
     */
    info?: boolean;
    /**
     * The prefix used to namespace the `localStorage` keys, set it to an
     * empty string to share the storage with the standalone front-end.
     */
    storagePrefix?: string;
    /**
     * If the page level side effects of the emulator UI (namely the
     * painting of the document's body with the background color) should
     * be contained within the component.
     */
    contain?: boolean;
    className?: string;
    style?: CSSProperties;
    /**
     * Called once the emulator has booted and is running, receives the
     * emulator instance for the more advanced use cases.
     */
    onReady?: (emulator: GameboyEmulator) => void;
    /**
     * Called when the boot process fails (eg: the ROM cannot be fetched).
     */
    onError?: (error: Error) => void;
    /**
     * Called whenever the background color changes through the UI.
     */
    onBackground?: (background: string) => void;
};

/**
 * The imperative handle exposed through the component's ref, allows the
 * control of the emulator from the parent component.
 */
export type BoytaceanHandle = {
    /**
     * The underlying emulator instance, `null` until the emulator has
     * finished booting.
     */
    readonly emulator: GameboyEmulator | null;
    /**
     * Suspends the emulation, no cycles are executed while paused.
     */
    pause: () => Promise<void>;
    /**
     * Resumes a previously paused emulation.
     */
    resume: () => Promise<void>;
    /**
     * Loads a new ROM from a remote URL, replacing the one currently
     * running, the remote origin must support CORS.
     */
    loadRom: (romUrl: string) => Promise<void>;
};

/**
 * Game Boy emulator component, renders the complete emulator UI inside
 * the React tree of the host application.
 *
 * The emulator is created once on mount, the props that control its
 * initial state are therefore only read at that point, use the ref
 * handle (or the `onReady` emulator instance) to drive it afterwards.
 *
 * @example
 * <Boytacean romUrl="https://example.com/game.gb" palette="christmas" />
 */
export const Boytacean = forwardRef<BoytaceanHandle, BoytaceanProps>(
    (
        {
            romUrl,
            playlistUrl,
            palette,
            background,
            backgrounds = BACKGROUNDS,
            fullscreen = false,
            keyboard = false,
            debug = false,
            verbose = false,
            info = false,
            storagePrefix = EMBED_STORAGE_PREFIX,
            contain = true,
            className,
            style,
            onReady,
            onError,
            onBackground
        },
        ref
    ) => {
        const [emulator, setEmulator] = useState<GameboyEmulator | null>(null);
        const [color, setColor] = useState<string | undefined>(background);

        // the callbacks are kept in refs so that the boot effect does not
        // have to depend on them, a parent re-rendering with new inline
        // handlers must not re-create the emulator
        const onReadyRef = useRef(onReady);
        const onErrorRef = useRef(onError);
        onReadyRef.current = onReady;
        onErrorRef.current = onError;

        // creates and initializes the emulator, this is deliberately
        // separated from the boot below so that the emulator UI is
        // already mounted by the time the emulation starts
        useEffect(() => {
            let cancelled = false;
            let instance: GameboyEmulator | null = null;

            (async () => {
                instance = new GameboyEmulator(
                    { background: background, debug: debug || verbose },
                    { storagePrefix: storagePrefix }
                );
                if (playlistUrl) {
                    instance.playlistUrl = playlistUrl;
                    await instance.loadPlaylist();
                }
                await instance.init();
                if (cancelled) return;
                setEmulator(instance);
            })().catch((err) => onErrorRef.current?.(err as Error));

            return () => {
                cancelled = true;
                instance?.pause();
                setEmulator(null);
            };
            // eslint-disable-next-line react-hooks/exhaustive-deps
        }, []);

        // boots the emulator once the UI is mounted, waiting for the
        // booted event instead of the start promise, as the latter only
        // settles when the emulation loop terminates
        useEffect(() => {
            if (!emulator) return;

            let cancelled = false;
            const onBooted = () => {
                emulator.unbind("booted", onBooted);
                if (cancelled) return;
                onReadyRef.current?.(emulator);
            };
            emulator.bind("booted", onBooted);
            emulator
                .start({
                    romUrl: romUrl ?? emulator.defaultRomUrl ?? undefined
                })
                .catch((err) => {
                    if (cancelled) return;
                    onErrorRef.current?.(err as Error);
                });

            return () => {
                cancelled = true;
                emulator.unbind("booted", onBooted);
            };
            // eslint-disable-next-line react-hooks/exhaustive-deps
        }, [emulator]);

        // keeps the emulator UI from painting the host application's
        // body with the current background color
        useEffect(() => {
            if (!contain) return;
            return containBodyBackground();
        }, [contain]);

        const handleBackground = useCallback(
            (value: string) => {
                setColor(value);
                onBackground?.(value);
            },
            [onBackground]
        );

        useImperativeHandle(
            ref,
            () => ({
                get emulator() {
                    return emulator;
                },
                pause: async () => await emulator?.pause(),
                resume: async () => await emulator?.resume(),
                loadRom: async (url: string) =>
                    await emulator?.boot({ loadRom: true, romPath: url })
            }),
            [emulator]
        );

        const sections = useMemo(
            () => (playlistUrl ? ["Playlist"] : []),
            [playlistUrl]
        );

        const containerStyle = useMemo(
            () => ({
                ...(contain && color
                    ? { backgroundColor: `#${color}` }
                    : undefined),
                ...style
            }),
            [contain, color, style]
        );

        const classes = [contain ? MOUNT_CLASS : undefined, className]
            .filter(Boolean)
            .join(" ");

        return (
            <div className={classes} style={containerStyle}>
                {emulator && (
                    <RecoilRoot>
                        <EmulatorApp
                            emulator={emulator}
                            fullscreen={fullscreen}
                            info={info}
                            debug={debug}
                            keyboard={keyboard}
                            sections={sections}
                            palette={palette}
                            background={background}
                            backgrounds={backgrounds}
                            onBackground={handleBackground}
                        />
                    </RecoilRoot>
                )}
            </div>
        );
    }
);

Boytacean.displayName = "Boytacean";
