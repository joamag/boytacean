/**
 * Represents a single entry in a playlist of ROMs
 * that can be loaded by the emulator.
 */
export type PlaylistEntry = {
    name: string;
    url: string;
    description?: string;
    thumbnail?: string;
    thumbnailSmall?: string;
    igdbId?: number;
};

/**
 * Represents a playlist of ROMs that can be loaded
 * by the emulator, including metadata about the
 * playlist itself.
 */
export type Playlist = {
    name?: string;
    description?: string;
    version?: string;
    author?: string;
    authorEmail?: string;
    releaseDate?: string;
    defaultUrl?: string;
    useThumbnail?: boolean;
    entries: PlaylistEntry[];
};

/**
 * Fetches a playlist from a remote URL and parses
 * its JSON contents into a playlist structure.
 *
 * @param url The URL of the JSON playlist file.
 * @returns The parsed playlist structure.
 */
export const fetchPlaylist = async (url: string): Promise<Playlist> => {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Problem retrieving playlist (${response.status})`);
    }
    const data = await response.json();

    // supports both a raw array of entries and a
    // structured playlist object with metadata
    if (Array.isArray(data)) {
        return { entries: data };
    }

    return data as Playlist;
};
