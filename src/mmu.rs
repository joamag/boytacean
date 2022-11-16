use crate::{debugln, pad::Pad, ppu::Ppu, rom::Cartridge, timer::Timer};

pub const BOOT_SIZE: usize = 2304;
pub const RAM_SIZE: usize = 8192;

pub struct Mmu {
    /// Register that controls the interrupts that are considered
    /// to be enabled and should be triggered.
    pub ie: u8,

    /// Reference to the PPU (Pixel Processing Unit) that is going
    /// to be used both for VRAM reading/writing and to forward
    /// some of the access operations.
    ppu: Ppu,

    /// Reference to the Game Pad structure that is going to control
    /// the I/O access to this device.
    pad: Pad,

    /// The timer controller to be used as part of the I/O access
    /// that is memory mapped.
    timer: Timer,

    /// The cartridge ROM that is currently loaded into the system,
    /// going to be used to access ROM and external RAM banks.
    rom: Cartridge,

    /// Flag that control the access to the boot section in the
    /// 0x0000-0x00FE memory area, this flag should be unset after
    /// the boot sequence has been finished.
    boot_active: bool,

    /// Buffer to be used to store the boot ROM, this is the code
    /// that is going to be executed at the beginning of the Game
    /// Boy execution. The buffer effectively used is of 256 bytes
    /// for the "normal" Game Boy (MGB) and 2308 bytes for the
    /// Game Boy Color (CGB). Note that in the case of the CGB
    /// the bios which is 2308 bytes long is in fact only 2048 bytes
    /// as the 256 bytes in range 0x100-0x1FF are meant to be
    /// overwritten byte the cartridge header.
    boot: [u8; BOOT_SIZE],

    ram: [u8; RAM_SIZE],
}

impl Mmu {
    pub fn new(ppu: Ppu, pad: Pad, timer: Timer) -> Self {
        Self {
            ppu,
            pad,
            timer,
            rom: Cartridge::new(),
            boot_active: true,
            boot: [0u8; BOOT_SIZE],
            ram: [0u8; RAM_SIZE],
            ie: 0x0,
        }
    }

    pub fn reset(&mut self) {
        self.rom = Cartridge::new();
        self.boot_active = true;
        self.boot = [0u8; BOOT_SIZE];
        self.ram = [0u8; RAM_SIZE];
        self.ie = 0x0;
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        &mut self.ppu
    }

    pub fn pad(&mut self) -> &mut Pad {
        &mut self.pad
    }

    pub fn timer(&mut self) -> &mut Timer {
        &mut self.timer
    }

    pub fn boot_active(&self) -> bool {
        self.boot_active
    }

