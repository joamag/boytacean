from .gb import GameBoyMode, GameBoy
from .palettes import PALETTES
from .video import VideoCapture

from .boytacean import (
    __version__,
    COMPILATION_DATE,
    COMPILATION_TIME,
    COMPILER,
    COMPILER_VERSION,
    NAME,
    VERSION,
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    CPU_FREQ,
    VISUAL_FREQ,
    LCD_CYCLES,
    GameBoy as GameBoyRust,
)
