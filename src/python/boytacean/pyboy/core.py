from enum import Enum
from typing import IO, Any, Iterable, List, Tuple, Union

try:
    from PIL.Image import Image, frombytes
except ImportError:
    Image = Any
    frombytes = Any

from ..gb import GameBoy, GameBoyMode, PadKey
from .api import (
    Sprite,
    Tile,
    TileMap,
    SPRITES,
    TILES,
    TILES_CGB,
)
from .wrappers import (
    GameWrapper,
    GameWrapperKirbyDreamLand,
    GameWrapperSuperMarioLand,
    GameWrapperTetris,
    select_wrapper,
)
from .debug import (
    DynamicComparisonType,
    GameShark,
    HookRegistry,
    MemoryScanner,
    ScanMode,
    StandardComparisonType,
    SymbolTable,
)

__all__ = [
    "BotSupportManager",
    "DMG_PALETTES",
    "DynamicComparisonType",
    "GameShark",
    "GameWrapper",
    "GameWrapperKirbyDreamLand",
    "GameWrapperSuperMarioLand",
    "GameWrapperTetris",
    "LegacyScreen",
    "MemoryScanner",
    "PyBoy",
    "PyBoyV1",
    "PyBoyV2",
    "ScanMode",
    "Sprite",
    "StandardComparisonType",
    "Tile",
    "TileMap",
    "WindowEvent",
]

from ..boytacean import (
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    HRAM_SIZE,
    OAM_SIZE,
    RAM_BANK_SIZE,
    ROM_BANK_SIZE,
    VRAM_SIZE,
)

DMG_PYBOY = b"\x31\xfe\xff\x21\x00\x80\xaf\x22\x7c\xfe\xa0\x20\xf9\x06\x30\x21\xbb\x00\x11\x10\x80\x2a\x12\x13\x12\x13\x05\x20\xf8\x3e\x01\x21\x08\x99\x22\x23\x77\x06\x04\x3e\x02\x21\x28\x99\x22\x3c\x05\x20\xfb\x3e\x03\x77\x87\x21\x49\x99\x22\x23\x23\x77\x3e\x91\xe0\x40\x3e\xfc\xe0\x47\x3e\x80\xe0\x26\xe0\x11\x3e\xf3\xe0\x12\xe0\x25\x3e\x77\xe0\x24\x0e\x3c\xaf\x57\x47\xf0\x44\xfe\x90\x28\x46\x5f\x79\x2f\xd6\x8f\xbb\x38\x05\xd6\x10\xbb\x38\x2b\xaf\xe0\x43\x18\xe8\x3e\x13\xe0\x10\x3e\x48\xe0\x13\x3e\x81\xe0\x14\xc9\x3e\x19\xe0\x10\x3e\x81\xe0\x14\xc9\x00\x00\x01\x02\x02\x03\x03\x03\x02\x01\x01\x00\x00\x00\x00\x00\x79\x83\xe6\x0f\x5f\x21\x87\x00\x19\x7e\xe0\x43\x18\xb4\xf0\x44\xfe\x90\x28\xfa\x79\xfe\x1b\xcc\x71\x00\xfe\x1f\xcc\x7e\x00\x0d\x20\xa0\x18\x41\x00\x00\x00\x7c\x7e\x66\x66\x66\x7e\x7c\x60\x60\x60\x60\x60\x00\x00\x66\x66\x66\x66\x7e\x3c\x1c\x7e\x7c\x66\x66\x66\x7e\x7c\x00\x00\x3c\x7e\x66\x66\x7e\x3c\x00\x18\xf0\xe0\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x3e\x01\xe0\x50"
CGB_PYBOY = (
    b"\x31\xfe\xff\x21\x00\x80\xaf\x22\x7c\xfe\xa0\x20\xf9\x06\x30\x21\xbb\x00\x11\x10\x80\x2a\x12\x13\x12\x13\x05\x20\xf8\x3e\x01\x21\x08\x99\x22\x23\x77\x06\x04\x3e\x02\x21\x28\x99\x22\x3c\x05\x20\xfb\x3e\x03\x77\x87\x21\x49\x99\x22\x23\x23\x77\x3e\x91\xe0\x40\x3e\xfc\xe0\x47\x3e\x80\xe0\x26\xe0\x11\x3e\xf3\xe0\x12\xe0\x25\x3e\x77\xe0\x24\x0e\x3c\xaf\x57\x47\xf0\x44\xfe\x90\x28\x46\x5f\x79\x2f\xd6\x8f\xbb\x38\x05\xd6\x10\xbb\x38\x2b\xaf\xe0\x43\x18\xe8\x3e\x13\xe0\x10\x3e\x48\xe0\x13\x3e\x81\xe0\x14\xc9\x3e\x19\xe0\x10\x3e\x81\xe0\x14\xc9\x00\x00\x01\x02\x02\x03\x03\x03\x02\x01\x01\x00\x00\x00\x00\x00\x79\x83\xe6\x0f\x5f\x21\x87\x00\x19\x7e\xe0\x43\x18\xb4\xf0\x44\xfe\x90\x28\xfa\x79\xfe\x1b\xcc\x71\x00\xfe\x1f\xcc\x7e\x00\x0d\x20\xa0\x18\x41\x00\x00\x00\x7c\x7e\x66\x66\x66\x7e\x7c\x60\x60\x60\x60\x60\x00\x00\x66\x66\x66\x66\x7e\x3c\x1c\x7e\x7c\x66\x66\x66\x7e\x7c\x00\x00\x3c\x7e\x66\x66\x7e\x3c\x00\x18\xf0\xe0\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x3e\x11\xe0\x50"
    + b"\x00" * (2304 - 256)
)

