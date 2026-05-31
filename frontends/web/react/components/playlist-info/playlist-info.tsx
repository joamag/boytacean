import { Info, Pair, PanelTab, Paragraph } from "emukit";
import React, { FC, useMemo } from "react";

import { Playlist } from "../../../ts";

import "./playlist-info.css";

type PlaylistInfoProps = {
    playlist: Playlist;
    style?: string[];
};

export const PlaylistInfo: FC<PlaylistInfoProps> = ({
    playlist,
    style = []
}) => {
    const classes = useMemo(
        () => ["playlist-info", ...style].join(" "),
        [style]
    );
    const tabs = [PlaylistInfoMain({ playlist })];
    const tabNames = ["Main"];
    if (playlist.description) {
        tabs.push(PlaylistInfoDescription({ playlist }));
        tabNames.push("Description");
    }
    return (
        <div className={classes}>
            <PanelTab tabs={tabs} tabNames={tabNames} flex={true} />
        </div>
    );
};

type PlaylistInfoTabProps = {
    playlist: Playlist;
};

export const PlaylistInfoMain: FC<PlaylistInfoTabProps> = ({ playlist }) => (
    <div className="playlist-info-main">
        <Info>
            {playlist.name && (
                <Pair key="name" name="Name" value={playlist.name} />
            )}
            {playlist.version && (
                <Pair key="version" name="Version" value={playlist.version} />
            )}
            {playlist.author && (
                <Pair key="author" name="Author" value={playlist.author} />
            )}
            {playlist.authorEmail && (
                <Pair
                    key="author-email"
                    name="Email"
                    value={playlist.authorEmail}
                />
            )}
            {playlist.releaseDate && (
                <Pair
                    key="release-date"
                    name="Release Date"
                    value={playlist.releaseDate}
                />
            )}
            <Pair
                key="entries"
                name="Entries"
                value={String(playlist.entries.length)}
            />
        </Info>
    </div>
);

export const PlaylistInfoDescription: FC<PlaylistInfoTabProps> = ({
    playlist
}) => <Paragraph>{playlist.description}</Paragraph>;

export default PlaylistInfo;
