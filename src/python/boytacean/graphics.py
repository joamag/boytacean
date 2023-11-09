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
        self.build()

    def build(self):
        init_sdl()
        self._window = Window(self._title, size=(self._width, self._height))
        self._window.show()

    def render_frame(self, frame_buffer: bytes):
        if not self._window:
            raise RuntimeError("Window not initialized")

        events = get_events()
        for event in events:
            if event.type == SDL_QUIT:
                running = False  # @TODO need to destroy the engine
                break

        # @TODO: this should not be build every single frame
        renderer = Renderer(self._window)

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
        texture = SDL_CreateTextureFromSurface(renderer.sdlrenderer, surface.contents)

        # @TODO: not sure this clear is required
        renderer.clear(Color(0, 0, 0))
        SDL_RenderCopy(
            renderer.sdlrenderer,
            texture,
            None,
            SDL_Rect(0, 0, DISPLAY_WIDTH, DISPLAY_HEIGHT),
        )
        renderer.present()

        # @TODO: need to destroy latter on
        # SDL_DestroyTexture(texture)
        # SDL_FreeSurface(surface)
