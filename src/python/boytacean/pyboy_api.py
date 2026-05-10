from typing import Any, List, Tuple, Union

try:
    from PIL.Image import Image, frombytes
except ImportError:
    Image = Any
    frombytes = Any

from .gb import GameBoy

VRAM_OFFSET = 0x8000
LCDC_OFFSET = 0xFF40
OAM_OFFSET = 0xFE00
LOW_TILEMAP = 0x9800
HIGH_TILEMAP = 0x9C00
LOW_TILEDATA = 0x8000
HIGH_TILEDATA = 0x8800
LOW_TILEDATA_NTILES = 0x100
TILES = 384
TILES_CGB = 768
SPRITES = 40
ROWS = 144
COLS = 160

TILE_BYTES = 16
TILE_WIDTH = 8
TILE_HEIGHT = 8


def _decode_tile_pixels(data: bytes) -> List[int]:
    # decodes the 16-byte 2bpp tile data into a flat list of 64
    # pixel values in the range 0..3, row-major
    pixels = [0] * (TILE_WIDTH * TILE_HEIGHT)
    for row in range(TILE_HEIGHT):
        low = data[row * 2]
        high = data[row * 2 + 1]
        for col in range(TILE_WIDTH):
            bit = 7 - col
            value = ((high >> bit) & 0x1) << 1 | ((low >> bit) & 0x1)
            pixels[row * TILE_WIDTH + col] = value
    return pixels


def _shade_to_rgba(value: int) -> Tuple[int, int, int, int]:
    # converts a 0..3 shade index into an RGBA quadruple, using the
    # canonical PyBoy DMG palette (white → black) so the produced
    # PIL/numpy buffers match the upstream colour ordering
    table = (
        (0xFF, 0xFF, 0xFF, 0xFF),
        (0xAA, 0xAA, 0xAA, 0xFF),
        (0x55, 0x55, 0x55, 0xFF),
        (0x00, 0x00, 0x00, 0xFF),
    )
    return table[value & 0x3]


class Tile:
    """
    Read-only handle to a single 8x8 tile in VRAM, identified either
    by its global tile identifier or by its data address. Mirrors the
    modern PyBoy `pyboy.api.tile.Tile` shape
    """

    shape: Tuple[int, int] = (TILE_WIDTH, TILE_HEIGHT)
    raw_buffer_format: str = "RGBA"

    def __init__(self, system: "GameBoy", identifier: int):
        self.tile_identifier = identifier
        if identifier >= TILES:
            self.vram_bank = 1
            local_id = identifier - TILES
        else:
            self.vram_bank = 0
            local_id = identifier
        self.data_address = LOW_TILEDATA + local_id * TILE_BYTES
        self._system = system

    def _pixels(self) -> List[int]:
        vram = self._system.vram()
        base = self.vram_bank * 0x2000 + (self.data_address - VRAM_OFFSET)
        return _decode_tile_pixels(vram[base : base + TILE_BYTES])

    def image(self) -> Image:
        rgba = bytes(b for value in self._pixels() for b in _shade_to_rgba(value))
        return frombytes("RGBA", (TILE_WIDTH, TILE_HEIGHT), rgba, "raw")

    def ndarray(self):
        from numpy import array, uint8

        rgba = [list(_shade_to_rgba(value)) for value in self._pixels()]
        return array(rgba, dtype=uint8).reshape((TILE_HEIGHT, TILE_WIDTH, 4))

    def __eq__(self, other) -> bool:
        if not isinstance(other, Tile):
            return NotImplemented
        return (
            self.data_address == other.data_address
            and self.vram_bank == other.vram_bank
        )

    def __hash__(self) -> int:
        return hash((self.data_address, self.vram_bank))

    def __repr__(self) -> str:
        return f"Tile: {self.tile_identifier}"


