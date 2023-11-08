from enum import Enum
from contextlib import contextmanager
from typing import Any, Iterable, Union

from PIL.Image import Image, frombytes

from .palettes import PALETTES
from .video import VideoCapture

from .boytacean import (
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    GameBoy as GameBoyRust,
)


class GameBoyMode(Enum):
    DMG = 1
    CGB = 2
    SGB = 3


class GameBoy:
    _frame_index: int = 0
    _video: Union[VideoCapture, None] = None

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
        self._video = None
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

    def save_image(self, filename: str, format: str = "png"):
        image = self.image()
        image.save(f"{filename}.{format.lower()}", format=format)

    def video(
        self,
        save=True,
        display=False,
    ) -> Any:
        from IPython.display import display as _display

        if self._video == None:
            raise RuntimeError("Not capturing a video")

        video = self._video.build(save=save)
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
    def rom_title(self) -> str:
        return self._system.rom_title()

    @property
    def version(self) -> str:
        return self._system.version()

    @property
    def clock_freq_s(self) -> str:
        return self._system.clock_freq_s()

    @property
    def palettes(self) -> Iterable[str]:
        return PALETTES.keys()

    @contextmanager
    def video_capture(
        self,
        video_format="avc1",
        video_extension="mp4",
        video_name="output",
        fps=5,
        frame_format="png",
        video=True,
        save=False,
        display=True,
    ):
        self._start_capture(
            video_format=video_format,
            video_extension=video_extension,
            video_name=video_name,
            fps=fps,
            frame_format=frame_format,
        )
        try:
            yield
            if video:
                self.video(save=save, display=display)
        finally:
            self._stop_capture()

    def _on_next_frame(self):
        if self._video != None and self._video.should_capture(self._frame_index):
            self._video.save_frame(self.image(), self._frame_index)
            self._video.compute_next(self._frame_index)

    def _start_capture(
        self,
        video_format="avc1",
        video_extension="mp4",
        video_name="output",
        fps=5,
        frame_format="png",
    ):
        if self._video != None:
            raise RuntimeError("Already capturing a video")
        self._video = VideoCapture(
            start_frame=self._frame_index,
            video_format=video_format,
            video_extension=video_extension,
            video_name=video_name,
            fps=fps,
            frame_format=frame_format,
        )

    def _stop_capture(self):
        if self._video:
            self._video.cleanup()
        self._video = None