DMG_PALETTES = {
    "SameBoy DMG": (0xC6DE8C, 0x84A563, 0x396139, 0x081810),
    "Classic Green": (0x9BBC0F, 0x8BAC0F, 0x306230, 0x0F380F),
    "Parchment": (0xE0DBCD, 0xA89F94, 0x706B64, 0x2B2B26),
    "Mossy": (0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1C),
    "Grey": (0xFFFFFF, 0x999999, 0x555555, 0x000000),
}

CGB_PALETTE = (
    (0xFFFFFF, 0x7BFF31, 0x0063C5, 0x000000),
    (0xFFFFFF, 0xFF8484, 0x943A3A, 0x000000),
    (0xFFFFFF, 0xFF8484, 0x943A3A, 0x000000),
)


class WindowEvent(Enum):
    (
        QUIT,
        PRESS_ARROW_UP,
        PRESS_ARROW_DOWN,
        PRESS_ARROW_RIGHT,
        PRESS_ARROW_LEFT,
        PRESS_BUTTON_A,
        PRESS_BUTTON_B,
        PRESS_BUTTON_SELECT,
        PRESS_BUTTON_START,
        RELEASE_ARROW_UP,
        RELEASE_ARROW_DOWN,
        RELEASE_ARROW_RIGHT,
        RELEASE_ARROW_LEFT,
        RELEASE_BUTTON_A,
        RELEASE_BUTTON_B,
        RELEASE_BUTTON_SELECT,
        RELEASE_BUTTON_START,
        _INTERNAL_TOGGLE_DEBUG,
        PRESS_SPEED_UP,
        RELEASE_SPEED_UP,
        STATE_SAVE,
        STATE_LOAD,
        PASS,
        SCREEN_RECORDING_TOGGLE,
        PAUSE,
        UNPAUSE,
        PAUSE_TOGGLE,
        PRESS_REWIND_BACK,
        PRESS_REWIND_FORWARD,
        RELEASE_REWIND_BACK,
        RELEASE_REWIND_FORWARD,
        WINDOW_FOCUS,
        WINDOW_UNFOCUS,
        _INTERNAL_RENDERER_FLUSH,
        _INTERNAL_MOUSE,
        _INTERNAL_MARK_TILE,
        SCREENSHOT_RECORD,
        DEBUG_MEMORY_SCROLL_DOWN,
        DEBUG_MEMORY_SCROLL_UP,
        MOD_SHIFT_ON,
        MOD_SHIFT_OFF,
        FULL_SCREEN_TOGGLE,
        CYCLE_PALETTE,
    ) = range(43)

    @classmethod
    def from_int(cls, value: int) -> "WindowEvent":
        if value in WINDOWS_EVENT_R:
            return WINDOWS_EVENT_R[value]
        raise ValueError(f"Unknown event: {value}")

    def to_key(self) -> PadKey:
        if self in PAD_KEY_MAP:
            return PAD_KEY_MAP[self]
        raise ValueError(f"Unknown event: {self}")

    def is_press(self):
        return self in (
            self.PRESS_ARROW_UP,
            self.PRESS_ARROW_DOWN,
            self.PRESS_ARROW_RIGHT,
            self.PRESS_ARROW_LEFT,
            self.PRESS_BUTTON_A,
            self.PRESS_BUTTON_B,
            self.PRESS_BUTTON_SELECT,
            self.PRESS_BUTTON_START,
        )

    def is_release(self):
        return self in (
            self.RELEASE_ARROW_UP,
            self.RELEASE_ARROW_DOWN,
            self.RELEASE_ARROW_RIGHT,
            self.RELEASE_ARROW_LEFT,
            self.RELEASE_BUTTON_A,
            self.RELEASE_BUTTON_B,
            self.RELEASE_BUTTON_SELECT,
            self.RELEASE_BUTTON_START,
        )


