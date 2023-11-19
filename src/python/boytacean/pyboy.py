from enum import Enum
from sys import modules
from typing import Any

from PIL.Image import Image

from .gb import GameBoy, GameBoyMode, PadKey

PATCH_CYTHON = True
""" If the cython module should be patched to avoiding importing it
and compiling the pyboy modules to native code making them incompatible
with the Boytacean stubs """


class NonImportableModule:
    def __getattr__(self, name):
        raise ImportError("This module cannot be imported.")


if PATCH_CYTHON:
    modules["cython"] = NonImportableModule()  # type: ignore

try:
    from pyboy.plugins.manager import PluginManager
except ImportError:
    PluginManager = None

try:
    from pyboy.botsupport import BotSupportManager
except ImportError:
    BotSupportManager = None


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
    ) = range(42)

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
    def __init__(self, system: "PyBoy"):
        self.renderer = None  # @TODO: add something here
        self._system = system


class MotherBoard:
    def __init__(self, system: "PyBoy", kwargs: dict[str, Any]):
        self.lcd = LCD(system)
        self._system = system
        self._kwargs = kwargs

    def getitem(self, addr: int) -> int:
        return self._system.read_memory(addr)

    @property
    def cgb(self):
        return self._system._mode == GameBoyMode.CGB


class PyBoy(GameBoy):
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
            boot=not bool(bootrom_file),
        )

        # adds some default values to kwargs, to provide
        # compatibility with pyboy
        kwargs.update(scale=3)

        self.mb = MotherBoard(self, kwargs)

        if bootrom_file:
            self.load_boot_path(bootrom_file)
        if gamerom_file:
            self.load_rom(gamerom_file)

        # runs the plugin manager initialization at the end of the __init__
        # so that we have a working and running emulator to work with
        self.plugin_manager = (
            PluginManager(self, self.mb, kwargs) if PluginManager else None
        )

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.stop()

    def set_emulation_speed(self, speed: float):
        print("Missing emulation speed control")

    def tick(self):
        super().next_frame()

    def stop(self):
        pass

    def game_wrapper(self):
        if self.plugin_manager is None:
            raise RuntimeError("Plugins not enabled")
        return self.plugin_manager.gamewrapper()

    def botsupport_manager(self):
        if BotSupportManager is None:
            raise RuntimeError("BotSupport not enabled")
        return BotSupportManager(self, self.mb)

    def send_input(self, event: WindowEvent):
        if event.is_press():
            self.key_press(event.to_key())

        if event.is_release():
            self.key_lift(event.to_key())

    def cartridge_title(self) -> str:
        return self.rom_title

    def screen_image(self) -> Image:
        return self.image()

    def get_memory_value(self, addr: int) -> int:
        return self.read_memory(addr)

    def set_memory_value(self, addr: int, value: int):
        self.write_memory(addr, value)

    def _post_tick(self):
        if self.plugin_manager:
            self.plugin_manager.post_tick()

    def _on_next_frame(self):
        self._post_tick()


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
