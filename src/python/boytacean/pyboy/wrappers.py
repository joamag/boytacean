from typing import Any, Tuple, Union

from ..gb import GameBoy, PadKey
from .api import SPRITES, Sprite, TileMap

NEXT_TETROMINO_ADDR = 0xC213

TETROMINO_TABLE = {"L": 0, "J": 4, "I": 8, "O": 12, "Z": 16, "S": 20, "T": 24}
TETROMINO_TABLE_INVERSE = {value: name for name, value in TETROMINO_TABLE.items()}


def _digits_at(system: "GameBoy", base: int, length: int) -> int:
    # decodes a sequence of `length` ASCII-encoded digit tiles into a
    # base-10 integer; tiles map to 0x00..0x09 plus 0x2F for blank
    value = 0
    for offset in range(length):
        tile = system.read_memory(base + offset)
        if tile <= 9:
            value = value * 10 + tile
    return value


class GameWrapper:
    """
    Generic game wrapper, used when no specific cartridge match is
    found. Specific wrappers inherit from this base class and
    override the `cartridge_title`, `shape` and `post_tick` hooks
    """

    cartridge_title: Union[str, None] = None
    shape: Tuple[int, int] = (32, 32)
    game_area_section: Tuple[int, int, int, int] = (0, 0, 32, 32)
    game_area_follow_scxy: bool = False
    tilemap_use_background: bool = True

    def __init__(self, system: "GameBoy"):
        self._system = system
        self.game_has_started = False
        self.saved_state: Union[bytes, None] = None
        self.mapping: Any = None
        self.sprite_offset: int = 0

    def start_game(self, timer_div: Union[int, None] = None):
        if timer_div is not None:
            self._system.timer_div = timer_div
        self.saved_state = GameBoy.save_state(self._system)
        self.game_has_started = True
        self.post_tick()

    def reset_game(self, timer_div: Union[int, None] = None):
        if self.saved_state is None:
            raise RuntimeError("Game has not been started")
        GameBoy.load_state(self._system, self.saved_state)
        if timer_div is not None:
            self._system.timer_div = timer_div
        self.post_tick()

    def game_over(self) -> bool:
        raise NotImplementedError("Generic wrapper does not implement game_over")

    def post_tick(self):
        pass

    def game_area(self):
        from numpy import uint32, zeros

        tilemap: TileMap = (
            self._tilemap_background()
            if self.tilemap_use_background
            else self._tilemap_window()
        )
        x, y, w, h = self.game_area_section
        result = zeros((h, w), dtype=uint32)

        if self.game_area_follow_scxy:
            scx = self._system.read_memory(0xFF43) // 8
            scy = self._system.read_memory(0xFF42) // 8
        else:
            scx = scy = 0

        for row in range(h):
            for col in range(w):
                column = (x + col + scx) % 32
                rrow = (y + row + scy) % 32
                result[row, col] = tilemap.tile_identifier(column, rrow)
        return result

    def _tilemap_background(self) -> TileMap:
        return TileMap(self._system, "BACKGROUND")

    def _tilemap_window(self) -> TileMap:
        return TileMap(self._system, "WINDOW")

    def __repr__(self) -> str:
        if self.cartridge_title is None:
            return "GameWrapper(Generic)"
        return f"GameWrapper({self.cartridge_title})"


