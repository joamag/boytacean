from PIL.Image import Image, frombytes

from .boytacean import DISPLAY_WIDTH, DISPLAY_HEIGHT, CPU_FREQ, GameBoy as GameBoyRust


class GameBoy:
    def __init__(
        self,
        ppu_enabled=True,
        apu_enabled=True,
        dma_enabled=True,
        timer_enabled=True,
        serial_enabled=True,
    ):
        super().__init__()
        self._system = GameBoyRust()
        self._system.set_ppu_enabled(ppu_enabled)
        self._system.set_apu_enabled(apu_enabled)
        self._system.set_dma_enabled(dma_enabled)
        self._system.set_timer_enabled(timer_enabled)
        self._system.set_serial_enabled(serial_enabled)

    def load(self):
        self._system.load()

    def load_rom(self, filename: str):
        self._system.load_rom_file(filename)

    def load_rom_data(self, data: bytes):
        self._system.load_rom(data)

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

    def image(self) -> Image:
        frame_buffer = self._system.frame_buffer()
        image = frombytes("RGB", (DISPLAY_WIDTH, DISPLAY_HEIGHT), frame_buffer, "raw")
        return image

    def save_image(self, filename: str, format: str = "PNG"):
        image = self.image()
        image.save(filename, format=format)

    @property
    def ppu_enabled(self) -> bool:
        return self._system.ppu_enabled()

    def set_ppu_enabled(self, value: bool):
        self._system.set_ppu_enabled(value)

    @property
    def apu_enabled(self) -> bool:
        return self._system.apu_enabled()

    def set_apu_enabled(self, value: bool):
        self._system.set_apu_enabled(value)

    @property
    def dma_enabled(self) -> bool:
        return self._system.dma_enabled()

    def set_dma_enabled(self, value: bool):
        self._system.set_dma_enabled(value)

    @property
    def timer_enabled(self) -> bool:
        return self._system.timer_enabled()

    def set_timer_enabled(self, value: bool):
        self._system.set_timer_enabled(value)

    @property
    def serial_enabled(self) -> bool:
        return self._system.serial_enabled()

    def set_serial_enabled(self, value: bool):
        self._system.set_serial_enabled(value)
