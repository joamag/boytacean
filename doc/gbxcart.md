# GBxCart RW

This document describes the serial protocol that Boytacean uses to read and flash Game Boy cartridges through a [GBxCart RW](https://www.gbxcart.com) running the L firmware. The firmware itself is undocumented, so everything here was established from the serial traffic between FlashGBX 3.37 and a GBxCart RW v1.3 with the L1 firmware (captured with `socat -x -v` between the program and the device), and from probes sent to the device, never from the code of other programs. The names are Boytacean's own, as used in `crates/cartridge/src/gbxcart.rs`.

## Connection

The device shows up as a CH340 USB serial converter (`1a86:7523`) and talks at 1 Mbaud (8N1). The port can only be open in one program at a time.

## Commands

Every command starts with a single byte, some of them followed by arguments (big endian), and the reply (if any) has a fixed size.

| Byte   | Name                 | Arguments                                                    | Reply                                                                                           |
| ------ | -------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `0xA1` | `QUERY_FW_INFO`      |                                                              | 9 bytes: length (8), kind (`L`), version (u16), PCB version (4 for v1.3), build timestamp (u32) |
| `0xA3` | `SET_MODE_DMG`       |                                                              |                                                                                                 |
| `0xA4` | `SET_VOLTAGE_3_3V`   |                                                              |                                                                                                 |
| `0xA5` | `SET_VOLTAGE_5V`     |                                                              |                                                                                                 |
| `0xA6` | `SET_VARIABLE`       | size (1), key (u32), value (u32)                             |                                                                                                 |
| `0xA7` | `SET_FLASH_COMMANDS` | `01 02 01`, then 6 commands of address (u32) and value (u16) |                                                                                                 |
| `0xA8` | `SET_ADDR_AS_INPUTS` |                                                              |                                                                                                 |
| `0xB1` | `DMG_CART_READ`      |                                                              | `TRANSFER_SIZE` bytes read from `ADDRESS`, which then moves past them                           |
| `0xB2` | `DMG_CART_WRITE`     | address (u32), value (1)                                     |                                                                                                 |
| `0xB4` | `DMG_MBC_RESET`      |                                                              | `0x01`                                                                                          |
| `0xD1` | `DMG_FLASH_WRITE`    | address (u32), value (1)                                     |                                                                                                 |
| `0xD3` | `DMG_FLASH_PROGRAM`  | `TRANSFER_SIZE` bytes of data                                | `0x01` once programmed                                                                          |

The variables set with `SET_VARIABLE`, identified by their size and key:

| Size | Key    | Name                 | Use                                                                 |
| ---- | ------ | -------------------- | ------------------------------------------------------------------- |
| 1    | `0x00` | `CART_MODE`          | 1 for Game Boy cartridges                                           |
| 1    | `0x01` | `DMG_ACCESS_MODE`    | 1 to read the ROM, 3 to read the RAM                                |
| 1    | `0x05` | `FLASH_PROGRAM_MODE` | must be 0 for `DMG_FLASH_PROGRAM` to program and reply as described |
| 1    | `0x08` | `DMG_READ_CS_PULSE`  | 1 while reading the RAM, which needs the chip select pulsed         |
| 1    | `0x09` | `DMG_WRITE_CS_PULSE` | 0                                                                   |
| 1    | `0x0B` | `DMG_READ_METHOD`    | 1                                                                   |
| 2    | `0x00` | `TRANSFER_SIZE`      | number of bytes of each read or program                             |
| 2    | `0x01` | `FLASH_BUFFER_SIZE`  | size of the write buffer of the flash chip (from its CFI table)     |
| 2    | `0x02` | `DMG_ROM_BANK`       | ROM bank selected in the MBC, set right after selecting it          |
| 4    | `0x00` | `ADDRESS`            | address of the next read or program                                 |

## Reading

1. `QUERY_FW_INFO`, refusing anything but the L firmware.
2. `SET_MODE_DMG`, `SET_VOLTAGE_5V`, `DMG_READ_METHOD` = 1, `CART_MODE` = 1, both chip select pulses at 0, and `DMG_MBC_RESET`.
3. The header (`0x0100` to `0x014F`) is read twice, the two reads must match and carry the Nintendo logo and a valid header checksum.
4. Each ROM bank is selected with the bank switching writes of its MBC (`DMG_CART_WRITE`) and read in transfers of 64 bytes.
5. For battery backed cartridges, the RAM is enabled, each bank selected and read with `DMG_ACCESS_MODE` = 3 and the chip select pulsed, and the RAM disabled again.
6. `SET_ADDR_AS_INPUTS` releases the cartridge bus.

Reading runs at about 56 to 61 KiB/s.

## Flashing

Flash cartridges hold a NOR flash chip behind an MBC5 compatible mapper. The whole chip is written through the switchable window at `0x4000`, selecting each 16 KiB bank of the chip with the MBC5 (`0x3000` for bit 8, `0x2100` for the lower bits, bank 0 included), while the commands to the chip go through `0x0000` to `0x3FFF`, where the bank 0 of the chip always is.

1. `QUERY_FW_INFO`, then `SET_VOLTAGE_3_3V`, `SET_MODE_DMG`, `DMG_READ_METHOD` = 1, `CART_MODE` = 1, both chip select pulses at 0, `DMG_MBC_RESET` and `FLASH_PROGRAM_MODE` = 0, followed by a read of the start of the cartridge: once the address lines were released (`SET_ADDR_AS_INPUTS`, as at the end of a previous run), writes don't reach the cartridge until the first read, which made the first CFI query of a run fail every time.
2. CFI query: `DMG_FLASH_WRITE` of `0x98` to `0x00AA`, then 256 bytes read from `0x0000`. In this wiring every CFI value shows up twice, so the value at CFI offset `n` is the byte at `2n`. The table must start with `QRY` and declare the AMD command set (`0x0002`), and gives the size of the chip, the size of its write buffer and its erase regions (number and size of the sectors). A reset (`0xF0` to `0x0000`) leaves the query.
3. Identification: `0xAA` to `0x0AAA`, `0x55` to `0x0555`, `0x90` to `0x0AAA`, then 4 bytes read from `0x0000` (manufacturer and device, each twice), and a reset.
4. `SET_FLASH_COMMANDS` with the buffered programming of the AMD command set: `0xAA` to `0x0AAA`, `0x55` to `0x0555`, then `0x25`, the buffer size, the data and `0x29` (the zero addresses and values of those last four are filled in by the firmware).
5. For every bank of the ROM, in order: select it (`0x3000` and `0x2100`) and set `DMG_ROM_BANK`, then erase every sector starting in the bank (reset, `0xAA` to `0x0AAA`, `0x55` to `0x0555`, `0x80` to `0x0AAA`, `0xAA` to `0x0AAA`, `0x55` to `0x0555`, then `0x30` to the sector address in the window), reading 2 bytes from the sector until they are `0xFF 0xFF` (the chip toggles other values while erasing) and resetting, and program the bank: `TRANSFER_SIZE` = 256, `FLASH_BUFFER_SIZE` and `ADDRESS` = `0x4000`, then 64 `DMG_FLASH_PROGRAM` of 256 bytes each, every one acknowledged with `0x01`. A sector larger than a bank (64 KiB) is erased once, from the first bank it covers.
6. Every bank is read back and compared with the ROM.
7. `SET_VOLTAGE_5V` and `SET_ADDR_AS_INPUTS`.

Any failure once the chip is detected (a ROM that doesn't fit, an erase timeout, a refused program or a failed verification) resets the chip, then sends `SET_VOLTAGE_5V` and `SET_ADDR_AS_INPUTS`, leaving the cartridge readable. An interrupted or failed flashing leaves the chip partly erased and programmed, which flashing it again recovers, as long as nothing replaces the backup of the original contents: the retry is made with `--no-backup` once that backup is kept safe (also as the cartridge may no longer have a valid header to back up), as in `gbxcart flash game.gb --yes --no-backup`, or `gbxcart flash cartridge.backup.gb --yes --no-backup` to restore the original contents.

Measured on a 4 MiB flash cartridge (CFI ID `01 7E`, 8 sectors of 8 KiB then 63 of 64 KiB, 32 byte write buffer): erasing a sector takes about 0.1 s, and programming runs at about 28 KiB/s (9 ms per 256 bytes), so a 4 MiB ROM takes about 2.5 minutes.

## Unknowns

* The meaning of the `01 02 01` header of `SET_FLASH_COMMANDS`, which is sent as observed.
* FlashGBX also sets the 1 byte variables `0x06` and `0x0A` to 0 around `SET_FLASH_COMMANDS`, with no effect observed on the cartridge above when set to 1, so Boytacean leaves them alone.
* `0x56` and `0x68`, which FlashGBX sends before `QUERY_FW_INFO` (replying `0x00` and the PCB version), are not needed.
* `DMG_FLASH_WRITE` and `DMG_CART_WRITE` behave the same for the commands above on the cartridge above. Boytacean uses the former for the flash chip, as FlashGBX does, as they may differ for other cartridges.
* Flash cartridges that write through the AUDIO pin, that swap data or address lines, or with other command sets, are not covered.
