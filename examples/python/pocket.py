#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
Frame-rate benchmark using the native boytacean Python surface.

Boots the bundled pocket demo with audio and serial disabled, runs
12000 frames via `gb.next_frame()` and reports the realised speedup
versus the Game Boy's nominal visual frequency. At the end, saves a
PNG snapshot of the final frame. Set `LOAD_GRAPHICS=0` to skip
graphics work for a headless-style measurement.

Run from the project root with:
    python examples/python/pocket.py

Requires: `pip install pillow pysdl2`
"""

from os import getenv
from time import time
from boytacean import GameBoy, VISUAL_FREQ
from os.path import dirname, realpath, join, splitext, basename

CURRENT_DIR = dirname(realpath(__file__))
ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
ROM_NAME = splitext(basename(ROM_PATH))[0]
IMAGE_NAME = f"{ROM_NAME}.png"

FRAME_COUNT = 12000
LOAD_GRAPHICS = bool(getenv("LOAD_GRAPHICS", True))

gb = GameBoy(apu_enabled=False, serial_enabled=False, load_graphics=LOAD_GRAPHICS)
gb.load_rom(ROM_PATH)
start = time()
for _ in range(FRAME_COUNT):
    gb.next_frame()
total = time() - start
print(f"Time taken: {total:.2f} seconds")
print(f"Speedup: {FRAME_COUNT / total / VISUAL_FREQ:.2f}x")
gb.save_image(IMAGE_NAME)