    pub fn set_boot_active(&mut self, value: bool) {
        self.boot_active = value;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0xf000 {
            // BOOT (256 B) + ROM0 (4 KB/16 KB)
            0x0000 => {
                // in case the boot mode is active and the
                // address is withing boot memory reads from it
                if self.boot_active && addr <= 0x00fe {
                    // if we're reading from this location we can
                    // safely assume that we're exiting the boot
                    // loading sequence and disable boot
                    if addr == 0x00fe {
                        self.boot_active = false;
                    }
                    return self.boot[addr as usize];
                }
                self.rom.read(addr)
            }

            // ROM 0 (12 KB/16 KB)
            0x1000 | 0x2000 | 0x3000 => self.rom.read(addr),

            // ROM 1 (Unbanked) (16 KB)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.rom.read(addr),

            // Graphics: VRAM (8 KB)
            0x8000 | 0x9000 => self.ppu.vram[(addr & 0x1fff) as usize],

            // External RAM (8 KB)
            0xa000 | 0xb000 => self.rom.read(addr),

            // Working RAM (8 KB)
            0xc000 | 0xd000 => self.ram[(addr & 0x1fff) as usize],

            // Working RAM Shadow
            0xe000 => self.ram[(addr & 0x1fff) as usize],

            // Working RAM Shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => self.ram[(addr & 0x1fff) as usize],
                0xe00 => self.ppu.oam[(addr & 0x009f) as usize],
                0xf00 => match addr & 0x00ff {
                    // 0xFF0F — IF: Interrupt flag
                    0x0f => {
                        (if self.ppu.int_vblank() { 0x01 } else { 0x00 }
                            | if self.ppu.int_stat() { 0x02 } else { 0x00 }
                            | if self.timer.int_tima() { 0x04 } else { 0x00 }
                            | if self.pad.int_pad() { 0x10 } else { 0x00 })
                    }

                    // 0xFF50 - Boot active flag
                    0x50 => {
                        if self.boot_active {
                            0x01
                        } else {
                            0x00
                        }
                    }

                    // 0xFF80-0xFFFE - High RAM (HRAM)
                    0x80..=0xfe => self.ppu.hram[(addr & 0x007f) as usize],

                    // 0xFFFF — IE: Interrupt enable
                    0xff => self.ie,

                    // Other registers
                    _ => match addr & 0x00f0 {
                        0x00 => match addr & 0x00ff {
                            0x00 => self.pad.read(addr),
                            0x04..=0x07 => self.timer.read(addr),
                            _ => {
                                debugln!("Reading from unknown IO control 0x{:04x}", addr);
                                0x00
                            }
                        },
                        0x40 | 0x50 | 0x60 | 0x70 => self.ppu.read(addr),
                        _ => {
                            debugln!("Reading from unknown IO control 0x{:04x}", addr);
                            0x00
                        }
                    },
                },
                addr => panic!("Reading from unknown location 0x{:04x}", addr),
            },

            addr => panic!("Reading from unknown location 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            // BOOT (256 B) + ROM0 (4 KB/16 KB)
            0x0000 => self.rom.write(addr, value),

            // ROM 0 (12 KB/16 KB)
            0x1000 | 0x2000 | 0x3000 => self.rom.write(addr, value),

            // ROM 1 (Unbanked) (16 KB)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.rom.write(addr, value),

            // Graphics: VRAM (8 KB)
            0x8000 | 0x9000 => {
                self.ppu.vram[(addr & 0x1fff) as usize] = value;
                if addr < 0x9800 {
                    self.ppu.update_tile(addr, value);
                }
            }

            // External RAM (8 KB)
            0xa000 | 0xb000 => self.rom.write(addr, value),

            // Working RAM (8 KB)
            0xc000 | 0xd000 => self.ram[(addr & 0x1fff) as usize] = value,

            // Working RAM Shadow
            0xe000 => self.ram[(addr & 0x1fff) as usize] = value,

            // Working RAM Shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => {
                    self.ram[(addr & 0x1fff) as usize] = value;
                }
                0xe00 => {
                    self.ppu.oam[(addr & 0x009f) as usize] = value;
                    self.ppu.update_object(addr, value);
                }
                0xf00 => match addr & 0x00ff {
                    // 0xFF0F — IF: Interrupt flag
                    0x0f => {
                        self.ppu.set_int_vblank(value & 0x01 == 0x01);
                        self.ppu.set_int_stat(value & 0x02 == 0x02);
                        self.timer.set_int_tima(value & 0x04 == 0x04);
                        self.pad.set_int_pad(value & 0x10 == 0x10);
                    }

                    // 0xFF50 - Boot active flag
                    0x50 => self.boot_active = false,

                    // 0xFF80-0xFFFE - High RAM (HRAM)
                    0x80..=0xfe => self.ppu.hram[(addr & 0x007f) as usize] = value,

                    // 0xFFFF — IE: Interrupt enable
                    0xff => self.ie = value,

                    // Other registers
                    _ => {
                        match addr & 0x00f0 {
                            0x00 => match addr & 0x00ff {
                                0x00 => self.pad.write(addr, value),
                                0x04..=0x07 => self.timer.write(addr, value),
                                _ => debugln!("Writing to unknown IO control 0x{:04x}", addr),
                            },
                            0x40 | 0x60 | 0x70 => {
                                match addr & 0x00ff {
                                    // 0xFF46 — DMA: OAM DMA source address & start
                                    0x0046 => {
                                        // @TODO must increment the cycle count by 160
                                        // and make this a separated dma.rs file
                                        debugln!("Going to start DMA transfer to 0x{:x}00", value);
                                        let data = self.read_many((value as u16) << 8, 160);
                                        self.write_many(0xfe00, &data);
                                    }

                                    // VRAM related write
                                    _ => self.ppu.write(addr, value),
                                }
                            }
                            0x50 => match addr & 0x00ff {
                                // 0xFF51-0xFF52 - VRAM DMA source (CGB only)
                                0x51..=0x52 => (),

                                _ => debugln!("Writing to unknown IO control 0x{:04x}", addr),
                            },
                            _ => debugln!("Writing to unknown IO control 0x{:04x}", addr),
                        }
                    }
                },
                addr => panic!("Writing to unknown location 0x{:04x}", addr),
            },

            addr => panic!("Writing to unknown location 0x{:04x}", addr),
        }
    }

    pub fn write_many(&mut self, addr: u16, data: &[u8]) {
        for (index, byte) in data.iter().enumerate() {
            self.write(addr + index as u16, *byte)
        }
    }

    pub fn read_many(&mut self, addr: u16, count: u16) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];

        for index in 0..count {
            let byte = self.read(addr + index);
            data.push(byte);
        }

        data
    }

    pub fn write_boot(&mut self, addr: u16, buffer: &[u8]) {
        self.boot[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn write_ram(&mut self, addr: u16, buffer: &[u8]) {
        self.ram[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn rom(&mut self) -> &mut Cartridge {
        &mut self.rom
    }

    pub fn set_rom(&mut self, rom: Cartridge) {
        self.rom = rom;
    }
}
