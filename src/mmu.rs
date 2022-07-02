use crate::ppu::Ppu;

pub const BIOS_SIZE: usize = 256;
pub const ROM_SIZE: usize = 32768;
pub const RAM_SIZE: usize = 8192;
pub const ERAM_SIZE: usize = 8192;

pub struct Mmu {
    ppu: Ppu,
    boot_active: bool,
    boot: [u8; BIOS_SIZE],
    rom: [u8; ROM_SIZE],
    ram: [u8; RAM_SIZE],
    eram: [u8; RAM_SIZE],
}

impl Mmu {
    pub fn new(ppu: Ppu) -> Mmu {
        Mmu {
            ppu: ppu,
            boot_active: true,
            boot: [0u8; BIOS_SIZE],
            rom: [0u8; ROM_SIZE],
            ram: [0u8; RAM_SIZE],
            eram: [0u8; ERAM_SIZE],
        }
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        &mut self.ppu
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
                self.rom[addr as usize]
            }
            // ROM0 (12 KB/16 KB)
            0x1000 | 0x2000 | 0x3000 => self.rom[addr as usize],
            // ROM1 (Unbanked) (16 KB)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.rom[addr as usize],
            // Graphics: VRAM (8 KB)
            0x8000 | 0x9000 => self.ppu.vram[(addr & 0x1fff) as usize],
            // External RAM (8 KB)
            0xa000 | 0xb000 => self.eram[(addr & 0x1fff) as usize],
            // Working RAM (8 KB)
            0xc000 | 0xd000 => self.ram[(addr & 0x1fff) as usize],
            // Working RAM Shadow
            0xe000 => self.ram[(addr & 0x1fff) as usize],
            // Working RAM Shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => self.ram[(addr & 0x1fff) as usize],
                0xe00 => {
                    println!("Reading from PPU OAM - NOT IMPLEMENTED");
                    0x00
                }
                0xf00 => {
                    if addr >= 0xff80 {
                        self.ppu.hram[(addr & 0x007f) as usize]
                    } else {
                        match addr & 0x00f0 {
                            0x40 | 0x50 | 0x60 | 0x70 => self.ppu.read(addr),
                            _ => {
                                println!("Reading from unknown IO control 0x{:04x}", addr);
                                0x00
                            }
                        }
                    }
                }
                addr => panic!("Reading from unknown location 0x{:04x}", addr),
            },
            addr => panic!("Reading from unknown location 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            // BOOT (256 B) + ROM0 (4 KB/16 KB)
            0x0000 => {
                println!("WRITING to BOOT")
            }
            // ROM0 (12 KB/16 KB)
            0x1000 | 0x2000 | 0x3000 => {
                println!("WRITING TO ROM 0");
            }
            // ROM1 (Unbanked) (16 KB)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                println!("WRITING TO ROM 1");
            }
            // Graphics: VRAM (8 KB)
            0x8000 | 0x9000 => {
                self.ppu.vram[(addr & 0x1fff) as usize] = value;
                if addr < 0x9800 {
                    self.ppu.update_tile(addr, value);
                }
            }
            // External RAM (8 KB)
            0xa000 | 0xb000 => {
                self.eram[(addr & 0x1fff) as usize] = value;
            }
            // Working RAM (8 KB)
            0xc000 | 0xd000 => {
                self.ram[(addr & 0x1fff) as usize] = value;
            }
            // Working RAM Shadow
            0xe000 => {
                self.ram[(addr & 0x1fff) as usize] = value;
            }
            // Working RAM Shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => {
                    self.ram[(addr & 0x1fff) as usize] = value;
                }
                0xe00 => {
                    println!("Writing to PPU OAM");
                }
                0xf00 => {
                    if addr >= 0xff80 {
                        self.ppu.hram[(addr & 0x007f) as usize] = value;
                    } else {
                        match addr & 0x00f0 {
                            0x40 | 0x50 | 0x60 | 0x70 => {
                                self.ppu.write(addr, value);
                            }
                            _ => {
                                println!("Writing to unknown IO control 0x{:04x}", addr);
                            }
                        }
                    }
                }
                addr => panic!("Writing in unknown location 0x{:04x}", addr),
            },
            addr => panic!("Writing in unknown location 0x{:04x}", addr),
        }
    }

    pub fn write_boot(&mut self, addr: u16, buffer: &[u8]) {
        self.boot[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn write_ram(&mut self, addr: u16, buffer: &[u8]) {
        self.ram[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }

    pub fn write_rom(&mut self, addr: u16, buffer: &[u8]) {
        self.rom[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }
}
