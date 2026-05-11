#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
Frame-rate benchmark using boytacean's PyBoy 1.x drop-in surface.

Counterpart to `pocket_pyboy_iface.py` but against the legacy 1.x
`PyBoyV1` class, which keeps the `pyboy.tick()` per-frame loop and
the `screen_image()` / `cartridge_title()` method-call shape. Used
to verify that scripts written for old PyBoy releases continue to
run unchanged against boytacean.

Run from the project root with:
    python examples/python/pocket_pyboy_iface_v1.py [rom.gb]

Requires: `pip install pillow pysdl2`
"""

from os import getenv
from sys import argv
from time import time
from boytacean import GameBoyMode, VISUAL_FREQ
from boytacean.boytacean import infer_mode
from boytacean.pyboy import PyBoyV1
from os.path import dirname, realpath, join, splitext, basename

CURRENT_DIR = dirname(realpath(__file__))
DEFAULT_ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
ROM_PATH = argv[1] if len(argv) > 1 else DEFAULT_ROM_PATH
IS_CGB = GameBoyMode(infer_mode(ROM_PATH)) == GameBoyMode.CGB
BOOT_ROM_PATH = join(
    CURRENT_DIR,
    f"../../res/boot/{'cgb_pyboy' if IS_CGB else 'dmg_pyboy'}.bin",
)
ROM_NAME = splitext(basename(ROM_PATH))[0]
IMAGE_NAME = f"{ROM_NAME}_pyboy_iface_v1.png"

FRAME_COUNT = 12000
LOAD_GRAPHICS = bool(getenv("LOAD_GRAPHICS", True))

with PyBoyV1(
    ROM_PATH,
    bootrom_file=BOOT_ROM_PATH,
    disable_renderer=not LOAD_GRAPHICS,
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
