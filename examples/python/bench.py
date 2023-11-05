from time import time
from boytacean import GameBoy, CPU_FREQ

CLOCK_COUNT = 100000000

gb = GameBoy(apu_enabled=False, serial_enabled=False)
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")
start = time()
cycles = gb.clocks(CLOCK_COUNT)
total = time() - start
print(f"Time taken: {total:.2f} seconds")
print(f"Speedup: {cycles / (CPU_FREQ * total):.2f}x")
gb.save_image("pocket.png")
