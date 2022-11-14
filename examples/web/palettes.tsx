export const PALETTES = [
    {
        name: "basic",
        colors: ["ffffff", "c0c0c0", "606060", "000000"]
    },
    {
        name: "hogwards",
        colors: ["b6a571", "8b7e56", "554d35", "201d13"]
    },
    {
        name: "christmas",
        colors: ["e8e7df", "8bab95", "9e5c5e", "534d57"]
    },
    {
        name: "goldsilver",
        colors: ["c5c66d", "97a1b0", "585e67", "232529"]
    },
    {
        name: "pacman",
        colors: ["ffff00", "ffb897", "3732ff", "000000"]
    },
    {
        name: "mariobros",
        colors: ["f7cec3", "cc9e22", "923404", "000000"]
    },
    {
        name: "pokemon",
        colors: ["f87800", "b86000", "783800", "000000"]
    }
];

export const PALETTES_MAP = Object.fromEntries(
    PALETTES.map((v) => [v.name, v])
);
