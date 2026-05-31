import { Emulator, Link, TextInput } from "emukit";
import React, { FC, useCallback, useMemo, useState } from "react";

import { Playlist, PlaylistEntry } from "../../../ts";
import { PlaylistInfo } from "../playlist-info/playlist-info";

import "./game-playlist.css";

type GamePlaylistProps = {
    entries: PlaylistEntry[];
    playlist?: Playlist;
    emulator: Emulator;
    onSelect?: (entry: PlaylistEntry) => void;
    style?: string[];
};

export const GamePlaylist: FC<GamePlaylistProps> = ({
    entries,
    playlist,
    emulator,
    onSelect,
    style = []
}) => {
    const classes = useMemo(
        () => ["game-playlist", ...style].join(" "),
        [style]
    );
    const [filter, setFilter] = useState("");
    const [loading, setLoading] = useState<string | null>(null);
    const [activeUrl, setActiveUrl] = useState<string | null>(null);

    const filteredEntries = useMemo(() => {
        if (!filter) return entries;
        const query = filter.toLowerCase();
        return entries.filter(
            (entry) =>
                entry.name.toLowerCase().includes(query) ||
                entry.description?.toLowerCase().includes(query)
        );
    }, [entries, filter]);

    const onInfoClick = useCallback(async () => {
        if (!playlist) return;
        await emulator.handlers.showModal?.(
            playlist.name ?? "Playlist Info",
            undefined,
            <PlaylistInfo playlist={playlist} />
        );
    }, [playlist, emulator]);

    const onEntryClick = useCallback(
        async (entry: PlaylistEntry) => {
            setLoading(entry.url);
            try {
                const response = await fetch(entry.url);
                if (!response.ok) {
                    throw new Error(
                        `Problem retrieving ROM (${response.status})`
                    );
                }
                const blob = await response.blob();
                const arrayBuffer = await blob.arrayBuffer();
                const romData = new Uint8Array(arrayBuffer);

                // extracts the file name from the URL to be
                // used as the ROM name for the emulator
                const urlParts = entry.url.split(/\//g);
                let romName = urlParts[urlParts.length - 1].split("?")[0];
                const romNameParts = romName.split(/\./g);
                romName = `${romNameParts[0]}.${romNameParts[romNameParts.length - 1]}`;

                await emulator.boot({
                    engine: null,
                    romName: romName,
                    romData: romData
                });
                setActiveUrl(entry.url);
                emulator.handlers.showToast?.(
                    `Loaded ${entry.name} successfully!`
                );
                onSelect?.(entry);
            } catch (err) {
                emulator.handlers.showToast?.(
                    `Failed to load ${entry.name}!`,
                    true
                );
                emulator.logger.error(`Failed to load ${entry.name} (${err})`);
            } finally {
                setLoading(null);
            }
        },
        [emulator, onSelect]
    );

    return (
        <div className={classes}>
            {playlist && (
                <div className="game-playlist-header">
                    <div className="game-playlist-header-left">
                        <span className="game-playlist-header-title">
                            {playlist.name ?? "Playlist"}
                        </span>
                        {playlist.author && (
                            <span className="game-playlist-header-subtitle">
                                by {playlist.author}
                            </span>
                        )}
                    </div>
                    <Link text="Info" onClick={onInfoClick} />
                </div>
            )}
            {playlist?.description && (
                <span className="game-playlist-header-description">
                    {playlist.description}
                </span>
            )}
            <TextInput
                size="medium"
                placeholder="Search games..."
                value={filter}
                onChange={setFilter}
            />
            <div className="game-playlist-list">
                {filteredEntries.length === 0 && (
                    <span className="game-playlist-empty">No games found.</span>
                )}
                {filteredEntries.map((entry) => (
                    <div
                        key={entry.url}
                        className={[
                            "game-playlist-entry",
                            loading === entry.url ? "loading" : "",
                            activeUrl === entry.url ? "active" : ""
                        ].join(" ")}
                        onClick={() => onEntryClick(entry)}
                    >
                        {(entry.thumbnail ?? entry.thumbnailSmall) && (
                            <img
                                className="game-playlist-thumbnail"
                                src={
                                    playlist?.useThumbnail
                                        ? (entry.thumbnail ??
                                          entry.thumbnailSmall)
                                        : (entry.thumbnailSmall ??
                                          entry.thumbnail)
                                }
                                alt={entry.name}
                            />
                        )}
                        <div className="game-playlist-info">
                            <div className="game-playlist-title">
                                <span className="game-playlist-name">
                                    {entry.name}
                                </span>
                                {loading === entry.url && (
                                    <span className="game-playlist-loading">
                                        Loading...
                                    </span>
                                )}
                            </div>
                            {entry.description && (
                                <span className="game-playlist-description">
                                    {entry.description}
                                </span>
                            )}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default GamePlaylist;
