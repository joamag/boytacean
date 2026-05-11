#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
Frame-rate benchmark using upstream PyBoy 1.x, for side-by-side comparison.

Mirrors `pocket.py` but against the upstream `pyboy` 1.x package
instead of boytacean, so the two `Speedup:` numbers can be compared
on the same ROM and frame count. Uses the 1.x API surface
(`disable_renderer`, `window_type`, per-frame `tick()`,
`cartridge_title()` and `screen_image()`), so a Python 3.11 install
with `pyboy==1.6.14` is required. Saves a separate
`pocket_pyboy_v1.png` snapshot of the final frame.

Run from the project root with:
    python examples/python/pocket_pyboy_v1.py [rom.gb]

Requires: `pip install "pyboy<2" "numpy<2"
"""

from os import getenv
from sys import argv
from time import time
from pyboy import PyBoy
from os.path import dirname, realpath, join, splitext, basename

CURRENT_DIR = dirname(realpath(__file__))
DEFAULT_ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
ROM_PATH = argv[1] if len(argv) > 1 else DEFAULT_ROM_PATH
ROM_NAME = splitext(basename(ROM_PATH))[0]
IMAGE_NAME = f"{ROM_NAME}_pyboy_v1.png"

FRAME_COUNT = 12000
VISUAL_FREQ = 59.7275
LOAD_GRAPHICS = bool(getenv("LOAD_GRAPHICS", True))

with PyBoy(
    ROM_PATH,
    disable_renderer=not LOAD_GRAPHICS,
    window_type="sdl2" if LOAD_GRAPHICS else "headless",
) as pyboy:
    pyboy.set_emulation_speed(0)
    print(pyboy.cartridge_title())
    start = time()
    for _ in range(FRAME_COUNT):
        pyboy.tick()
    total = time() - start
    print(f"Time taken: {total:.2f} seconds")
    print(f"Speedup: {FRAME_COUNT / total / VISUAL_FREQ:.2f}x")
    image = pyboy.screen_image()
    if image:
        image.save(IMAGE_NAME)
