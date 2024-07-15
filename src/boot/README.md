# Boytacean Boot ROM files

These files have been forked from the [SameBoy](https://github.com/LIJI32/SameBoy) boot Roms.

Special thanks to the original creators and maintainers of [SameBoy](https://github.com/LIJI32/SameBoy) for their work and contributions to the open source community. Their work has been invaluable to the development of the Boytacean project.

## 2bpp file format

To convert a PNG into the 2BPP format, use the following command:

```bash
rgbgfx -Z -c embedded -t logo.tilemap -o logo.2bpp logo.png
```

To reverse the 2bpp files to PNG, use the following command:

```bash
rgbgfx -Z -r 16 -o logo.2bpp -t logo.tilemap logo.reverse.png
```

To run the complete process with compression and de-compression use the following command:

```bash
rgbgfx -Z -c embedded -t logo.tilemap -o logo.2bpp logo.png
python pb12.py compress logo.2bpp logo.pb12
python pb12.py decompress logo.pb12 logo.decompress.2bpp
rgbgfx -Z -r 16 -o logo.decompress.2bpp -t logo.tilemap logo.reverse.png
```

## Resources

* [Gameboy 2BPP Graphics Format](https://www.huderlem.com/demos/gameboy2bpp.html)
* [rgbgfx(1) — Game Boy graphics converter](https://rgbds.gbdev.io/docs/master/rgbgfx.1)
