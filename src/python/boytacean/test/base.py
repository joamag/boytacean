import unittest

from os.path import dirname, realpath, join

from boytacean import GameBoy


class BaseTest(unittest.TestCase):

    def test_pocket(self):
        CURRENT_DIR = dirname(realpath(__file__))
        ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/demo/pocket.gb")
        FRAME_COUNT = 600
        LOAD_GRAPHICS = False

        gb = GameBoy(
            apu_enabled=False, serial_enabled=False, load_graphics=LOAD_GRAPHICS
        )
        gb.load_rom(ROM_PATH)
        for _ in range(FRAME_COUNT):
            gb.next_frame()
