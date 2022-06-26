use crate::ppu::Ppu;

pub const RAM_SIZE: usize = 8192;

pub struct Mmu {
    ppu: Ppu,
    ram: [u8; RAM_SIZE],
}

impl Mmu {
    pub fn new(ppu: Ppu) -> Mmu {
        Mmu {
            ppu: ppu,
            ram: [0u8; RAM_SIZE],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            // BIOS
            0x0000 => {
                println!("WRITING to BIOS")
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
                        println!("WRITING TO Zero page");
                    } else {
                        println!("WRITING TO IO control");
                    }
                }
                addr => panic!("Writing in unknown location 0x{:04x}", addr),
            },
            addr => panic!("Writing in unknown location 0x{:04x}", addr),
        }
    }

    pub fn write_buffer(&mut self, addr: u16, buffer: &[u8]) {
        self.ram[addr as usize..addr as usize + buffer.len()].clone_from_slice(buffer);
    }
}
