import boytacean

gb = boytacean.GameBoy()
gb.load()
gb.load_rom("../../res/roms/demo/pocket.gb")
gb.clocks(10000000)
gb.save_image("pocket.png")
