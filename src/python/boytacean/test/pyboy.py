import unittest

from io import BytesIO
from os.path import dirname, realpath, join

from boytacean.pyboy import (
    GameWrapper,
    GameWrapperKirbyDreamLand,
    GameWrapperSuperMarioLand,
    GameWrapperTetris,
    PyBoy,
    PyBoyV1,
    PyBoyV2,
    Sprite,
    Tile,
    TileMap,
    WindowEvent,
)

CURRENT_DIR = dirname(realpath(__file__))
POCKET_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/demo/pocket.gb")
ACID2_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/test/dmg_acid2.gb")


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


class TileTest(unittest.TestCase):

    def test_tile(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            tile = pb.get_tile(0)
            self.assertIsInstance(tile, Tile)
            self.assertEqual(tile.tile_identifier, 0)
            self.assertEqual(tile.shape, (8, 8))
            self.assertEqual(tile.data_address, 0x8000)
            self.assertEqual(tile.image().size, (8, 8))
            self.assertEqual(tile.image().mode, "RGBA")
            self.assertEqual(tile.ndarray().shape, (8, 8, 4))

    def test_tile_equality(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            self.assertEqual(pb.get_tile(0), pb.get_tile(0))
            self.assertNotEqual(pb.get_tile(0), pb.get_tile(1))

    def test_tile_out_of_range(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(ValueError):
                pb.get_tile(9999)


class SpriteTest(unittest.TestCase):

    def test_sprite(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            sprite = pb.get_sprite(0)
            self.assertIsInstance(sprite, Sprite)
            self.assertEqual(sprite._sprite_index, 0)
            self.assertIsInstance(sprite.x, int)
            self.assertIsInstance(sprite.y, int)
            self.assertIsInstance(sprite.tile_identifier, int)
            self.assertIn(sprite.shape, ((8, 8), (8, 16)))

    def test_sprite_by_tile_identifier(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            sprite = pb.get_sprite(0)
            results = pb.get_sprite_by_tile_identifier([sprite.tile_identifier, 999])
            self.assertEqual(len(results), 2)
            self.assertIn(0, results[0])
            self.assertEqual(results[1], [])

    def test_sprite_out_of_range(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(ValueError):
                pb.get_sprite(40)


class TileMapTest(unittest.TestCase):

    def test_tilemap_attributes(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            self.assertEqual(pb.tilemap_background.shape, (32, 32))
            self.assertEqual(pb.tilemap_window.shape, (32, 32))

    def test_tilemap_indexing(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            tm = pb.tilemap_background
            self.assertIsInstance(tm[0, 0], int)
            self.assertEqual(len(tm[0:4, 0]), 4)
            self.assertEqual(len(tm[0, 0:4]), 4)
            grid = tm[0:3, 0:3]
            self.assertEqual(len(grid), 3)
            self.assertEqual(len(grid[0]), 3)

    def test_tilemap_tile_objects(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            tm = pb.tilemap_background
            tm.use_tile_objects(True)
            self.assertIsInstance(tm[0, 0], Tile)
            tm.use_tile_objects(False)
            self.assertIsInstance(tm[0, 0], int)

    def test_tilemap_search(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(80)
            tm = pb.tilemap_background
            results = tm.search_for_identifiers([0, 1])
            self.assertEqual(len(results), 2)
            self.assertGreaterEqual(len(results[0]) + len(results[1]), 0)

    def test_tilemap_invalid_select(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(KeyError):
                TileMap(pb, "INVALID")


class GameWrapperTest(unittest.TestCase):

    def test_generic_wrapper_auto_selected(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            self.assertIsInstance(pb.game_wrapper, GameWrapper)
            self.assertEqual(pb.game_wrapper.shape, (32, 32))

    def test_start_reset_round_trip(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(60)
            pb.game_wrapper.start_game()
            pre = pb.mb.cpu.registers.PC
            pb.tick(30)
            self.assertNotEqual(pb.mb.cpu.registers.PC, pre)
            pb.game_wrapper.reset_game()
            self.assertEqual(pb.mb.cpu.registers.PC, pre)

    def test_game_area_dimensions(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(60)
            pb.game_area_dimensions(0, 0, 8, 4, follow_scrolling=False)
            area = pb.game_area()
            self.assertEqual(area.shape, (4, 8))

    def test_game_area_mapping_validation(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(ValueError):
                pb.game_area_mapping([0] * 10)
            pb.game_area_mapping(None, sprite_offset=42)
            self.assertEqual(pb.game_wrapper.sprite_offset, 42)

    def test_specific_wrapper_titles(self):
        self.assertEqual(GameWrapperTetris.cartridge_title, "TETRIS")
        self.assertEqual(GameWrapperTetris.shape, (10, 18))
        self.assertEqual(GameWrapperSuperMarioLand.cartridge_title, "SUPER MARIOLAND")
        self.assertEqual(GameWrapperSuperMarioLand.shape, (20, 16))
        self.assertEqual(GameWrapperKirbyDreamLand.cartridge_title, "KIRBY DREAM LAN")
        self.assertEqual(GameWrapperKirbyDreamLand.shape, (20, 16))