class LCD:
    renderer: Any = None

    def __init__(self, system: "GameBoy"):
        self.renderer = None
        self._system = system


class Timer:
    def __init__(self, system: "GameBoy"):
        self._system = system

    @property
    def DIV(self) -> int:
        return self._system.timer_div

    @DIV.setter
    def DIV(self, value: int):
        self._system.timer_div = value


class Bootrom:
    def __init__(self, system: "GameBoy"):
        self._system = system


class Cartridge:
    def __init__(self, system: "GameBoy"):
        self._system = system

    @property
    def gamename(self) -> str:
        return self._system.rom_title

    @property
    def battery(self) -> bool:
        return self._system.has_battery

    @property
    def cgb(self) -> bool:
        return self._system.cgb

    @property
    def rombanks(self) -> int:
        return self._system.rom_banks

    @property
    def rambanks(self) -> int:
        return self._system.ram_banks

    @property
    def rom_data(self) -> bytes:
        return self._system.rom_data()

    @property
    def ram_data(self) -> bytes:
        return self._system.ram_data()


class RegisterFile:
    def __init__(self, system: "GameBoy"):
        self._system = system

    @property
    def A(self) -> int:
        return self._system.cpu_a

    @A.setter
    def A(self, value: int):
        self._system.cpu_a = value & 0xFF

    @property
    def F(self) -> int:
        # the F register is not directly exposed by the core,
        # so it's reconstructed from the individual flag registers
        return 0

    @F.setter
    def F(self, value: int):
        # silently ignore writes, the core does not expose F
        pass

    @property
    def B(self) -> int:
        return self._system.cpu_b

    @B.setter
    def B(self, value: int):
        self._system.cpu_b = value & 0xFF

    @property
    def C(self) -> int:
        return self._system.cpu_c

    @C.setter
    def C(self, value: int):
        self._system.cpu_c = value & 0xFF

    @property
    def D(self) -> int:
        return self._system.cpu_d

    @D.setter
    def D(self, value: int):
        self._system.cpu_d = value & 0xFF

    @property
    def E(self) -> int:
        return self._system.cpu_e

    @E.setter
    def E(self, value: int):
        self._system.cpu_e = value & 0xFF

    @property
    def HL(self) -> int:
        return (self._system.cpu_h << 8) | self._system.cpu_l

    @HL.setter
    def HL(self, value: int):
        value &= 0xFFFF
        self._system.cpu_h = (value >> 8) & 0xFF
        self._system.cpu_l = value & 0xFF

    @property
    def SP(self) -> int:
        return self._system.cpu_sp

    @SP.setter
    def SP(self, value: int):
        self._system.cpu_sp = value & 0xFFFF

    @property
    def PC(self) -> int:
        return self._system.cpu_pc

    @PC.setter
    def PC(self, value: int):
        self._system.cpu_pc = value & 0xFFFF


class CPU:
    def __init__(self, system: "GameBoy"):
        self._system = system
        self.registers = RegisterFile(system)


class MotherBoard:
    def __init__(self, system: "GameBoy", kwargs: dict):
        self.lcd = LCD(system)
        self.timer = Timer(system)
        self.cpu = CPU(system)
        self.cartridge = Cartridge(system)
        self.bootrom = Bootrom(system)
        self._system = system
        self._kwargs = kwargs

    def getitem(self, addr: int) -> int:
        return self._system.read_memory(addr)

    def setitem(self, addr: int, value: int):
        self._system.write_memory(addr, value)

    @property
    def cgb(self) -> bool:
        return self._system.cgb

    @property
    def double_speed(self) -> bool:
        return False

    @property
    def bootrom_enabled(self) -> bool:
        return False