class GameWrapperTetris(GameWrapper):
    cartridge_title = "TETRIS"
    shape = (10, 18)
    game_area_section = (2, 0, 10, 18)
    game_area_follow_scxy = False

    score = 0
    level = 0
    lines = 0

    def start_game(self, timer_div: Union[int, None] = None):
        # advance past the title screens until the in-game playfield
        # has clearly been reached, then snapshot for reset_game
        if timer_div is not None:
            self._system.timer_div = timer_div
        self.saved_state = None
        for _ in range(60):
            self._system.next_frame()
        self._press_start(2)
        for _ in range(60):
            self._system.next_frame()
        self._press_start(2)
        for _ in range(60):
            self._system.next_frame()
        self._press_start(2)
        for _ in range(180):
            self._system.next_frame()
        self.saved_state = GameBoy.save_state(self._system)
        self.game_has_started = True
        self.post_tick()

    def post_tick(self):
        self.score = _digits_at(self._system, 0x9866, 6)
        self.level = _digits_at(self._system, 0x9867 + 0x60, 4)
        self.lines = _digits_at(self._system, 0x9866 + 0x90, 4)

    def next_tetromino(self) -> str:
        value = self._system.read_memory(NEXT_TETROMINO_ADDR)
        return TETROMINO_TABLE_INVERSE.get(value, "?")

    def set_tetromino(self, shape: str):
        if shape not in TETROMINO_TABLE:
            raise ValueError(f"Unknown tetromino: {shape}")
        # the upstream wrapper patches two ROM sites that bias the
        # spawn tables; on Boytacean the ROM is read-only from
        # Python, so we instead force the next-tetromino slot
        self._system.write_memory(NEXT_TETROMINO_ADDR, TETROMINO_TABLE[shape])

    def game_over(self) -> bool:
        # the losing condition raises a row of "L" pieces along the
        # top of the playfield; tile 135 is the "game over" sigil
        return self._system.read_memory(0x9866) == 135

    def _press_start(self, frames: int):
        # internal helper used by start_game to navigate the title
        # sequence without exposing button helpers on the wrapper
        self._system.key_press(PadKey.Start)
        for _ in range(frames):
            self._system.next_frame()
        self._system.key_lift(PadKey.Start)


class GameWrapperSuperMarioLand(GameWrapper):
    cartridge_title = "SUPER MARIOLAND"
    shape = (20, 16)
    game_area_section = (0, 2, 20, 16)
    game_area_follow_scxy = True

    world: Tuple[int, int] = (0, 0)
    coins: int = 0
    lives_left: int = 0
    score: int = 0
    time_left: int = 0
    level_progress: int = 0

    def post_tick(self):
        world_byte = self._system.read_memory(0xFFB4)
        self.world = ((world_byte >> 4) & 0xF, world_byte & 0xF)
        self.lives_left = _bcd(self._system.read_memory(0xDA15))
        self.coins = _bcd(self._system.read_memory(0xFFFA))
        self.score = _digits_at(self._system, 0x9820, 6)
        self.time_left = _digits_at(self._system, 0x9829, 3)

        scx = self._system.read_memory(0xFF43)
        level_block = self._system.read_memory(0xC0AB)
        mario_x = self._system.read_memory(0xC202)
        self.level_progress = level_block * 16 + (scx - 7) % 16 + mario_x

    def game_over(self) -> bool:
        return self._system.read_memory(0xC0A4) == 0x39

    def set_lives_left(self, amount: int):
        if not 0 <= amount <= 99:
            raise ValueError(f"Lives must be in 0..99, got {amount}")
        self._system.write_memory(0xDA15, _to_bcd(amount))


class GameWrapperKirbyDreamLand(GameWrapper):
    cartridge_title = "KIRBY DREAM LAN"
    shape = (20, 16)
    game_area_section = (0, 0, 20, 16)
    game_area_follow_scxy = True

    score: int = 0
    health: int = 0
    lives_left: int = 0

    def post_tick(self):
        self.score = (
            self._system.read_memory(0xD070) * 1000000
            + self._system.read_memory(0xD071) * 10000
            + self._system.read_memory(0xD072) * 100
            + self._system.read_memory(0xD073)
        )
        self.health = self._system.read_memory(0xD086)
        self.lives_left = max(self._system.read_memory(0xD089) - 1, 0)

    def game_over(self) -> bool:
        return self.health == 0 and self.lives_left == 0


WRAPPERS = {
    GameWrapperTetris.cartridge_title: GameWrapperTetris,
    GameWrapperSuperMarioLand.cartridge_title: GameWrapperSuperMarioLand,
    GameWrapperKirbyDreamLand.cartridge_title: GameWrapperKirbyDreamLand,
}


def select_wrapper(system: "GameBoy") -> GameWrapper:
    title = system.rom_title
    cls = WRAPPERS.get(title, GameWrapper)
    return cls(system)


def _bcd(value: int) -> int:
    return ((value >> 4) & 0xF) * 10 + (value & 0xF)


def _to_bcd(value: int) -> int:
    return ((value // 10) & 0xF) << 4 | (value % 10) & 0xF


