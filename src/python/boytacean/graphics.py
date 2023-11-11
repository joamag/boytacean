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
from sdl2.ext import Window, Renderer, Color, init as init_sdl, get_events

from .boytacean import DISPLAY_WIDTH, DISPLAY_HEIGHT


class Display:
    _width: int = DISPLAY_WIDTH
    _height: int = DISPLAY_HEIGHT
    _title: str = "Boytacean"
    _window: Union[Window, None] = None
    _renderer: Union[Renderer, None] = None

    def __init__(
        self,
        width: int = DISPLAY_WIDTH,
        height: int = DISPLAY_HEIGHT,
        title="Boytacean",
    ):
        self._width = width
        self._height = height
        self._title = title
        self._window = None
        self._renderer = None
        self.build()

    def build(self):
        init_sdl()
        self._window = Window(self._title, size=(self._width, self._height))
        self._window.show()
        self._renderer = Renderer(self._window)

    def render_frame(self, frame_buffer: bytes):
        if not self._window:
            raise RuntimeError("Window not initialized")

        if not self._renderer:
            raise RuntimeError("Renderer not initialized")

        # we consider that every time there's a request for a new
        # frame draw the queue of SDL events should be flushed
        events = get_events()
        for event in events:
            pass

        surface = SDL_CreateRGBSurfaceFrom(
            frame_buffer,
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
            24,
            DISPLAY_WIDTH * 3,
            0x000000ff,
            0x0000ff00,
            0x00ff0000,
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
                SDL_Rect(0, 0, DISPLAY_WIDTH, DISPLAY_HEIGHT),
            )
            self._renderer.present()
        finally:
            SDL_DestroyTexture(texture)
            SDL_FreeSurface(surface)