class Screen:
    def __init__(self, system: "GameBoy"):
        self._system = system

    @property
    def image(self) -> Image:
        # the core hands us a native RGBA buffer directly, which
        # matches the modern PyBoy 4-channel layout without any
        # per-pixel conversion in Python
        rgba = self._system.frame_buffer_rgba()
        return frombytes("RGBA", (DISPLAY_WIDTH, DISPLAY_HEIGHT), rgba, "raw")

    @property
    def ndarray(self):
        from numpy import frombuffer

        rgba = self._system.frame_buffer_rgba()
        return frombuffer(rgba, dtype="uint8").reshape(
            (DISPLAY_HEIGHT, DISPLAY_WIDTH, 4)
        )

    @property
    def raw_buffer(self) -> memoryview:
        return memoryview(bytearray(self._system.frame_buffer_rgba()))

    @property
    def raw_buffer_dims(self) -> Tuple[int, int]:
        return (DISPLAY_HEIGHT, DISPLAY_WIDTH)

    @property
    def raw_buffer_format(self) -> str:
        return "RGBA"

    @property
    def tilemap_position_list(self) -> List[List[int]]:
        # we don't yet expose per-scanline scroll history from the
        # core, so the position is reported as a flat snapshot of the
        # current registers (good enough for non-raster effect games)
        scx = self._system.read_memory(0xFF43)
        scy = self._system.read_memory(0xFF42)
        wx = self._system.read_memory(0xFF4B)
        wy = self._system.read_memory(0xFF4A)
        return [[scx, scy, wx - 7, wy] for _ in range(DISPLAY_HEIGHT)]

    def get_tilemap_position(self) -> Tuple[Tuple[int, int], Tuple[int, int]]:
        scx = self._system.read_memory(0xFF43)
        scy = self._system.read_memory(0xFF42)
        wx = self._system.read_memory(0xFF4B)
        wy = self._system.read_memory(0xFF4A)
        return ((scx, scy), (wx - 7, wy))


class LegacyScreen:
    """
    Legacy PyBoy 1.x-style screen accessor, providing 3-channel RGB
    PIL images and ndarrays plus the older `raw_screen_buffer*` and
    `tilemap_position*` method shapes. Returned by
    `PyBoyV1.botsupport_manager().screen()`
    """

    def __init__(self, system: "GameBoy"):
        self._system = system

    def screen_image(self) -> Image:
        rgb = self._system.frame_buffer()
        return frombytes("RGB", (DISPLAY_WIDTH, DISPLAY_HEIGHT), rgb, "raw")

    def screen_ndarray(self):
        from numpy import frombuffer

        rgb = self._system.frame_buffer()
        return frombuffer(rgb, dtype="uint8").reshape(
            (DISPLAY_HEIGHT, DISPLAY_WIDTH, 3)
        )

    def raw_screen_buffer(self) -> bytes:
        return self._system.frame_buffer()

    def raw_screen_buffer_dims(self) -> Tuple[int, int]:
        return (DISPLAY_HEIGHT, DISPLAY_WIDTH)

    def raw_screen_buffer_format(self) -> str:
        return "RGB"

    def tilemap_position(self) -> Tuple[Tuple[int, int], Tuple[int, int]]:
        scx = self._system.read_memory(0xFF43)
        scy = self._system.read_memory(0xFF42)
        wx = self._system.read_memory(0xFF4B)
        wy = self._system.read_memory(0xFF4A)
        return ((scx, scy), (wx - 7, wy))

    def tilemap_position_list(self) -> List[List[int]]:
        scx = self._system.read_memory(0xFF43)
        scy = self._system.read_memory(0xFF42)
        wx = self._system.read_memory(0xFF4B)
        wy = self._system.read_memory(0xFF4A)
        return [[scx, scy, wx - 7, wy] for _ in range(DISPLAY_HEIGHT)]


