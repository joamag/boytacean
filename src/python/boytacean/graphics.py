from math import ceil
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
        self.build()

    def build(self):
        init_sdl()
        self._window = Window(
            self._title,
            size=(int(self._width * self._scale), int(self._height * self._scale)),
        )
        self._window.show()
        self._renderer = Renderer(self._window)

    def should_render(self, frame_index) -> bool:
        return frame_index >= self._next_frame

    def render_frame(self, frame_buffer: bytes):
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
