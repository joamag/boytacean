__version__: str
COMPILATION_DATE: str
COMPILATION_TIME: str
COMPILER: str
COMPILER_VERSION: str
NAME: str
VERSION: str
DISPLAY_WIDTH: int
DISPLAY_HEIGHT: int
CPU_FREQ: int
VISUAL_FREQ: float
LCD_CYCLES: int

class GameBoy:
    def __init__(self, mode: int): ...
    def reset(self): ...
    def boot(self): ...
    def load(self, boot: bool): ...
    def load_boot(self, data: bytes): ...
    def load_boot_path(self, path: str): ...
    def load_rom(self, data: bytes): ...
    def load_rom_file(self, path: str): ...
    def read_memory(self, addr: int) -> int: ...
    def write_memory(self, addr: int, value: int): ...
    def clock(self) -> int: ...
    def clock_many(self, count: int) -> int: ...
    def clock_step(self, addr: int) -> int: ...
    def clocks(self, count: int) -> int: ...
    def clocks_cycles(self, limit: int) -> int: ...
    def next_frame(self) -> int: ...
    def step_to(self, addr: int) -> int: ...
    def key_press(self, key: int): ...
    def key_lift(self, key: int): ...
    def frame_buffer(self) -> bytes: ...
    def set_palette_colors(self, colors_hex: str): ...
    def ppu_enabled(self) -> bool: ...
    def set_ppu_enabled(self, value: bool): ...
    def apu_enabled(self) -> bool: ...
    def set_apu_enabled(self, value: bool): ...
    def dma_enabled(self) -> bool: ...
    def set_dma_enabled(self, value: bool): ...
    def timer_enabled(self) -> bool: ...
    def set_timer_enabled(self, value: bool): ...
    def serial_enabled(self) -> bool: ...
    def set_serial_enabled(self, value: bool): ...
    def rom_title(self) -> str: ...
    def version(self) -> str: ...
    def clock_freq_s(self) -> str: ...
    def boot_rom_s(self) -> str: ...
    def timer_div(self) -> int: ...
    def set_timer_div(self, value: int): ...
    def save_state(self) -> bytes: ...
    def load_state(self, data: bytes): ...
