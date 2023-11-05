from time import time
from boytacean import GameBoy

gb = GameBoy()
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")
start = time()
for _ in range(6000):
    gb.next_frame()
print(f"Time taken: {(time() - start)}")
gb.save_image("pocket.png")
