from PIL import Image

from .boytacean import DISPLAY_WIDTH, DISPLAY_HEIGHT, GameBoy as GameBoyRust


class GameBoy:
    def __init__(self, apu_enabled=True):
        super().__init__()
        self._system = GameBoyRust()
        self._system.set_apu_enabled(apu_enabled)

    def load(self):
        self._system.load()

    def load_rom(self, filename: str):
        self._system.load_rom(filename)

    def clock(self) -> int:
        return self._system.clock()

    def clock_m(self, count: int) -> int:
        return self._system.clock_m(count)

    def clocks(self, count: int) -> int:
        return self._system.clocks(count)

    def next_frame(self) -> int:
        return self._system.next_frame()

    def frame_buffer(self):
        return self._system.frame_buffer()

    def save_image(self, filename: str, format: str = "PNG"):
        frame_buffer = self._system.frame_buffer()
        image = Image.frombytes(
            "RGB", (DISPLAY_WIDTH, DISPLAY_HEIGHT), frame_buffer, "raw"
        )
        image.save(filename, format=format)

    @property
    def apu_enabled(self) -> bool:
        return self._system.apu_enabled()

    def set_apu_enabled(self, value: bool):
        self._system.set_apu_enabled(value)
