use boytacean::ppu::{Palette, PaletteInfo};

pub struct PaletteInfoStatic {
    name: &'static str,
    colors: Palette,
}

impl PaletteInfoStatic {
    pub fn to_palette_info(&self) -> PaletteInfo {
        PaletteInfo::new(self.name, self.colors)
    }
}

static PALETTES: [PaletteInfoStatic; 7] = [
    PaletteInfoStatic {
        name: "basic",
        colors: [
            [0xff, 0xff, 0xff],
            [0xc0, 0xc0, 0xc0],
            [0x60, 0x60, 0x60],
            [0x00, 0x00, 0x00],
        ],
    },
    PaletteInfoStatic {
        name: "hogwards",
        colors: [
            [0xb6, 0xa5, 0x71],
            [0x8b, 0x7e, 0x56],
            [0x55, 0x4d, 0x35],
            [0x20, 0x1d, 0x13],
        ],
    },
    PaletteInfoStatic {
        name: "christmas",
        colors: [
            [0xe8, 0xe7, 0xdf],
            [0x8b, 0xab, 0x95],
            [0x9e, 0x5c, 0x5e],
            [0x53, 0x4d, 0x57],
        ],
    },
    PaletteInfoStatic {
        name: "goldsilver",
        colors: [
            [0xc5, 0xc6, 0x6d],
            [0x97, 0xa1, 0xb0],
            [0x58, 0x5e, 0x67],
            [0x23, 0x52, 0x29],
        ],
    },
    PaletteInfoStatic {
        name: "pacman",
        colors: [
            [0xff, 0xff, 0x00],
            [0xff, 0xb8, 0x97],
            [0x37, 0x32, 0xff],
            [0x00, 0x00, 0x00],
        ],
    },
    PaletteInfoStatic {
        name: "mariobros",
        colors: [
            [0xf7, 0xce, 0xc3],
            [0xcc, 0x9e, 0x22],
            [0x92, 0x34, 0x04],
            [0x00, 0x00, 0x00],
        ],
    },
    PaletteInfoStatic {
        name: "pokemon",
        colors: [
            [0xf8, 0x78, 0x00],
            [0xb8, 0x60, 0x00],
            [0x78, 0x38, 0x00],
            [0x00, 0x00, 0x00],
        ],
    },
];

pub fn get_palette_names() -> Vec<String> {
    PALETTES.iter().map(|p| p.name.to_string()).collect()
}

pub fn get_palette(name: String) -> PaletteInfo {
    for palette in PALETTES.iter() {
        if palette.name == name {
            return palette.to_palette_info();
        }
    }
    PALETTES[0].to_palette_info()
}
