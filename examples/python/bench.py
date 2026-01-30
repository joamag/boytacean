from time import time
from boytacean import GameBoy, CPU_FREQ
from os.path import dirname, realpath, join

CURRENT_DIR = dirname(realpath(__file__))
ROM_PATH = join(CURRENT_DIR, "../../res/roms/demo/pocket.gb")

CLOCK_COUNT = 100000000

gb = GameBoy(apu_enabled=False, serial_enabled=False)
gb.load_rom(ROM_PATH)
start = time()
cycles = gb.clocks(CLOCK_COUNT)
total = time() - start
print(f"Time taken: {total:.2f} seconds")
print(f"Speedup: {cycles / (CPU_FREQ * total):.2f}x")
