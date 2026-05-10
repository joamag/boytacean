from math import ceil
from time import perf_counter
from typing import Union
from sdl2 import (
    SDL_QUIT,
    SDL_CreateRGBSurfaceFrom,
    SDL_CreateTextureFromSurface,
    SDL_RenderCopy,
    SDL_Rect,
    SDL_DestroyTexture,
    SDL_FreeSurface,
)
from sdl2.ext import Window, Renderer, init as init_sdl, get_events

from .boytacean import DISPLAY_WIDTH, DISPLAY_HEIGHT, VISUAL_FREQ


class Display:
    _width: int = DISPLAY_WIDTH
    _height: int = DISPLAY_HEIGHT
    _title: str = "Boytacean"
    _scale: float = 3.0
    _frame_gap: int = 60
    _next_frame: int = 60
    _window: Union[Window, None] = None
    _renderer: Union[Renderer, None] = None
    _hud_enabled: bool = False
    _hud_base_title: str = "Boytacean"
    _hud_clock_freq: int = 0
    _hud_last_t: float = 0.0
    _hud_last_frame: int = 0
    _hud_interval: float = 1.0

    def __init__(
        self,
        width: int = DISPLAY_WIDTH,
        height: int = DISPLAY_HEIGHT,
        title="Boytacean",
        scale=3.0,
        start_frame=0,
        fps=5,
    ):
        self._width = width
        self._height = height
        self._title = title
        self._scale = scale
        self._frame_gap = ceil(VISUAL_FREQ / fps)
        self._next_frame = start_frame + self._frame_gap
        self._window = None
        self._renderer = None
        self._hud_enabled = False
        self._hud_base_title = title
        self._hud_clock_freq = 0
        self._hud_last_t = 0.0
        self._hud_last_frame = 0
        self.build()

    def build(self):
        init_sdl()
        window = Window(
            self._title,
            size=(int(self._width * self._scale), int(self._height * self._scale)),
        )
        window.show()
        self._window = window
        self._renderer = Renderer(window)

    def should_render(self, frame_index) -> bool:
        return frame_index >= self._next_frame

    def set_title(self, title: str):
        self._title = title
        if self._window is not None:
            self._window.title = title

    def set_hud(
        self,
        enabled: bool,
        base_title: Union[str, None] = None,
        clock_freq: Union[int, None] = None,
        interval: float = 1.0,
    ):
        # turns the live FPS / clock-frequency HUD on or off; when
        # enabled the window title is rewritten at most once every
        # `interval` seconds from inside `render_frame`
        self._hud_enabled = enabled
        if base_title is not None:
            self._hud_base_title = base_title
            self.set_title(base_title)
        if clock_freq is not None:
            self._hud_clock_freq = clock_freq
        self._hud_interval = interval
        self._hud_last_t = perf_counter()
        self._hud_last_frame = 0

    def render_frame(self, frame_buffer: bytes, frame_index: int = 0):
        if self._window is None:
            raise RuntimeError("Window not initialized")

        if self._renderer is None:
            raise RuntimeError("Renderer not initialized")

        # we consider that every time there's a request for a new
        # frame draw the queue of SDL events should be flushed
        events = get_events()
        for _ in events:
            pass

        surface = SDL_CreateRGBSurfaceFrom(
            frame_buffer,
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
            24,
            DISPLAY_WIDTH * 3,
            0x000000FF,
            0x0000FF00,
            0x00FF0000,
            0x0,
        )
        texture = SDL_CreateTextureFromSurface(
            self._renderer.sdlrenderer, surface.contents
        )

        try:
            SDL_RenderCopy(
                self._renderer.sdlrenderer,
                texture,
                None,
                None,
            )
            self._renderer.present()
        finally:
            SDL_DestroyTexture(texture)
            SDL_FreeSurface(surface)

        self._next_frame += self._frame_gap

        self._refresh_hud(frame_index)

    def _refresh_hud(self, frame_index: int):
        if not self._hud_enabled or self._window is None:
            return
        now = perf_counter()
        elapsed = now - self._hud_last_t
        if elapsed < self._hud_interval:
            return
        frames = max(frame_index - self._hud_last_frame, 0)
        fps = frames / elapsed if elapsed > 0 else 0.0
        title = (
            f"{self._hud_base_title}  -  {fps:.0f} fps  -  "
            f"{self._hud_clock_freq / 1e6:.2f} MHz  -  "
            f"{fps / VISUAL_FREQ:.1f}x"
        )
        self._window.title = title
        self._hud_last_t = now
        self._hud_last_frame = frame_index