class Sprite:
    """
    Read-only handle to a single sprite (object) in OAM, mirroring the
    modern PyBoy `pyboy.api.sprite.Sprite` shape with x/y offsets and
    attribute-byte decoding
    """

    def __init__(self, system: "GameBoy", index: int):
        if not 0 <= index < SPRITES:
            raise ValueError(f"Sprite index out of range: {index}")
        oam = system.oam()
        offset = index * 4
        raw_y = oam[offset]
        raw_x = oam[offset + 1]
        tile = oam[offset + 2]
        attr = oam[offset + 3]
        lcdc = system.read_memory(LCDC_OFFSET)
        sprite_height = 16 if lcdc & 0x04 else 8

        self._sprite_index = index
        self.x = raw_x - 8
        self.y = raw_y - 16
        self.tile_identifier = tile
        self.attr_obj_bg_priority = bool(attr & 0x80)
        self.attr_y_flip = bool(attr & 0x40)
        self.attr_x_flip = bool(attr & 0x20)
        self.attr_palette_number = (attr >> 4) & 0x1
        self.attr_cgb_bank_number = (attr >> 3) & 0x1
        self.shape = (TILE_WIDTH, sprite_height)
        self.tiles = [Tile(system, tile)]
        if sprite_height == 16:
            self.tiles.append(Tile(system, (tile & 0xFE) + 1))
        self.on_screen = -sprite_height < self.y < ROWS and -TILE_WIDTH < self.x < COLS

    def __eq__(self, other) -> bool:
        if not isinstance(other, Sprite):
            return NotImplemented
        return self._sprite_index == other._sprite_index

    def __hash__(self) -> int:
        return hash(("Sprite", self._sprite_index))

    def __repr__(self) -> str:
        return (
            f"Sprite [{self._sprite_index}]: Position: ({self.x}, {self.y}), "
            f"Shape: {self.shape}, Tiles: ({self.tiles}), "
            f"On screen: {self.on_screen}"
        )


class TileMap:
    """
    Read-only handle to one of the two background tile maps (32x32
    tiles each) in VRAM, mirroring the modern PyBoy
    `pyboy.api.tilemap.TileMap` shape with bracket access and
    background/window selection
    """

    shape: Tuple[int, int] = (32, 32)

    def __init__(self, system: "GameBoy", select: str):
        if select not in ("BACKGROUND", "WINDOW"):
            raise KeyError(f"Unknown tilemap: {select!r}")
        self._select = select
        self._system = system
        self._use_tile_objects = False

    @property
    def map_offset(self) -> int:
        lcdc = self._system.read_memory(LCDC_OFFSET)
        if self._select == "BACKGROUND":
            return HIGH_TILEMAP if lcdc & 0x08 else LOW_TILEMAP
        return HIGH_TILEMAP if lcdc & 0x40 else LOW_TILEMAP

    @property
    def signed_tile_data(self) -> bool:
        lcdc = self._system.read_memory(LCDC_OFFSET)
        return not bool(lcdc & 0x10)

    def use_tile_objects(self, switch: bool):
        self._use_tile_objects = bool(switch)

    def tile(self, column: int, row: int) -> Tile:
        return Tile(self._system, self.tile_identifier(column, row))

    def tile_identifier(self, column: int, row: int) -> int:
        if not 0 <= column < 32 or not 0 <= row < 32:
            raise ValueError(f"Tilemap coordinate out of range: ({column}, {row})")
        addr = self.map_offset + row * 32 + column
        raw = self._system.read_memory(addr)
        if self.signed_tile_data:
            # convert the signed offset relative to 0x9000 into a flat
            # 0..255 identifier in the high tile-data window
            signed = raw if raw < 128 else raw - 256
            return LOW_TILEDATA_NTILES + signed
        return raw

    def search_for_identifiers(self, identifiers: List[int]) -> List[List[List[int]]]:
        results: List[List[List[int]]] = [[] for _ in identifiers]
        for row in range(32):
            for column in range(32):
                tid = self.tile_identifier(column, row)
                for index, identifier in enumerate(identifiers):
                    if tid == identifier:
                        results[index].append([column, row])
        return results

    def __getitem__(self, key) -> Union[int, Tile, List, List[List]]:
        column, row = self._unpack_key(key)
        if isinstance(column, slice) or isinstance(row, slice):
            cols = self._slice_to_indices(column)
            rows = self._slice_to_indices(row)
            single_col = not isinstance(column, slice)
            single_row = not isinstance(row, slice)
            if single_row:
                return [self._lookup(c, rows[0]) for c in cols]
            if single_col:
                return [self._lookup(cols[0], r) for r in rows]
            return [[self._lookup(c, r) for c in cols] for r in rows]
        return self._lookup(column, row)

    def __repr__(self) -> str:
        rows = []
        for row in range(32):
            cells = [f"{self.tile_identifier(c, row):3d}" for c in range(32)]
            rows.append(" ".join(cells))
        return "\n".join(rows)

    def _lookup(self, column: int, row: int) -> Union[int, Tile]:
        if self._use_tile_objects:
            return self.tile(column, row)
        return self.tile_identifier(column, row)

    def _unpack_key(self, key) -> Tuple[Union[int, slice], Union[int, slice]]:
        if isinstance(key, tuple):
            if len(key) != 2:
                raise ValueError(f"Expected (column, row), got {key!r}")
            return key[0], key[1]
        return key, slice(None)

    def _slice_to_indices(self, value) -> List[int]:
        if isinstance(value, slice):
            start, stop, step = value.indices(32)
            return list(range(start, stop, step))
        return [value]
