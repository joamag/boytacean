use crate::ppu::Ppu;

pub const BIOS_SIZE: usize = 256;
pub const ROM_SIZE: usize = 32768;
pub const RAM_SIZE: usize = 8192;
pub const ERAM_SIZE: usize = 8192;
pub const HRAM_SIZE: usize = 128;

pub struct Mmu {
    ppu: Ppu,
    boot_active: bool,
    boot: [u8; BIOS_SIZE],
    rom: [u8; ROM_SIZE],
    ram: [u8; RAM_SIZE],
    eram: [u8; RAM_SIZE],
    hram: [u8; HRAM_SIZE],
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
            hram: [0u8; HRAM_SIZE],
        }
    }

    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0xf000 {
            // BIOS
            0x0000 => {
                // in case the boot mode is active and the
                // address is withing boot memory reads from it
                if self.boot_active && addr <= 0x00fe {
                    if addr == 0x00fe {
                        self.boot_active = false;
                    }
                    return self.boot[addr as usize];
                }
                self.rom[addr as usize]
            }
            // ROM0
            0x1000 | 0x2000 | 0x3000 => self.rom[addr as usize],
            // ROM1 (unbanked) (16k)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.rom[addr as usize],
            // Graphics: VRAM (8k)
            0x8000 | 0x9000 => {
                println!("READING FROM VRAM");
                self.ppu.vram[(addr & 0x1fff) as usize]
            }
            // External RAM (8k)
            0xa000 | 0xb000 => {
                println!("READING FROM ERAM");
                self.eram[(addr & 0x1fff) as usize]
            }
            // Working RAM (8k)
            0xc000 | 0xd000 => self.ram[(addr & 0x1fff) as usize],
            // Working RAM shadow
            0xe000 => {
                println!("READING FROM RAM Shadow");
                self.ram[(addr & 0x1fff) as usize]
            }
            // Working RAM shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => self.ram[(addr & 0x1fff) as usize],
                0xe00 => {
                    println!("READING FROM GPU OAM - NOT IMPLEMENTED");
                    0x00
                }
                0xf00 => {
                    if addr >= 0xff80 {
                        self.hram[(addr & 0x7f) as usize]
                    } else {
                        println!("WRITING TO IO control");
                        0x00
                    }
                }
                addr => panic!("Reading from unknown location 0x{:04x}", addr),
            },
            addr => panic!("Reading from unknown location 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            // BOOT
            0x0000 => {
                println!("WRITING to BOOT")
            }
            // ROM0
            0x1000 | 0x2000 | 0x3000 => {
                println!("WRITING TO ROM 0");
            }
            // ROM1 (unbanked) (16k)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                println!("WRITING TO ROM 1");
            }
            // Graphics: VRAM (8k)
            0x8000 | 0x9000 => {
                println!("WRITING TO VRAM");
                self.ppu.vram[(addr & 0x1fff) as usize] = value;
            }
            // External RAM (8k)
            0xa000 | 0xb000 => {
                println!("WRITING TO ERAM");
            }
            // Working RAM (8k)
            0xc000 | 0xd000 => {
                println!("WRITING TO RAM");
                self.ram[(addr & 0x1fff) as usize] = value;
            }
            // Working RAM shadow
            0xe000 => {
                println!("WRITING TO RAM Shadow");
                self.ram[(addr & 0x1fff) as usize] = value;
            }
            // Working RAM shadow, I/O, Zero-page RAM
            0xf000 => match addr & 0x0f00 {
                0x000 | 0x100 | 0x200 | 0x300 | 0x400 | 0x500 | 0x600 | 0x700 | 0x800 | 0x900
                | 0xa00 | 0xb00 | 0xc00 | 0xd00 => {
                    self.ram[(addr & 0x1fff) as usize] = value;
                }
                0xe00 => {
                    println!("WRITING TO GPU OAM");
                }
                0xf00 => {
                    if addr >= 0xff80 {
                        self.hram[(addr & 0x7f) as usize] = value;
                    } else {
                        println!("WRITING TO IO control");
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
}
