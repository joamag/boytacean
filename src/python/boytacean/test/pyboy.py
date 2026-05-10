import os
import tempfile
import unittest

from io import BytesIO
from os.path import dirname, exists, realpath, join

from boytacean.pyboy import (
    BotSupportManager,
    DynamicComparisonType,
    GameWrapper,
    GameWrapperKirbyDreamLand,
    GameWrapperSuperMarioLand,
    GameWrapperTetris,
    LegacyScreen,
    PyBoy,
    PyBoyV1,
    PyBoyV2,
    ScanMode,
    Sprite,
    StandardComparisonType,
    Tile,
    TileMap,
    WindowEvent,
)

CURRENT_DIR = dirname(realpath(__file__))
POCKET_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/demo/pocket.gb")
ACID2_ROM_PATH = join(CURRENT_DIR, "../../../../res/roms/test/dmg_acid2.gb")


def _has_module(name: str) -> bool:
    from importlib.util import find_spec

    try:
        return find_spec(name) is not None
    except (ImportError, ValueError):
        return False


requires_pocket = unittest.skipUnless(
    exists(POCKET_ROM_PATH),
    f"pocket.gb not present at {POCKET_ROM_PATH}; skipping ROM-dependent test",
)
requires_acid2 = unittest.skipUnless(
    exists(ACID2_ROM_PATH),
    f"dmg_acid2.gb not present at {ACID2_ROM_PATH}; skipping ROM-dependent test",
)

requires_numpy = unittest.skipUnless(
    _has_module("numpy"), "numpy not installed; skipping ndarray-dependent test"
)
requires_pillow = unittest.skipUnless(
    _has_module("PIL"), "Pillow not installed; skipping image-dependent test"
)


