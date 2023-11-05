from time import time
from pyboy import PyBoy

FRAME_COUNT = 12000

with PyBoy("../../res/roms/demo/pocket.gb", disable_renderer=True) as pyboy:
    pyboy.set_emulation_speed(0)
    print(pyboy.cartridge_title())
    start = time()
    for _ in range(FRAME_COUNT):
        pyboy.tick()
    total = time() - start
    print(f"Time taken: {total} seconds")
    print(f"Speedup: {FRAME_COUNT / total / 60}x")
    pyboy.screen_image().save("pocket_pyboy.png")