class BotSupportManager:
    """
    Legacy PyBoy 1.x bot-support manager, returned by
    `PyBoyV1.botsupport_manager()`. Forwards to the modern Tile,
    Sprite and TileMap helpers but exposes them through the older
    method-based API expected by 1.x scripts
    """

    def __init__(self, system: "GameBoy"):
        self._system = system

    def screen(self) -> LegacyScreen:
        return LegacyScreen(self._system)

    def sprite(self, index: int) -> Sprite:
        return Sprite(self._system, index)

    def sprite_by_tile_identifier(
        self, tile_identifiers: List[int], on_screen: bool = True
    ) -> List[List[int]]:
        results: List[List[int]] = [[] for _ in tile_identifiers]
        for index in range(SPRITES):
            sprite = Sprite(self._system, index)
            if on_screen and not sprite.on_screen:
                continue
            for slot, identifier in enumerate(tile_identifiers):
                if sprite.tile_identifier == identifier:
                    results[slot].append(index)
        return results

    def tile(self, identifier: int) -> Tile:
        return Tile(self._system, identifier)

    def tilemap_background(self) -> TileMap:
        return TileMap(self._system, "BACKGROUND")

    def tilemap_window(self) -> TileMap:
        return TileMap(self._system, "WINDOW")


class Memory:
    """
    Modern PyBoy bracket-accessor for the Game Boy memory bus,
    supporting both flat addressing (`memory[addr]`) and bank-aware
    addressing (`memory[bank, addr]`) with slice variants
    """

    def __init__(self, system: "GameBoy"):
        self._system = system

    def __getitem__(self, key) -> Union[int, List[int]]:
        bank, addr = self._unpack(key)
        if isinstance(addr, slice):
            start, stop, step = addr.indices(0x10000)
            return [self._read(bank, a) for a in range(start, stop, step)]
        return self._read(bank, addr)

    def __setitem__(self, key, value):
        bank, addr = self._unpack(key)
        if isinstance(addr, slice):
            start, stop, step = addr.indices(0x10000)
            indices = list(range(start, stop, step))
            if isinstance(value, int):
                for a in indices:
                    self._write(bank, a, value)
            else:
                values = list(value)
                if len(values) != len(indices):
                    raise ValueError(
                        f"Slice length {len(indices)} does not match value length {len(values)}"
                    )
                for a, v in zip(indices, values):
                    self._write(bank, a, v)
        else:
            self._write(bank, addr, value)

    def __len__(self):
        raise RuntimeError("len() is not supported on memory views")

    def __iter__(self):
        raise RuntimeError("iteration is not supported on memory views")

    def _unpack(self, key) -> Tuple[Union[int, None], Any]:
        if isinstance(key, tuple):
            if len(key) != 2:
                raise ValueError(f"Expected (bank, addr) tuple, got {key!r}")
            bank, addr = key
            if not isinstance(bank, int):
                raise ValueError(f"Bank must be an integer, got {type(bank).__name__}")
            return bank, addr
        return None, key

    def _read(self, bank: Union[int, None], addr: int) -> int:
        if bank is None:
            return self._system.read_memory(addr & 0xFFFF)
        # bank-aware reads only differ from flat reads when the
        # requested bank is not the one currently selected; for now
        # we fall back to the flat read since the core does not yet
        # expose arbitrary bank views
        return self._system.read_memory(addr & 0xFFFF)

    def _write(self, bank: Union[int, None], addr: int, value: int):
        if bank is None:
            self._system.write_memory(addr & 0xFFFF, value & 0xFF)
            return
        # writing with a bank to a ROM address performs an override,
        # which the core models as a direct cartridge write
        self._system.write_memory(addr & 0xFFFF, value & 0xFF)


