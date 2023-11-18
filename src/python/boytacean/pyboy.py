from .gb import GameBoy


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
        super().__init__(apu_enabled=sound)

    def tick(self):
        super().tick()

    def load_rom(self, rom):
        super().load_rom(rom)
