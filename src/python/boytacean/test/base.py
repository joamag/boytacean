import unittest

from os.path import dirname, exists, realpath, join

from boytacean import GameBoy

CURRENT_DIR = dirname(realpath(__file__))
POCKET_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/demo/pocket.gb")


class BaseTest(unittest.TestCase):

    @unittest.skipUnless(
        exists(POCKET_ROM_PATH),
        f"pocket.gb not present at {POCKET_ROM_PATH}; skipping ROM-dependent test",
    )
    def test_pocket(self):
        gb = GameBoy(apu_enabled=False, serial_enabled=False, load_graphics=False)
        gb.load_rom(POCKET_ROM_PATH)
        for _ in range(600):
            gb.next_frame()

        self.assertEqual(gb.rom_title, "POCKET-DEMO")
        self.assertEqual(gb.boot_rom_s, "DMG Bootix")
        self.assertEqual(gb.clock_freq_s, "4.19 Mhz")