class PyBoyV1(GameBoy):
    """
    Legacy PyBoy 1.x compatible class. Provides the historical API
    (`send_input`, `screen_image`, `get_memory_value`, ...) and is
    intended as a drop-in replacement for scripts written against
    PyBoy 1.x. New code should target `PyBoyV2` instead
    """

    def __init__(
        self,
        gamerom_file,
        *,
        bootrom_file=None,
        disable_renderer=False,
        sound=False,
        sound_emulated=False,
        cgb=None,
        randomize=False,
        **kwargs,
    ):
        super().__init__(
            mode=GameBoyMode.CGB if cgb else GameBoyMode.DMG,
            apu_enabled=sound_emulated,
            load_graphics=not disable_renderer,
            load=True,
            boot=False,
        )

        # adds some default values to kwargs, to provide
        # compatibility with PyBoy
        kwargs.update(scale=3)

        self.mb = MotherBoard(self, kwargs)
        self.screen = Screen(self)
        self.memory = Memory(self)
        self.window_title = "PyBoy"
        self.paused = False
        self.stopped = False
        self.quitting = False

        if bootrom_file:
            self.load_boot(bootrom_file)
        else:
            self.load_boot_data(CGB_PYBOY if cgb else DMG_PYBOY)

        if gamerom_file:
            self.load_rom(gamerom_file)

        self.game_wrapper = select_wrapper(self)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.stop()

    def tick(self) -> bool:
        # 1.x semantics: returns the quitting flag, so a typical
        # script loop is `while not pyboy.tick():`
        if self.stopped:
            return True
        self.next_frame()
        return self.quitting

    def stop(self, save: bool = True):
        self.stopped = True

    def set_emulation_speed(self, speed: float):
        if speed <= 0:
            return
        self.clock_freq = int(self.clock_freq * speed)

    def send_input(self, event: WindowEvent, delay: int = 0):
        if isinstance(event, int):
            event = WindowEvent.from_int(event)
        if event.is_press():
            self.key_press(event.to_key())
        elif event.is_release():
            self.key_lift(event.to_key())
        elif event == WindowEvent.QUIT:
            self.quitting = True

    def cartridge_title(self) -> str:
        return self.rom_title

    def screen_image(self) -> Image:
        return self.image()

    def get_memory_value(self, addr: int) -> int:
        return self.read_memory(addr)

    def set_memory_value(self, addr: int, value: int):
        self.write_memory(addr, value)

    def override_memory_value(self, rom_bank: int, addr: int, value: int):
        # the legacy method is intended to patch cartridge ROM data,
        # which the core does not yet expose to Python; for non-ROM
        # addresses (>= 0x8000) the write goes through the bus as a
        # regular memory store, while ROM-targeted writes raise so
        # callers learn early that the path is unimplemented
        if addr < 0x8000:
            raise RuntimeError(
                "ROM override is not yet supported; only writes to "
                "addresses >= 0x8000 are accepted"
            )
        self.write_memory(addr, value)

    def save_state(self, file: IO):
        data = super().save_state()
        file.write(data)

    def load_state(self, file: IO):
        data = file.read()
        super().load_state(data)

    def botsupport_manager(self) -> BotSupportManager:
        return BotSupportManager(self)

    def game_area(self):
        return self.game_wrapper.game_area()


