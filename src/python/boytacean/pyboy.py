from PIL.Image import Image

from .gb import GameBoy, GameBoyMode


class PyBoy(GameBoy):
    def __init__(
        self,
        gamerom_file,
        *,
        bootrom_file=None,
        disable_renderer=False,
        sound=False,
        sound_emulated=False,
        cgb=None,
        randomize=False,
        **kwargs
    ):
        super().__init__(
            mode=GameBoyMode.CGB if cgb else GameBoyMode.DMG,
            apu_enabled=sound_emulated,
            load_graphics=not disable_renderer,
            load=True,
            boot=not bool(bootrom_file),
        )
        if bootrom_file:
            self.load_boot_path(bootrom_file)
        if gamerom_file:
            self.load_rom(gamerom_file)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        pass

    def set_emulation_speed(self, speed: float):
        print("Missing emulation speed control")

    def tick(self):
        super().next_frame()

    def stop(self):
        pass

    def cartridge_title(self) -> str:
        return self.rom_title

    def screen_image(self) -> Image:
        return self.image()

    def get_memory_value(self, addr: int) -> int:
        return self.read_memory(addr)

    def set_memory_value(self, addr: int, value: int):
        self.write_memory(addr, value)
