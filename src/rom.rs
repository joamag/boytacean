use core::fmt;
use std::{
    cmp::max,
    fmt::{Display, Formatter},
};

use crate::debugln;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub const ROM_BANK_SIZE: usize = 16384;
pub const RAM_BANK_SIZE: usize = 8192;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum RomType {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc2 = 0x05,
    Mbc2Battery = 0x06,
    Unknown = 0xff,
}

impl Display for RomType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            RomType::RomOnly => "ROM Only",
            RomType::Mbc1 => "MBC 1",
            RomType::Mbc1Ram => "MBC 1 + RAM",
            RomType::Mbc1RamBattery => "MBC 1 + RAM + Battery",
            RomType::Mbc2 => "MBC 2",
            RomType::Mbc2Battery => "MBC 2 + RAM",
            RomType::Unknown => "Unknown",
        };
        write!(f, "{}", str)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum RomSize {
    Size32K,
    Size64K,
    Size128K,
    Size256K,
    Size512K,
    Size1M,
    Size2M,
    Size4M,
    Size8M,
    SizeUnknown,
}

impl RomSize {
    pub fn rom_banks(&self) -> u16 {
        match self {
            RomSize::Size32K => 2,
            RomSize::Size64K => 4,
            RomSize::Size128K => 8,
            RomSize::Size256K => 16,
            RomSize::Size512K => 32,
            RomSize::Size1M => 64,
            RomSize::Size2M => 128,
            RomSize::Size4M => 256,
            RomSize::Size8M => 512,
            RomSize::SizeUnknown => 0,
        }
    }
}

impl Display for RomSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            RomSize::Size32K => "32 KB",
            RomSize::Size64K => "64 KB",
            RomSize::Size128K => "128 KB",
            RomSize::Size256K => "256 KB",
            RomSize::Size512K => "512 KB",
            RomSize::Size1M => "1 MB",
            RomSize::Size2M => "2 MB",
            RomSize::Size4M => "4 MB",
            RomSize::Size8M => "8 MB",
            RomSize::SizeUnknown => "Unknown",
        };
        write!(f, "{}", str)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum RamSize {
    NoRam,
    Unused,
    Size8K,
    Size32K,
    Size64K,
    Size128K,
    SizeUnknown,
}

impl RamSize {
    pub fn ram_banks(&self) -> u16 {
        match self {
            RamSize::NoRam => 0,
            RamSize::Unused => 0,
            RamSize::Size8K => 1,
            RamSize::Size32K => 4,
            RamSize::Size64K => 8,
            RamSize::Size128K => 16,
            RamSize::SizeUnknown => 0,
        }
    }
}

impl Display for RamSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            RamSize::NoRam => "No RAM",
            RamSize::Unused => "Unused",
            RamSize::Size8K => "8 KB",
            RamSize::Size32K => "32 KB",
            RamSize::Size128K => "128 KB",
            RamSize::Size64K => "64 KB",
            RamSize::SizeUnknown => "Unknown",
        };
        write!(f, "{}", str)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone)]
pub struct Cartridge {
    /// The complete data of the ROM cartridge, should
    /// include the complete set o ROM banks.
    rom_data: Vec<u8>,

    /// The base RAM that is going to be used to store
    /// temporary data for basic cartridges.
    ram_data: Vec<u8>,

    /// The MBC (Memory Bank Controller) to be used for
    /// RAM and ROM access on the current cartridge.
    mbc: &'static Mbc,

    /// The offset address to the ROM bank (#1) that is
    /// currently in use by the ROM cartridge.
    rom_offset: usize,

    /// The offset address to the ERAM bank that is
    /// currently in use by the ROM cartridge.
    ram_offset: usize,
}

impl Cartridge {
    pub fn new() -> Self {
        Self {
            rom_data: vec![],
            ram_data: vec![],
            mbc: &NO_MBC,
            rom_offset: 0x4000,
            ram_offset: 0x0000,
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        let mut cartridge = Cartridge::new();
        cartridge.set_data(data);
        cartridge
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr & 0xf000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 | 0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                (self.mbc.read_rom)(self, addr)
            }
            0xa000 | 0xb000 => (self.mbc.read_ram)(self, addr),
            _ => {
                debugln!("Reading from unknown Cartridge control 0x{:04x}", addr);
                0x00
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 | 0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                (self.mbc.write_rom)(self, addr, value)
            }
            0xa000 | 0xb000 => (self.mbc.write_ram)(self, addr, value),
            _ => debugln!("Writing to unknown Cartridge address 0x{:04x}", addr),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.rom_data
    }

    pub fn get_bank(&self, index: u8) -> &[u8] {
        let start = index as usize * ROM_BANK_SIZE;
        let end = (index + 1) as usize * ROM_BANK_SIZE;
        &self.rom_data[start..end]
    }

