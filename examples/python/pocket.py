import boytacean

gb = boytacean.GameBoy()
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")
for _ in range(6000):
    gb.next_frame()
gb.save_image("pocket.png")
