import boytacean

gb = boytacean.GameBoy()
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")

for i in range(10000000):
    gb.clock()

gb.save_image("tobias.png")
