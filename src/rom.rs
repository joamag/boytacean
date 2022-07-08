use core::fmt;
use std::fmt::{Display, Formatter};

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

pub enum RomSize {
    Size32K = 32,
    Size64K = 64,
    Size128K = 128,
    SizeUnknown = 0,
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

    pub fn size(&self) -> RomSize {
        match self.data[0x0148] {
            0x00 => RomSize::Size32K,
            0x01 => RomSize::Size64K,
            0x02 => RomSize::Size128K,
            _ => RomSize::SizeUnknown,
        }
    }
}

impl Display for Rom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name => {}\nType => {}\nSize => {}",
            self.title(),
            self.rom_type() as u8,
            self.size() as u32
        )
    }
}
