from time import time
from pyboy import PyBoy
from os.path import dirname, realpath, join, splitext, basename

CURRENT_DIR = dirname(realpath(__file__))
ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
ROM_NAME = splitext(basename(ROM_PATH))[0]
IMAGE_NAME = f"{ROM_NAME}_pyboy.png"

FRAME_COUNT = 12000
VISUAL_FREQ = 59.7275

with PyBoy(ROM_PATH, disable_renderer=True) as pyboy:
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
