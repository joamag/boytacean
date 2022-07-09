use core::fmt;
use std::fmt::{Display, Formatter};

pub const BANK_SIZE: usize = 16384;

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

pub enum RomSize {
    Size32K,
    Size64K,
    Size128K,
    Size256K,
    Size512K,
    Size1M,
    Size2M,
    SizeUnknown,
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
            RomSize::SizeUnknown => "Unknown",
        };
        write!(f, "{}", str)
    }
}

pub enum RamSize {
    NoRam,
    Unused,
    Size8K,
    Size32K,
    Size64K,
    Size128K,
    SizeUnknown,
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

pub struct Cartridge {
    /// The complete data of the ROM cartridge, should
    /// include the complete set o ROM banks.
    data: Vec<u8>,

    /// The offset address to the ROM bank (#1) that is
    /// currently in use by the ROM cartridge.
    rom_offset: u16,

    /// The MBC (Memory Bank Controller) to be used for
    /// RAM and ROM access on the current cartridge.
    mbc: &'static Mbc,
}

impl Cartridge {
    pub fn new() -> Self {
        Self {
            data: vec![],
            rom_offset: 0x0000,
            mbc: &NO_MBC,
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        let mut cartridge = Cartridge::new();
        cartridge.set_data(data);
        cartridge
    }

    pub fn read(&self, addr: u16) -> u8 {
        (self.mbc.read)(self, addr)
    }

    pub fn write(&self, addr: u16, value: u8) {
        (self.mbc.write)(self, addr, value)
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn get_bank(&self, index: u8) -> &[u8] {
        let start = index as usize * BANK_SIZE;
        let end = (index + 1) as usize * BANK_SIZE;
        &self.data[start..end]
    }

    pub fn title(&self) -> &str {
        std::str::from_utf8(&self.data[0x0134..0x0143]).unwrap()
    }

    pub fn rom_type(&self) -> RomType {
        match self.data[0x0147] {
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
        match self.data[0x0148] {
            0x00 => RomSize::Size32K,
            0x01 => RomSize::Size64K,
            0x02 => RomSize::Size128K,
            0x03 => RomSize::Size256K,
            0x04 => RomSize::Size512K,
            0x05 => RomSize::Size1M,
            0x06 => RomSize::Size2M,
            _ => RomSize::SizeUnknown,
        }
    }

    pub fn ram_size(&self) -> RamSize {
        match self.data[0x0148] {
            0x00 => RamSize::NoRam,
            0x01 => RamSize::Unused,
            0x02 => RamSize::Size8K,
            0x03 => RamSize::Size32K,
            0x04 => RamSize::Size128K,
            0x05 => RamSize::Size64K,
            _ => RamSize::SizeUnknown,
        }
    }

    fn set_data(&mut self, data: &[u8]) {
        self.data = data.to_vec();
        self.rom_offset = 0x0000;
        self.set_mbc();
    }

    fn set_mbc(&self) -> &'static Mbc {
        match self.rom_type() {
            RomType::RomOnly => &NO_MBC,
            RomType::Mbc1 => &MBC1,
            RomType::Mbc1Ram => &MBC1,
            RomType::Mbc1RamBattery => &MBC1,
            _ => &NO_MBC,
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
    pub read: fn(rom: &Cartridge, addr: u16) -> u8,
    pub write: fn(rom: &Cartridge, addr: u16, value: u8),
}

pub static NO_MBC: Mbc = Mbc {
    name: "No MBC",
    read: |rom: &Cartridge, addr: u16| -> u8 { rom.data[addr as usize] },
    write: |_rom: &Cartridge, addr: u16, _value: u8| {
        println!("Writing to ROM at 0x{:04x}", addr);
    },
};

pub static MBC1: Mbc = Mbc {
    name: "MBC1",
    read: |rom: &Cartridge, addr: u16| -> u8 { 0x00 },
    write: |rom: &Cartridge, addr: u16, value: u8| {},
};
