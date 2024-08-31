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
    bootrom_file=BOOT_ROM_PATH,
    disable_renderer=not LOAD_GRAPHICS,
    plugin_manager=False,
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
