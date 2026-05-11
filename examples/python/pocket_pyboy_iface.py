#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
Frame-rate benchmark using boytacean's PyBoy 2.x drop-in surface.

Drives the bundled pocket demo through `boytacean.pyboy.PyBoy`, the
modern 2.x compatibility shim served by boytacean, using the batched
`tick(FRAME_COUNT, render=False)` fast path. Acts as a regression
check that the PyBoy 2.x interface produces sensible throughput
numbers, and that the SDL window swap (`LOAD_GRAPHICS=1`) doesn't
silently break it.

Run from the project root with:
    python examples/python/pocket_pyboy_iface.py

Requires: `pip install pillow pysdl2`
"""

from os import getenv
from time import time
from boytacean import VISUAL_FREQ
from boytacean.pyboy import PyBoy
from os.path import dirname, realpath, join, splitext, basename

CURRENT_DIR = dirname(realpath(__file__))
ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
BOOT_ROM_PATH = join(CURRENT_DIR, "../../res/boot/dmg_pyboy.bin")
ROM_NAME = splitext(basename(ROM_PATH))[0]
IMAGE_NAME = f"{ROM_NAME}_pyboy_iface.png"

FRAME_COUNT = 12000
LOAD_GRAPHICS = bool(getenv("LOAD_GRAPHICS", True))

with PyBoy(
    ROM_PATH,
    bootrom=BOOT_ROM_PATH,
    window="SDL2" if LOAD_GRAPHICS else "headless",
) as pyboy:
    pyboy.set_emulation_speed(0)
    print(pyboy.cartridge_title)
    start = time()
    pyboy.tick(FRAME_COUNT, render=False)
    total = time() - start
    print(f"Time taken: {total:.2f} seconds")
    print(f"Speedup: {FRAME_COUNT / total / VISUAL_FREQ:.2f}x")
    image = pyboy.screen.image
    if image:
        image.save(IMAGE_NAME)
