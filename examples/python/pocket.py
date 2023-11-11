from time import time
from boytacean import GameBoy, VISUAL_FREQ
from os.path import dirname, realpath, join

CURRENT_DIR = dirname(realpath(__file__))
ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")
IMAGE_NAME = "pocket.png"

FRAME_COUNT = 12000
LOAD_GRAPHICS = False

gb = GameBoy(apu_enabled=False, serial_enabled=False, load_graphics=LOAD_GRAPHICS)
gb.load_rom(ROM_PATH)
start = time()
for _ in range(FRAME_COUNT):
    gb.next_frame()
total = time() - start
print(f"Time taken: {total:.2f} seconds")
print(f"Speedup: {FRAME_COUNT / total / VISUAL_FREQ:.2f}x")
gb.save_image(IMAGE_NAME)
