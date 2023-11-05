from time import time
from boytacean import GameBoy

FRAME_COUNT = 12000

gb = GameBoy(apu_enabled=False, serial_enabled=False)
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")
start = time()
for _ in range(FRAME_COUNT):
    gb.next_frame()
total = time() - start
print(f"Time taken: {total:.2f} seconds")
print(f"Speedup: {FRAME_COUNT / total / 60:.2f}x")
gb.save_image("pocket.png")