    pub fn get_mbc(&self) -> &'static Mbc {
        match self.rom_type() {
            RomType::RomOnly => &NO_MBC,
            RomType::Mbc1 => &MBC1,
            RomType::Mbc1Ram => &MBC1,
            RomType::Mbc1RamBattery => &MBC1,
            _ => &NO_MBC,
        }
    }

    pub fn set_rom_bank(&mut self, rom_bank: u8) {
        self.rom_offset = rom_bank as usize * ROM_BANK_SIZE;
    }

    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_offset = ram_bank as usize * RAM_BANK_SIZE;
    }

    fn set_data(&mut self, data: &[u8]) {
        self.rom_data = data.to_vec();
        self.rom_offset = 0x4000;
        self.ram_offset = 0x0000;
        self.set_mbc();
        self.allocate_ram();
        self.set_rom_bank(1);
        self.set_ram_bank(0);
    }

    fn set_mbc(&mut self) {
        self.mbc = self.get_mbc();
    }

    fn allocate_ram(&mut self) {
        let ram_banks = max(self.ram_size().ram_banks(), 1);
        self.ram_data = vec![0u8; ram_banks as usize * RAM_BANK_SIZE];
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Cartridge {
    pub fn title(&self) -> String {
        String::from(std::str::from_utf8(&self.rom_data[0x0134..0x0143]).unwrap())
    }

    pub fn rom_type(&self) -> RomType {
        match self.rom_data[0x0147] {
            0x00 => RomType::RomOnly,
            0x01 => RomType::Mbc1,
            0x02 => RomType::Mbc1Ram,
            0x03 => RomType::Mbc1RamBattery,
            0x05 => RomType::Mbc2,
            0x06 => RomType::Mbc2Battery,
            _ => RomType::Unknown,
        }
    }

    pub fn rom_size(&self) -> RomSize {
        match self.rom_data[0x0148] {
            0x00 => RomSize::Size32K,
            0x01 => RomSize::Size64K,
            0x02 => RomSize::Size128K,
            0x03 => RomSize::Size256K,
            0x04 => RomSize::Size512K,
            0x05 => RomSize::Size1M,
            0x06 => RomSize::Size2M,
            0x07 => RomSize::Size4M,
            0x08 => RomSize::Size8M,
            _ => RomSize::SizeUnknown,
        }
    }

    pub fn ram_size(&self) -> RamSize {
        match self.rom_data[0x0149] {
            0x00 => RamSize::NoRam,
            0x01 => RamSize::Unused,
            0x02 => RamSize::Size8K,
            0x03 => RamSize::Size32K,
            0x04 => RamSize::Size128K,
            0x05 => RamSize::Size64K,
            _ => RamSize::SizeUnknown,
        }
    }
}

impl Display for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name => {}\nType => {}\nROM Size => {}\nRAM Size => {}",
            self.title(),
            self.rom_type(),
            self.rom_size(),
            self.ram_size()
        )
    }
}

pub struct Mbc {
    pub name: &'static str,
    pub read_rom: fn(rom: &Cartridge, addr: u16) -> u8,
    pub write_rom: fn(rom: &mut Cartridge, addr: u16, value: u8),
    pub read_ram: fn(rom: &Cartridge, addr: u16) -> u8,
    pub write_ram: fn(rom: &mut Cartridge, addr: u16, value: u8),
}

pub static NO_MBC: Mbc = Mbc {
    name: "No MBC",
    read_rom: |rom: &Cartridge, addr: u16| -> u8 { rom.rom_data[addr as usize] },
    write_rom: |_rom: &mut Cartridge, addr: u16, _value: u8| {
        match addr {
            // ignores this address as Tetris and some other games write
            // to this address for some reason (probably MBC1 compatibility)
            0x2000 => (),
            _ => panic!("Writing in unknown Cartridge ROM location 0x{:04x}", addr),
        };
    },
    read_ram: |rom: &Cartridge, addr: u16| -> u8 { rom.ram_data[(addr & 0x1fff) as usize] },
    write_ram: |rom: &mut Cartridge, addr: u16, value: u8| {
        rom.ram_data[(addr & 0x1fff) as usize] = value;
    },
};

pub static MBC1: Mbc = Mbc {
    name: "MBC1",
    read_rom: |rom: &Cartridge, addr: u16| -> u8 {
        match addr & 0xf000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => rom.rom_data[addr as usize],
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                rom.rom_data[rom.rom_offset + (addr - 0x4000) as usize]
            }
            _ => panic!("Reading from unknown Cartridge ROM location 0x{:04x}", addr),
        }
    },
    write_rom: |rom: &mut Cartridge, addr: u16, value: u8| {
        match addr & 0xf000 {
            0x0000 | 0x1000 => {
                println!("RAM enable => {}", value);
            }
            0x2000 | 0x3000 => {
                let mut rom_bank = value & 0x1f;
                // @todo this is slow and must be pre-computed in cartridge
                rom_bank = rom_bank & (rom.rom_size().rom_banks() * 2 - 1) as u8;
                if rom_bank == 0 {
                    rom_bank = 1;
                }
                rom.set_rom_bank(rom_bank);
            }
            0x4000 | 0x5000 => {
                let ram_bank = value & 0x03;
                rom.set_ram_bank(ram_bank);
            }
            // ROM mode selection
            0x6000 | 0x7000 => {
                debugln!("SETTING MODE {}", value);
            }
            _ => panic!("Writing to unknown Cartridge ROM location 0x{:04x}", addr),
        }
    },
    read_ram: |rom: &Cartridge, addr: u16| -> u8 {
        rom.ram_data[rom.ram_offset + (addr - 0xa000) as usize]
    },
    write_ram: |rom: &mut Cartridge, addr: u16, value: u8| {
        rom.ram_data[rom.ram_offset + (addr - 0xa000) as usize] = value;
    },
};
