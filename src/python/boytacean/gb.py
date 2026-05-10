from enum import Enum
from contextlib import contextmanager
from typing import Any, Iterable, Union, cast

try:
    from PIL.Image import Image, frombytes
except ImportError:
    Image = Any
    frombytes = Any

from .palettes import PALETTES
from .video import VideoCapture

from .boytacean import DISPLAY_WIDTH, DISPLAY_HEIGHT, GameBoy as GameBoyRust


class GameBoyMode(Enum):
    DMG = 1
    CGB = 2
    SGB = 3


class PadKey(Enum):
    Up = 1
    Down = 2
    Left = 3
    Right = 4
    Start = 5
    Select = 6
    A = 7
    B = 8


class GameBoy:
    _frame_index: int = 0
    _video: Union[VideoCapture, None] = None
    _display: Union[Any, None] = None

    def __init__(
        self,
        mode=GameBoyMode.DMG,
        ppu_enabled=True,
        apu_enabled=True,
        dma_enabled=True,
        timer_enabled=True,
        serial_enabled=True,
        load_graphics=False,
        load=True,
        boot=True,
    ):
        super().__init__()
        self._mode = mode
        self._frame_index = 0
        self._video = None
        self._display = None
        self._system = GameBoyRust(mode.value)
        self._system.set_ppu_enabled(ppu_enabled)
        self._system.set_apu_enabled(apu_enabled)
        self._system.set_dma_enabled(dma_enabled)
        self._system.set_timer_enabled(timer_enabled)
        self._system.set_serial_enabled(serial_enabled)
        if load_graphics:
            self.load_graphics()
        if load:
            self.load(boot=boot)

    def _repr_markdown_(self) -> str:
        return f"""
# Boytacean
This is a [Game Boy](https://en.wikipedia.org/wiki/Game_Boy) emulator built using the [Rust Programming Language](https://www.rust-lang.org) and is running inside this browser with the help of [WebAssembly](https://webassembly.org).

| Field      | Value               |
| ---------- | ------------------- |
| Version    | {self.version}      |
| Boot ROM   | {self.boot_rom_s}   |
| Clock      | {self.clock_freq_s} |
"""

    def boot(self):
        self._system.boot()

    def load(self, boot=True):
        self._system.load(boot)

    def load_boot(self, path: str):
        self._system.load_boot_path(path)

    def load_boot_data(self, data: bytes):
        self._system.load_boot(data)

    def load_rom(self, path: str):
        self._system.load_rom_file(path)

    def load_rom_data(self, data: bytes):
        self._system.load_rom(data)

    def read_memory(self, addr: int) -> int:
        return self._system.read_memory(addr)

    def write_memory(self, addr: int, value: int):
        self._system.write_memory(addr, value)

    def clock(self) -> int:
        return self._system.clock()

    def clock_many(self, count: int) -> int:
        return self._system.clock_many(count)

    def clock_step(self, addr: int) -> int:
        return self._system.clock_step(addr)

    def clocks(self, count: int) -> int:
        return self._system.clocks(count)

    def clocks_cycles(self, limit: int) -> int:
        return self._system.clocks_cycles(limit)

    def next_frame(self) -> int:
        cycles = self._system.next_frame()
        self._frame_index += 1
        self._on_next_frame()
        return cycles

    def next_frames(self, count: int) -> int:
        # in-Rust batched frame loop, used by callers that don't need a
        # per-frame Python callback (PyBoyV2.tick takes this fast path
        # whenever no hooks or gameshark cheats are active)
        cycles = self._system.next_frames(count)
        self._frame_index += count
        if self._video is not None or self._display is not None:
            self._on_next_frame()
        return cycles

    def step_to(self, addr: int) -> int:
        return self._system.step_to(addr)

    def skip_frames(self, count: int) -> int:
        cycles = 0
        for _ in range(count):
            cycles += self.next_frame()
        return cycles

    def key_press(self, key: PadKey):
        self._system.key_press(key.value)

    def key_lift(self, key: PadKey):
        self._system.key_lift(key.value)

    def frame_buffer(self) -> bytes:
        return self._system.frame_buffer()

    def frame_buffer_rgba(self) -> bytes:
        return self._system.frame_buffer_rgba()

    def vram(self) -> bytes:
        return self._system.vram()

    def oam(self) -> bytes:
        return self._system.oam()

    def hram(self) -> bytes:
        return self._system.hram()

    def rom_data(self) -> bytes:
        return self._system.rom_data()

    def ram_data(self) -> bytes:
        return self._system.ram_data()

    def image(self) -> Image:
        frame_buffer = cast(bytes, self._system.frame_buffer())
        image = frombytes("RGB", (DISPLAY_WIDTH, DISPLAY_HEIGHT), frame_buffer, "raw")
        return image

    def save_image(self, path: str, format: str = "png"):
        image = self.image()
        image.save(path, format=format)

    def video(
        self,
        save=True,
        display=False,
    ) -> Any:
        from IPython.display import display as _display

        if self._video is None:
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

    def load_graphics(self):
        from .graphics import Display

        self._display = Display()

    def save_state(self) -> bytes:
        return self._system.save_state()

    def load_state(self, data: bytes):
        self._system.load_state(data)

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
    def ppu_ly(self) -> int:
        return self._system.ppu_ly()

    @property
    def ppu_frame(self) -> int:
        return self._system.ppu_frame()

    @property
    def cpu_pc(self) -> int:
        return self._system.cpu_pc()

    @cpu_pc.setter
    def cpu_pc(self, value: int):
        self._system.set_cpu_pc(value)

    @property
    def cpu_sp(self) -> int:
        return self._system.cpu_sp()

    @cpu_sp.setter
    def cpu_sp(self, value: int):
        self._system.set_cpu_sp(value)

    @property
    def cpu_a(self) -> int:
        return self._system.cpu_a()

    @cpu_a.setter
    def cpu_a(self, value: int):
        self._system.set_cpu_a(value)

    @property
    def cpu_b(self) -> int:
        return self._system.cpu_b()

    @cpu_b.setter
    def cpu_b(self, value: int):
        self._system.set_cpu_b(value)

    @property
    def cpu_c(self) -> int:
        return self._system.cpu_c()

    @cpu_c.setter
    def cpu_c(self, value: int):
        self._system.set_cpu_c(value)

    @property
    def cpu_d(self) -> int:
        return self._system.cpu_d()

    @cpu_d.setter
    def cpu_d(self, value: int):
        self._system.set_cpu_d(value)

    @property
    def cpu_e(self) -> int:
        return self._system.cpu_e()

    @cpu_e.setter
    def cpu_e(self, value: int):
        self._system.set_cpu_e(value)

    @property
    def cpu_h(self) -> int:
        return self._system.cpu_h()

    @cpu_h.setter
    def cpu_h(self, value: int):
        self._system.set_cpu_h(value)

    @property
    def cpu_l(self) -> int:
        return self._system.cpu_l()

    @cpu_l.setter
    def cpu_l(self, value: int):
        self._system.set_cpu_l(value)

    @property
    def cgb(self) -> bool:
        return self._system.cgb()

    @property
    def dmg(self) -> bool:
        return self._system.dmg()

    @property
    def sgb(self) -> bool:
        return self._system.sgb()

    @property
    def rom_title(self) -> str:
        return self._system.rom_title()

    @property
    def rom_bank(self) -> int:
        return self._system.rom_bank()

    @property
    def ram_bank(self) -> int:
        return self._system.ram_bank()

    @property
    def rom_banks(self) -> int:
        return self._system.rom_banks()

    @property
    def ram_banks(self) -> int:
        return self._system.ram_banks()

    @property
    def ram_enabled(self) -> bool:
        return self._system.ram_enabled()

    @property
    def has_battery(self) -> bool:
        return self._system.has_battery()

    @property
    def checksum(self) -> int:
        return self._system.checksum()

    @property
    def version(self) -> str:
        return self._system.version()

    @property
    def clock_freq(self) -> int:
        return self._system.clock_freq()

    @clock_freq.setter
    def clock_freq(self, value: int):
        self._system.set_clock_freq(value)

    @property
    def clock_freq_s(self) -> str:
        return self._system.clock_freq_s()

    @property
    def boot_rom_s(self) -> str:
        return self._system.boot_rom_s()

    @property
    def rom_type_s(self) -> str:
        return self._system.rom_type_s()

    @property
    def rom_size_s(self) -> str:
        return self._system.rom_size_s()

    @property
    def ram_size_s(self) -> str:
        return self._system.ram_size_s()

    @property
    def timer_div(self) -> int:
        return self._system.timer_div()

    @timer_div.setter
    def timer_div(self, value: int):
        self._system.set_timer_div(value)

    @property
    def frame_count(self) -> int:
        return self._frame_index

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
        if self._video is not None and self._video.should_capture(self._frame_index):
            self._video.save_frame(self.image(), self._frame_index)
            self._video.compute_next(self._frame_index)

        if self._display is not None and self._display.should_render(self._frame_index):
            from .graphics import Display

            cast(Display, self._display).render_frame(self.frame_buffer())

    def _start_capture(
        self,
        video_format="avc1",
        video_extension="mp4",
        video_name="output",
        fps=5,
        frame_format="png",
    ):
        if self._video is not None:
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