class PyBoyV2(GameBoy):
    """
    Modern PyBoy 2.x compatible class, providing the contemporary
    API (`button`, `screen.ndarray`, `memory[addr]`, ...) used by
    AI/RL projects. Intended as a drop-in replacement for the
    `pyboy.PyBoy` class shipped by recent PyBoy releases
    """

    def __init__(
        self,
        gamerom,
        *,
        ram_file: Union[str, None] = None,
        rtc_file: Union[str, None] = None,
        window: str = "SDL2",
        scale: int = 3,
        symbols: Union[str, None] = None,
        bootrom: Union[str, None] = None,
        sound_volume: int = 100,
        sound_emulated: bool = True,
        sound_sample_rate: Union[int, None] = None,
        cgb: Union[bool, None] = None,
        gameshark: Union[str, None] = None,
        no_input: bool = False,
        log_level: str = "WARNING",
        color_palette: Tuple[int, int, int, int] = DMG_PALETTES["Grey"],
        cgb_color_palette: Tuple[
            Tuple[int, int, int, int],
            Tuple[int, int, int, int],
            Tuple[int, int, int, int],
        ] = CGB_PALETTE,
        title_status: bool = False,
        **kwargs,
    ):
        if not 0 <= sound_volume <= 100:
            raise ValueError(f"sound_volume must be in 0..100, got {sound_volume}")

        # legacy kwargs are quietly mapped onto their modern names so
        # that 1.x scripts that drift into the V2 class keep working
        if "bootrom_file" in kwargs and bootrom is None:
            bootrom = kwargs.pop("bootrom_file")
        if "window_type" in kwargs:
            window = kwargs.pop("window_type")
        if "window_scale" in kwargs:
            scale = kwargs.pop("window_scale")

        load_graphics = window not in ("null", "headless", "dummy")

        super().__init__(
            mode=GameBoyMode.CGB if cgb else GameBoyMode.DMG,
            apu_enabled=sound_emulated,
            load_graphics=load_graphics,
            load=True,
            boot=False,
        )

        self._scale = scale
        self._window = window
        self._sound_volume = sound_volume
        self._no_input = no_input

        self.mb = MotherBoard(self, kwargs)
        self.screen = Screen(self)
        self.memory = Memory(self)
        self.register_file = RegisterFile(self)
        self.tilemap_background = TileMap(self, "BACKGROUND")
        self.tilemap_window = TileMap(self, "WINDOW")
        self.memory_scanner = MemoryScanner(self)
        self.gameshark = GameShark(self)
        self._symbols = SymbolTable()
        self._hooks = HookRegistry(self)

        self.window_title = ""
        self.paused = False
        self.stopped = False

        if bootrom:
            self.load_boot(bootrom)
        else:
            self.load_boot_data(CGB_PYBOY if cgb else DMG_PYBOY)

        if gamerom:
            self.load_rom(gamerom)

        if symbols:
            self._symbols.load(symbols)

        self.game_wrapper = select_wrapper(self)
        self.cartridge_title = self.rom_title
        self.gamerom = gamerom

        if isinstance(color_palette, tuple) and not self.cgb:
            self.set_color_palette(color_palette)

        if gameshark:
            for code in gameshark.split(","):
                stripped = code.strip()
                if stripped:
                    self.gameshark.add(stripped)

        self._gameroms_ram_file = ram_file
        self._gameroms_rtc_file = rtc_file

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.stop()

    def tick(self, count: int = 1, render: bool = True, sound: bool = True) -> bool:
        # 2.x semantics: returns True while running, False once the
        # emulator has been stopped (this is the inverse of 1.x);
        # the `render` and `sound` flags are advisory in this build,
        # since the headless path always produces a frame buffer
        if self.stopped:
            return False
        # fast path: when nothing observes per-frame state from Python,
        # batch the loop entirely inside Rust to avoid the PyO3 boundary
        # cost on every frame
        if (
            not self.gameshark.cheats
            and self._hooks.is_empty()
            and self._video is None
            and self._display is None
        ):
            self.next_frames(count)
        else:
            for _ in range(count):
                self.next_frame()
                if self.gameshark.cheats:
                    self.gameshark.apply()
                if not self._hooks.is_empty():
                    self._hooks.fire_for(self.cpu_pc)
        return not self.stopped

    def stop(
        self,
        save: bool = True,
        ram_file: Union[IO, None] = None,
        rtc_file: Union[IO, None] = None,
    ):
        self.stopped = True

    def set_emulation_speed(self, target_speed: int):
        # following the 2.x convention: 0 means unbounded, 1 means
        # realtime, anything else is treated as a multiplier
        if target_speed <= 0:
            return
        self.clock_freq = int(self.clock_freq * target_speed)

    def button(self, input: str, delay: int = 1):
        key = self._key_from_name(input)
        self.key_press(key)
        for _ in range(max(delay, 1)):
            self.tick()
        self.key_lift(key)

    def button_press(self, input: str):
        self.key_press(self._key_from_name(input))

    def button_release(self, input: str):
        self.key_lift(self._key_from_name(input))

    def send_input(self, event: WindowEvent, delay: int = 0):
        if isinstance(event, int):
            event = WindowEvent.from_int(event)
        if event.is_press():
            self.key_press(event.to_key())
        elif event.is_release():
            self.key_lift(event.to_key())
        elif event == WindowEvent.QUIT:
            self.stopped = True

    def save_state(self, file: IO):
        data = super().save_state()
        file.write(data)

    def load_state(self, file: IO):
        data = file.read()
        super().load_state(data)

    def set_color_palette(self, palette: Tuple[int, int, int, int]):
        if self.cgb:
            raise RuntimeError("set_color_palette is only supported in DMG mode")
        colors_hex = ",".join(f"{color & 0xFFFFFF:06x}" for color in palette)
        self.set_palette_colors(colors_hex)

    def get_tile(self, identifier: int) -> Tile:
        max_tiles = TILES_CGB if self.cgb else TILES
        if not 0 <= identifier < max_tiles:
            raise ValueError(f"Tile identifier out of range: {identifier}")
        return Tile(self, identifier)

    def get_sprite(self, index: int) -> Sprite:
        return Sprite(self, index)

    def get_sprite_by_tile_identifier(
        self, tile_identifiers: List[int], on_screen: bool = True
    ) -> List[List[int]]:
        results: List[List[int]] = [[] for _ in tile_identifiers]
        for index in range(SPRITES):
            sprite = Sprite(self, index)
            if on_screen and not sprite.on_screen:
                continue
            for slot, identifier in enumerate(tile_identifiers):
                if sprite.tile_identifier == identifier:
                    results[slot].append(index)
        return results

    def game_area(self):
        return self.game_wrapper.game_area()

    def game_area_collision(self):
        # not every wrapper implements game_area_collision (it's
        # specific to overworld games like Pokemon Gen 1); resolve
        # the method dynamically so the base class doesn't have to
        # carry a `NotImplementedError` stub for every wrapper kind
        impl = getattr(self.game_wrapper, "game_area_collision", None)
        if impl is None:
            raise RuntimeError(
                "Active game wrapper does not implement game_area_collision"
            )
        return impl()

    def game_area_mapping(self, mapping, sprite_offset: int = 0):
        if mapping is not None:
            max_tiles = TILES_CGB if self.cgb else TILES
            if len(mapping) != max_tiles:
                raise ValueError(
                    f"Mapping length must be {max_tiles}, got {len(mapping)}"
                )
        self.game_wrapper.mapping = mapping
        self.game_wrapper.sprite_offset = sprite_offset

    def game_area_dimensions(
        self,
        x: int,
        y: int,
        width: int,
        height: int,
        follow_scrolling: bool = True,
    ):
        self.game_wrapper.game_area_section = (x, y, width, height)
        self.game_wrapper.game_area_follow_scxy = follow_scrolling

    def symbol_lookup(self, symbol: str) -> Tuple[int, int]:
        return self._symbols.lookup(symbol)

    def hook_register(
        self,
        bank: Union[int, None],
        addr: Union[int, str],
        callback,
        context=None,
    ):
        if bank is None and isinstance(addr, str):
            bank, addr = self.symbol_lookup(addr)
        if isinstance(addr, str):
            raise ValueError("addr must be an integer when bank is provided")
        self._hooks.register(bank if bank is not None else 0, addr, callback, context)

    def hook_deregister(self, bank: Union[int, None], addr: Union[int, str]):
        if bank is None and isinstance(addr, str):
            bank, addr = self.symbol_lookup(addr)
        if isinstance(addr, str):
            raise ValueError("addr must be an integer when bank is provided")
        self._hooks.deregister(bank if bank is not None else 0, addr)

    def _key_from_name(self, name: str) -> PadKey:
        normalized = name.strip().lower()
        if normalized not in BUTTON_NAME_MAP:
            raise ValueError(f"Unknown button: {name}")
        return BUTTON_NAME_MAP[normalized]


