use core::fmt;
use std::fmt::{Display, Formatter};

pub const BANK_SIZE: usize = 16384;

pub struct Rom {
    data: Vec<u8>,
}
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

impl Rom {
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
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
}

impl Display for Rom {
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