class PyBoyV2Test(unittest.TestCase):

    def test_alias(self):
        self.assertIs(PyBoy, PyBoyV2)

    def test_window_event_count(self):
        self.assertEqual(len(list(WindowEvent)), 43)
        self.assertEqual(WindowEvent.CYCLE_PALETTE.value, 42)

    @requires_pocket
    def test_pocket(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            self.assertEqual(pb.cartridge_title, "POCKET-DEMO")
            self.assertFalse(pb.stopped)
            self.assertTrue(pb.tick(10))
            self.assertEqual(pb.frame_count, 10)

    @requires_pocket
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

    @requires_pocket
    @requires_numpy
    @requires_pillow
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

    @requires_pocket
    def test_button(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.button("a", delay=2)
            pb.button_press("start")
            pb.button_release("start")
            with self.assertRaises(ValueError):
                pb.button("invalid")

    @requires_pocket
    def test_motherboard(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            self.assertFalse(pb.mb.cgb)
            self.assertEqual(pb.mb.cartridge.gamename, "POCKET-DEMO")
            self.assertIsInstance(pb.mb.cpu.registers.PC, int)
            self.assertIsInstance(pb.mb.cpu.registers.HL, int)

    @requires_pocket
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


@requires_pocket
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


@requires_acid2
class TileTest(unittest.TestCase):

    @requires_numpy
    @requires_pillow
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


@requires_acid2
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


@requires_acid2
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

    @requires_pocket
    def test_generic_wrapper_auto_selected(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            self.assertIsInstance(pb.game_wrapper, GameWrapper)
            self.assertEqual(pb.game_wrapper.shape, (32, 32))

    @requires_pocket
    def test_start_reset_round_trip(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(60)
            pb.game_wrapper.start_game()
            pre = pb.mb.cpu.registers.PC
            pb.tick(30)
            self.assertNotEqual(pb.mb.cpu.registers.PC, pre)
            pb.game_wrapper.reset_game()
            self.assertEqual(pb.mb.cpu.registers.PC, pre)

    @requires_pocket
    def test_game_area_dimensions(self):
        with PyBoyV2(POCKET_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(60)
            pb.game_area_dimensions(0, 0, 8, 4, follow_scrolling=False)
            area = pb.game_area()
            self.assertEqual(area.shape, (4, 8))

    @requires_pocket
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


@requires_acid2
class SymbolLookupTest(unittest.TestCase):

    def _write_sym(self) -> str:
        sym = tempfile.NamedTemporaryFile(
            suffix=".sym", mode="w", delete=False, encoding="utf-8"
        )
        sym.write("[labels]\n; comment\n00:0100 EntryPoint\n02:6000 BankedFunc\n")
        sym.close()
        return sym.name

    def test_symbol_lookup(self):
        path = self._write_sym()
        try:
            with PyBoyV2(
                ACID2_ROM_PATH,
                window="headless",
                sound_emulated=False,
                symbols=path,
            ) as pb:
                self.assertEqual(pb.symbol_lookup("EntryPoint"), (0, 0x100))
                self.assertEqual(pb.symbol_lookup("BankedFunc"), (2, 0x6000))
                with self.assertRaises(ValueError):
                    pb.symbol_lookup("nope")
        finally:
            os.unlink(path)


@requires_acid2
class HookTest(unittest.TestCase):

    def test_register_and_deregister(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            fired = []

            def cb(ctx):
                fired.append(ctx)

            pb.hook_register(0, 0x0100, cb, context="ctx")
            pb.hook_deregister(0, 0x0100)

    def test_double_register_rejected(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.hook_register(0, 0x0100, lambda ctx: None, None)
            with self.assertRaises(ValueError):
                pb.hook_register(0, 0x0100, lambda ctx: None, None)

    def test_deregister_unknown_rejected(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(ValueError):
                pb.hook_deregister(0, 0xBEEF)


@requires_acid2
class MemoryScannerTest(unittest.TestCase):

    def test_scan_exact(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            for offset, value in enumerate([0x12, 0x34, 0x12, 0x56, 0x12]):
                pb.memory[0xC200 + offset] = value
            addrs = pb.memory_scanner.scan_memory(
                target_value=0x12,
                start_addr=0xC200,
                end_addr=0xC210,
                standard_comparison_type=StandardComparisonType.EXACT,
            )
            self.assertEqual(addrs, [0xC200, 0xC202, 0xC204])

    def test_scan_greater_than(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            for offset, value in enumerate([0x10, 0x40, 0x20, 0x40, 0x10]):
                pb.memory[0xC200 + offset] = value
            addrs = pb.memory_scanner.scan_memory(
                target_value=0x30,
                start_addr=0xC200,
                end_addr=0xC210,
                standard_comparison_type=StandardComparisonType.GREATER_THAN,
            )
            self.assertEqual(addrs, [0xC201, 0xC203])

    def test_rescan_changed(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            for offset, value in enumerate([0x12, 0x12, 0x12]):
                pb.memory[0xC200 + offset] = value
            pb.memory_scanner.scan_memory(
                target_value=0x12,
                start_addr=0xC200,
                end_addr=0xC210,
                standard_comparison_type=StandardComparisonType.EXACT,
            )
            pb.memory[0xC201] = 0x99
            survivors = pb.memory_scanner.rescan_memory(
                dynamic_comparison_type=DynamicComparisonType.CHANGED
            )
            self.assertEqual(survivors, [0xC201])


@requires_acid2
class GameSharkTest(unittest.TestCase):

    def test_apply_8bit_write(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            pb.memory[0xC100] = 0x00
            pb.gameshark.add("014200C1")
            pb.tick(1)
            self.assertEqual(pb.memory[0xC100], 0x42)

    def test_clear_all_restores(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            pb.tick(2)
            pb.memory[0xC101] = 0x07
            pb.gameshark.add("014401C1")
            pb.tick(1)
            self.assertEqual(pb.memory[0xC101], 0x44)
            pb.gameshark.clear_all()
            pb.tick(1)
            self.assertEqual(pb.memory[0xC101], 0x07)

    def test_invalid_codes_rejected(self):
        with PyBoyV2(ACID2_ROM_PATH, window="headless", sound_emulated=False) as pb:
            with self.assertRaises(ValueError):
                pb.gameshark.add("BAD")
            with self.assertRaises(ValueError):
                pb.gameshark.add("01010001")


@requires_acid2
class BotSupportManagerTest(unittest.TestCase):

    def test_manager_type(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            mgr = pb.botsupport_manager()
            self.assertIsInstance(mgr, BotSupportManager)

    @requires_numpy
    @requires_pillow
    def test_legacy_screen(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            pb.tick()
            screen = pb.botsupport_manager().screen()
            self.assertIsInstance(screen, LegacyScreen)
            image = screen.screen_image()
            self.assertEqual(image.size, (160, 144))
            self.assertEqual(image.mode, "RGB")
            arr = screen.screen_ndarray()
            self.assertEqual(arr.shape, (144, 160, 3))
            raw = screen.raw_screen_buffer()
            self.assertEqual(len(raw), 160 * 144 * 3)
            self.assertEqual(screen.raw_screen_buffer_dims(), (144, 160))
            self.assertEqual(screen.raw_screen_buffer_format(), "RGB")

    def test_legacy_tilemap_position(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            pb.tick()
            screen = pb.botsupport_manager().screen()
            pos = screen.tilemap_position()
            self.assertIsInstance(pos, tuple)
            self.assertEqual(len(pos), 2)
            posl = screen.tilemap_position_list()
            self.assertEqual(len(posl), 144)
            self.assertEqual(len(posl[0]), 4)

    def test_sprite_tile_tilemap_forwarding(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            for _ in range(80):
                pb.tick()
            mgr = pb.botsupport_manager()
            self.assertIsInstance(mgr.sprite(0), Sprite)
            self.assertIsInstance(mgr.tile(0), Tile)
            self.assertIsInstance(mgr.tilemap_background(), TileMap)
            self.assertIsInstance(mgr.tilemap_window(), TileMap)

    def test_sprite_by_tile_identifier(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            for _ in range(80):
                pb.tick()
            mgr = pb.botsupport_manager()
            sprite = mgr.sprite(0)
            results = mgr.sprite_by_tile_identifier([sprite.tile_identifier, 999])
            self.assertEqual(len(results), 2)
            self.assertIn(0, results[0])
            self.assertEqual(results[1], [])

    def test_override_memory_value(self):
        with PyBoyV1(ACID2_ROM_PATH, disable_renderer=True, sound_emulated=False) as pb:
            pb.tick()
            pb.set_memory_value(0xC100, 0x42)
            self.assertEqual(pb.get_memory_value(0xC100), 0x42)
            # WRAM writes go through the bus, ROM patches are rejected
            pb.override_memory_value(0, 0xC101, 0x77)
            self.assertEqual(pb.get_memory_value(0xC101), 0x77)
            with self.assertRaises(RuntimeError):
                pb.override_memory_value(0, 0x0100, 0xFF)