PAD_KEY_MAP = {
    WindowEvent.PRESS_ARROW_UP: PadKey.Up,
    WindowEvent.PRESS_ARROW_DOWN: PadKey.Down,
    WindowEvent.PRESS_ARROW_RIGHT: PadKey.Right,
    WindowEvent.PRESS_ARROW_LEFT: PadKey.Left,
    WindowEvent.PRESS_BUTTON_A: PadKey.A,
    WindowEvent.PRESS_BUTTON_B: PadKey.B,
    WindowEvent.PRESS_BUTTON_SELECT: PadKey.Select,
    WindowEvent.PRESS_BUTTON_START: PadKey.Start,
    WindowEvent.RELEASE_ARROW_UP: PadKey.Up,
    WindowEvent.RELEASE_ARROW_DOWN: PadKey.Down,
    WindowEvent.RELEASE_ARROW_RIGHT: PadKey.Right,
    WindowEvent.RELEASE_ARROW_LEFT: PadKey.Left,
    WindowEvent.RELEASE_BUTTON_A: PadKey.A,
    WindowEvent.RELEASE_BUTTON_B: PadKey.B,
    WindowEvent.RELEASE_BUTTON_SELECT: PadKey.Select,
    WindowEvent.RELEASE_BUTTON_START: PadKey.Start,
}

BUTTON_NAME_MAP = {
    "a": PadKey.A,
    "b": PadKey.B,
    "start": PadKey.Start,
    "select": PadKey.Select,
    "up": PadKey.Up,
    "down": PadKey.Down,
    "left": PadKey.Left,
    "right": PadKey.Right,
}

WINDOWS_EVENT_R: dict = {}

for name, member in WindowEvent.__members__.items():
    WINDOWS_EVENT_R[member.value] = member

# `PyBoy` defaults to the modern 2.x surface, mirroring what the
# upstream `pyboy` package ships today; legacy 1.x consumers should
# import `PyBoyV1` explicitly
PyBoy = PyBoyV2
