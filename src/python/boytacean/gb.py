from enum import Enum
from glob import glob
from math import ceil
from shutil import rmtree
from tempfile import mkdtemp
from contextlib import contextmanager
from typing import Any, Union

from PIL.Image import Image, frombytes

from .palettes import PALETTES

from .boytacean import (
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    CPU_FREQ,
    VISUAL_FREQ,
    LCD_CYCLES,
    GameBoy as GameBoyRust,
)


class GameBoyMode(Enum):
    DMG = 1
    CGB = 2
    SGB = 3


class GameBoy:
    _frame_index: int = 0
    _start_frame: Union[int, None]
    _frame_gap: int
    _capture_temp_dir: Union[str, None]

    def __init__(
        self,
        mode=GameBoyMode.DMG,
        ppu_enabled=True,
        apu_enabled=True,
        dma_enabled=True,
        timer_enabled=True,
        serial_enabled=True,
        load=True,
    ):
        super().__init__()
        self._frame_index = 0
        self._next_frame = None
        self._frame_gap = VISUAL_FREQ
        self._capture_temp_dir = None
        self._system = GameBoyRust(mode.value)
        self._system.set_ppu_enabled(ppu_enabled)
        self._system.set_apu_enabled(apu_enabled)
        self._system.set_dma_enabled(dma_enabled)
        self._system.set_timer_enabled(timer_enabled)
        self._system.set_serial_enabled(serial_enabled)
        if load:
            self.load()

    def _repr_markdown_(self) -> str:
        return f"""
# Boytacean
This is a [Game Boy](https://en.wikipedia.org/wiki/Game_Boy) emulator built using the [Rust Programming Language](https://www.rust-lang.org) and is running inside this browser with the help of [WebAssembly](https://webassembly.org).

| Field   | Value               |
| ------- | ------------------- |
| Version | {self.version}      |
| Clock   | {self.clock_freq_s} |
"""

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
        cycles = self._system.next_frame()
        self._frame_index += 1
        self._on_next_frame()
        return cycles

    def frame_buffer(self):
        return self._system.frame_buffer()

    def image(self) -> Image:
        frame_buffer = self._system.frame_buffer()
        image = frombytes("RGB", (DISPLAY_WIDTH, DISPLAY_HEIGHT), frame_buffer, "raw")
        return image

    def save_image(self, filename: str, format: str = "PNG"):
        image = self.image()
        image.save(filename, format=format)

    def video(self, encoder="avc1", display=True) -> Any:
        from cv2 import VideoWriter, VideoWriter_fourcc, imread
        from IPython.display import Video, display as _display

        image_paths = glob(f"{self._capture_temp_dir}/*.png")
        video_path = f"{self._capture_temp_dir}/video.mp4"

        encoder = VideoWriter(
            video_path,
            VideoWriter_fourcc(*encoder),
            VISUAL_FREQ / self._frame_gap,
            (DISPLAY_WIDTH, DISPLAY_HEIGHT),
        )

        try:
            for image_file in sorted(image_paths):
                img = imread(image_file)
                encoder.write(img)
        finally:
            encoder.release()

        video = Video(video_path, embed=True, html_attributes="controls loop autoplay")

        if display:
            _display(video)

        return video

    def set_palette(self, name: str):
        if not name in PALETTES:
            raise ValueError(f"Unknown palette: {name}")
        palette = PALETTES[name]
        self.set_palette_colors(palette)

    def set_palette_colors(self, colors_hex: str):
        self._system.set_palette_colors(colors_hex)

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

    @property
    def version(self) -> str:
        return self._system.version()

    @property
    def clock_freq_s(self) -> str:
        return self._system.clock_freq_s()

    @contextmanager
    def video_capture(self, fps=5):
        self._start_capture(fps=fps)
        try:
            yield
        finally:
            self.video()
            self._stop_capture()

    def _on_next_frame(self):
        if self._next_frame != None and self._frame_index >= self._next_frame:
            self._next_frame = self._next_frame + self._frame_gap
            self.save_image(
                f"{self._capture_temp_dir}/frame_{self._frame_index:08d}.png"
            )

    def _start_capture(self, fps=5):
        self._next_frame = self._frame_index + self._frame_gap
        self._frame_gap = ceil(VISUAL_FREQ / fps)
        self._capture_temp_dir = mkdtemp()

    def _stop_capture(self):
        self._next_frame = None
        if self._capture_temp_dir:
            rmtree(self._capture_temp_dir)
