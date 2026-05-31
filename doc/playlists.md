# Playlists

The Boytacean web front-end can load a **playlist**: a JSON file describing a collection of ROMs that can be browsed, searched, and booted directly from the emulator UI. This is useful for homebrew showcases, ROM hack collections, demos, or any curated set of games hosted online.

## Using a playlist

Point the emulator at a playlist by passing its URL through the `playlist_url` (or the shorter `playlist`) GET parameter:

```text
https://boytacean.joao.me?playlist_url=https://example.com/playlist.json
```

When a playlist URL is provided:

- The **Playlist** section becomes available in the sidebar (toggled with the list icon) and is shown by default.
- Each entry can be searched by name or description and booted with a single click, which fetches the ROM and starts emulation.
- If the playlist defines a [`defaultUrl`](#playlist-fields), that ROM is booted automatically at startup (unless an explicit `rom_url` is also supplied, which takes precedence).

The playlist file is fetched over HTTP, so the host **must allow CORS**. The same applies to every ROM (and thumbnail) URL referenced inside the playlist — they are fetched directly by the browser.

## JSON format

A playlist can be expressed in two equivalent shapes.

### Array form

The simplest form is a raw array of [entries](#entry-fields). This omits all playlist-level metadata:

```json
[
    {
        "name": "My Homebrew Game",
        "url": "https://example.com/roms/game.gb"
    },
    {
        "name": "Another Demo",
        "url": "https://example.com/roms/demo.gbc",
        "description": "A colourful CGB demo."
    }
]
```

### Object form

The richer form wraps the entries in an object, allowing playlist-level metadata (name, author, description, default ROM, etc.):

```json
{
    "name": "My Playlist",
    "description": "A collection of homebrew games.",
    "version": "1.0.0",
    "author": "Jane Doe",
    "authorEmail": "jane@example.com",
    "releaseDate": "2026-05-31",
    "defaultUrl": "https://example.com/roms/game.gb",
    "useThumbnail": true,
    "entries": [
        {
            "name": "My Homebrew Game",
            "url": "https://example.com/roms/game.gb",
            "description": "The flagship game of this collection.",
            "thumbnail": "https://example.com/thumbs/game.png",
            "thumbnailSmall": "https://example.com/thumbs/game-small.png",
            "igdbId": 12345
        }
    ]
}
```

## Playlist fields

These fields apply to the playlist as a whole and are only available in the [object form](#object-form).

| Field          | Type      | Required | Description                                                                                              |
| -------------- | --------- | -------- | -------------------------------------------------------------------------------------------------------- |
| `entries`      | Entry[]   | Yes      | The list of ROM entries. See [Entry fields](#entry-fields).                                              |
| `name`         | String    | No       | Display name of the playlist, shown in the section header and the info panel.                            |
| `description`  | String    | No       | Free-text description, shown in the header and in a dedicated "Description" tab of the info panel.       |
| `version`      | String    | No       | Playlist version, shown in the info panel.                                                               |
| `author`       | String    | No       | Author name, shown as a "by …" subtitle and in the info panel.                                           |
| `authorEmail`  | String    | No       | Author contact email, shown in the info panel.                                                           |
| `releaseDate`  | String    | No       | Release date of the playlist (free-form text), shown in the info panel.                                  |
| `defaultUrl`   | String    | No       | URL of a ROM to boot automatically at startup, used as a fallback when no `rom_url` parameter is given.  |
| `useThumbnail` | Boolean   | No       | Controls thumbnail selection per entry. See [Thumbnails](#thumbnails). Defaults to `false`.              |

## Entry fields

Each entry describes a single loadable ROM.

| Field            | Type   | Required | Description                                                                                                          |
| ---------------- | ------ | -------- | -------------------------------------------------------------------------------------------------------------------- |
| `name`           | String | Yes      | Display name of the game. Included in the search filter.                                                             |
| `url`            | String | Yes      | URL of the ROM to fetch and boot when the entry is clicked. Must be unique within the playlist and CORS-accessible.  |
| `description`    | String | No       | Short description shown under the name. Included in the search filter.                                               |
| `thumbnail`      | String | No       | URL of a (typically larger) cover image shown next to the entry.                                                     |
| `thumbnailSmall` | String | No       | URL of a smaller cover image, preferred by default to keep lists lightweight. See [Thumbnails](#thumbnails).         |
| `igdbId`         | Number | No       | The [IGDB](https://www.igdb.com) identifier of the game, reserved for metadata and future integrations.              |

## Thumbnails

An entry may provide `thumbnail`, `thumbnailSmall`, or both. Which one is used depends on the playlist-level `useThumbnail` flag:

- `useThumbnail: false` (default) — prefers `thumbnailSmall`, falling back to `thumbnail` when the small variant is absent. This keeps long lists light.
- `useThumbnail: true` — prefers the larger `thumbnail`, falling back to `thumbnailSmall`.

If neither field is set, no image is shown for the entry.

## Search

The search box filters entries case-insensitively across both the `name` and `description` fields, so a descriptive entry is easier to find in large playlists.

## ROM naming

When an entry is booted, the ROM name is derived from the last path segment of its `url` (query string stripped). Keeping a meaningful file name and extension (`.gb` / `.gbc`) in the URL therefore produces a cleaner display name.

## Error handling

Failures are reported through non-blocking toast notifications and the console logger, leaving the rest of the UI usable:

- If the playlist URL cannot be retrieved or parsed, the playlist simply fails to load.
- If an individual ROM cannot be fetched, a toast reports the failure for that entry while the playlist remains browsable.
