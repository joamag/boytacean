import unittest

from io import BytesIO
from os.path import dirname, realpath, join

from boytacean.pyboy import PyBoy, PyBoyV1, PyBoyV2, WindowEvent

CURRENT_DIR = dirname(realpath(__file__))
POCKET_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/demo/pocket.gb")


class PyBoyV2Test(unittest.TestCase):

    def test_alias(self):
        self.assertIs(PyBoy, PyBoyV2)

    def test_window_event_count(self):
        self.assertEqual(len(list(WindowEvent)), 43)
        self.assertEqual(WindowEvent.CYCLE_PALETTE.value, 42)

    def test_pocket(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            self.assertEqual(pb.cartridge_title, "POCKET-DEMO")
            self.assertFalse(pb.stopped)
            self.assertTrue(pb.tick(10))
            self.assertEqual(pb.frame_count, 10)

    def test_memory_view(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            value = pb.memory[0xFF40]
            self.assertIsInstance(value, int)
            slice_values = pb.memory[0xFF40:0xFF44]
            self.assertEqual(len(slice_values), 4)
            pb.memory[0xC000] = 0x42
            self.assertEqual(pb.memory[0xC000], 0x42)
            pb.memory[0xC000:0xC004] = 0
            self.assertEqual(pb.memory[0xC000:0xC004], [0, 0, 0, 0])

    def test_screen(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            image = pb.screen.image
            self.assertEqual(image.size, (160, 144))
            self.assertEqual(image.mode, "RGBA")
            ndarray = pb.screen.ndarray
            self.assertEqual(ndarray.shape, (144, 160, 4))
            self.assertEqual(str(ndarray.dtype), "uint8")
            self.assertEqual(pb.screen.raw_buffer_dims, (144, 160))

    def test_button(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.button("a", delay=2)
            pb.button_press("start")
            pb.button_release("start")
            with self.assertRaises(ValueError):
                pb.button("invalid")

    def test_motherboard(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            self.assertFalse(pb.mb.cgb)
            self.assertEqual(pb.mb.cartridge.gamename, "POCKET-DEMO")
            self.assertIsInstance(pb.mb.cpu.registers.PC, int)
            self.assertIsInstance(pb.mb.cpu.registers.HL, int)

    def test_save_state_roundtrip(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(20)
            pc_before = pb.mb.cpu.registers.PC
            buffer = BytesIO()
            pb.save_state(buffer)
            pb.tick(20)
            buffer.seek(0)
            pb.load_state(buffer)
            self.assertEqual(pb.mb.cpu.registers.PC, pc_before)


class PyBoyV1Test(unittest.TestCase):

    def test_pocket(self):
        with PyBoyV1(
            POCKET_ROM_PATH, disable_renderer=True, sound_emulated=False
        ) as pb:
            self.assertEqual(pb.cartridge_title(), "POCKET-DEMO")
            self.assertFalse(pb.tick())
            self.assertEqual(pb.get_memory_value(0xFF40), pb.read_memory(0xFF40))

    def test_send_input(self):
        with PyBoyV1(
            POCKET_ROM_PATH, disable_renderer=True, sound_emulated=False
        ) as pb:
            pb.send_input(WindowEvent.PRESS_BUTTON_A)
            pb.tick()
            pb.send_input(WindowEvent.RELEASE_BUTTON_A)
            pb.tick()

    def test_screen_image(self):
        with PyBoyV1(
            POCKET_ROM_PATH, disable_renderer=True, sound_emulated=False
        ) as pb:
            pb.tick()
            image = pb.screen_image()
            self.assertEqual(image.size, (160, 144))
            self.assertEqual(image.mode, "RGB")
