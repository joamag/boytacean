//! Bank switching of the Memory Bank Controllers (MBCs) of the
//! cartridges that can be dumped.
//!
//! Dumping a cartridge means driving its own MBC with writes in the
//! cartridge bus, this module only knows which writes select a ROM or
//! a RAM bank and where that bank can then be read, not how the writes
//! reach the cartridge.

use boytacean_common::error::Error;

use crate::header::RAM_BANK_SIZE;

/// The address at which the switchable ROM bank can be read.
pub const ROM_BANK_ADDRESS: u16 = 0x4000;

/// The address at which the (switchable) RAM bank can be read.
pub const RAM_BANK_ADDRESS: u16 = 0xa000;

/// The size in bytes of the RAM that is built into the MBC2, made
/// of 512 half bytes (nibbles).
pub const MBC2_RAM_SIZE: usize = 512;

/// A write in the cartridge bus, as an address and a value.
pub type BusWrite = (u16, u8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mapper {
    NoMbc,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

impl Mapper {
    /// Obtains the mapper for the cartridge type byte of the header,
    /// failing for the mappers that can't be dumped.
    pub fn from_cartridge_type(value: u8) -> Result<Self, Error> {
        match value {
            0x00 | 0x08 | 0x09 => Ok(Mapper::NoMbc),
            0x01..=0x03 => Ok(Mapper::Mbc1),
            0x05 | 0x06 => Ok(Mapper::Mbc2),
            0x0f..=0x13 => Ok(Mapper::Mbc3),
            0x19..=0x1e => Ok(Mapper::Mbc5),
            _ => Err(Error::CustomError(format!(
                "Unsupported cartridge type: 0x{value:02x}"
            ))),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Mapper::NoMbc => "No MBC",
            Mapper::Mbc1 => "MBC1",
            Mapper::Mbc2 => "MBC2",
            Mapper::Mbc3 => "MBC3",
            Mapper::Mbc5 => "MBC5",
        }
    }

    /// The writes that select the provided ROM bank, so that it can
    /// then be read at [`Mapper::rom_bank_address`].
    pub fn rom_bank_writes(&self, bank: u16) -> Vec<BusWrite> {
        match self {
            Mapper::NoMbc => vec![],
            // the mode 1 makes the upper bits also select the bank
            // seen at 0x0000-0x3FFF, the only way to reach the banks
            // 0x20, 0x40 and 0x60 of the larger cartridges
            Mapper::Mbc1 => vec![
                (0x6000, 0x01),
                (0x4000, ((bank >> 5) & 0x03) as u8),
                (0x2000, (bank & 0x1f) as u8),
            ],
            Mapper::Mbc2 => vec![(0x2100, (bank & 0x0f) as u8)],
            // uses the 8 bits of the MBC30, which reaches 256 banks
            Mapper::Mbc3 => vec![(0x2000, (bank & 0xff) as u8)],
            Mapper::Mbc5 => vec![
                (0x2000, (bank & 0xff) as u8),
                (0x3000, ((bank >> 8) & 0x01) as u8),
            ],
        }
    }

    /// The address at which the provided ROM bank can be read, once
    /// selected with the writes from [`Mapper::rom_bank_writes`].
    pub fn rom_bank_address(&self, bank: u16) -> u16 {
        match self {
            _ if bank == 0 => 0x0000,
            Mapper::Mbc1 if bank & 0x1f == 0 => 0x0000,
            _ => ROM_BANK_ADDRESS,
        }
    }

    /// The writes that enable the access to the RAM of the cartridge.
    pub fn ram_enable_writes(&self) -> Vec<BusWrite> {
        match self {
            Mapper::NoMbc => vec![],
            Mapper::Mbc1 => vec![(0x6000, 0x01), (0x0000, 0x0a)],
            Mapper::Mbc2 | Mapper::Mbc3 | Mapper::Mbc5 => vec![(0x0000, 0x0a)],
        }
    }

    /// The writes that select the provided RAM bank, so that it can
    /// then be read at [`RAM_BANK_ADDRESS`].
    pub fn ram_bank_writes(&self, bank: u8) -> Vec<BusWrite> {
        match self {
            Mapper::NoMbc | Mapper::Mbc2 => vec![],
            Mapper::Mbc1 => vec![(0x4000, bank & 0x03)],
            // uses the 8 banks of the MBC30, 0x08-0x0C are the RTC registers
            Mapper::Mbc3 => vec![(0x4000, bank & 0x07)],
            Mapper::Mbc5 => vec![(0x4000, bank & 0x0f)],
        }
    }

    /// The writes that disable the access to the RAM of the cartridge,
    /// leaving the MBC in its power on state.
    pub fn ram_disable_writes(&self) -> Vec<BusWrite> {
        match self {
            Mapper::NoMbc => vec![],
            Mapper::Mbc1 => vec![(0x0000, 0x00), (0x6000, 0x00)],
            Mapper::Mbc2 | Mapper::Mbc3 | Mapper::Mbc5 => vec![(0x0000, 0x00)],
        }
    }

    /// The size in bytes of the RAM to be read, which for the MBC2
    /// is its built-in RAM, whatever the header declares.
    pub fn ram_size(&self, header_ram_size: usize) -> usize {
        match self {
            Mapper::Mbc2 => MBC2_RAM_SIZE,
            _ => header_ram_size,
        }
    }

    /// The number of RAM banks to be read for the provided RAM size,
    /// a RAM smaller than a bank still takes one.
    pub fn ram_banks(&self, ram_size: usize) -> u8 {
        ram_size.div_ceil(RAM_BANK_SIZE) as u8
    }

    /// The number of ROM banks that the bank switching of the mapper
    /// can address, a header declaring more can't be trusted.
    pub fn max_rom_banks(&self) -> u16 {
        match self {
            Mapper::NoMbc => 2,
            Mapper::Mbc1 => 128,
            Mapper::Mbc2 => 16,
            Mapper::Mbc3 => 256,
            Mapper::Mbc5 => 512,
        }
    }

    /// The number of RAM banks that the bank switching of the mapper
    /// can address, a header declaring more can't be trusted.
    pub fn max_ram_banks(&self) -> u8 {
        match self {
            Mapper::NoMbc | Mapper::Mbc2 => 1,
            Mapper::Mbc1 => 4,
            Mapper::Mbc3 => 8,
            Mapper::Mbc5 => 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{Mapper, MBC2_RAM_SIZE, ROM_BANK_ADDRESS};

    #[test]
    fn test_from_cartridge_type() {
        for (value, mapper) in [
            (0x00, Mapper::NoMbc),
            (0x08, Mapper::NoMbc),
            (0x09, Mapper::NoMbc),
            (0x01, Mapper::Mbc1),
            (0x02, Mapper::Mbc1),
            (0x03, Mapper::Mbc1),
            (0x05, Mapper::Mbc2),
            (0x06, Mapper::Mbc2),
            (0x0f, Mapper::Mbc3),
            (0x10, Mapper::Mbc3),
            (0x11, Mapper::Mbc3),
            (0x12, Mapper::Mbc3),
            (0x13, Mapper::Mbc3),
            (0x19, Mapper::Mbc5),
            (0x1b, Mapper::Mbc5),
            (0x1e, Mapper::Mbc5),
        ] {
            assert_eq!(Mapper::from_cartridge_type(value), Ok(mapper));
        }
    }

    #[test]
    fn test_from_cartridge_type_unsupported() {
        for value in [0x04, 0x0b, 0x0d, 0x14, 0x20, 0x22, 0xfc, 0xfd, 0xfe, 0xff] {
            assert_eq!(
                Mapper::from_cartridge_type(value),
                Err(Error::CustomError(format!(
                    "Unsupported cartridge type: 0x{value:02x}"
                )))
            );
        }
    }

    #[test]
    fn test_description() {
        assert_eq!(Mapper::NoMbc.description(), "No MBC");
        assert_eq!(Mapper::Mbc1.description(), "MBC1");
        assert_eq!(Mapper::Mbc2.description(), "MBC2");
        assert_eq!(Mapper::Mbc3.description(), "MBC3");
        assert_eq!(Mapper::Mbc5.description(), "MBC5");
    }

    #[test]
    fn test_rom_bank_writes_no_mbc() {
        assert_eq!(Mapper::NoMbc.rom_bank_writes(1), vec![]);
    }

    #[test]
    fn test_rom_bank_writes_mbc1() {
        assert_eq!(
            Mapper::Mbc1.rom_bank_writes(0x01),
            vec![(0x6000, 0x01), (0x4000, 0x00), (0x2000, 0x01)]
        );
        assert_eq!(
            Mapper::Mbc1.rom_bank_writes(0x1f),
            vec![(0x6000, 0x01), (0x4000, 0x00), (0x2000, 0x1f)]
        );
        assert_eq!(
            Mapper::Mbc1.rom_bank_writes(0x21),
            vec![(0x6000, 0x01), (0x4000, 0x01), (0x2000, 0x01)]
        );
        assert_eq!(
            Mapper::Mbc1.rom_bank_writes(0x7f),
            vec![(0x6000, 0x01), (0x4000, 0x03), (0x2000, 0x1f)]
        );
    }

    #[test]
    fn test_rom_bank_writes_mbc1_upper_banks() {
        for (bank, upper) in [(0x20, 0x01), (0x40, 0x02), (0x60, 0x03)] {
            assert_eq!(
                Mapper::Mbc1.rom_bank_writes(bank),
                vec![(0x6000, 0x01), (0x4000, upper), (0x2000, 0x00)]
            );
        }
    }

    #[test]
    fn test_rom_bank_writes_mbc2() {
        assert_eq!(Mapper::Mbc2.rom_bank_writes(0x0f), vec![(0x2100, 0x0f)]);
    }

    #[test]
    fn test_rom_bank_writes_mbc3() {
        assert_eq!(Mapper::Mbc3.rom_bank_writes(0x7f), vec![(0x2000, 0x7f)]);
    }

    #[test]
    fn test_rom_bank_writes_mbc30() {
        assert_eq!(Mapper::Mbc3.rom_bank_writes(0x80), vec![(0x2000, 0x80)]);
        assert_eq!(Mapper::Mbc3.rom_bank_writes(0xff), vec![(0x2000, 0xff)]);
    }

    #[test]
    fn test_rom_bank_writes_mbc5() {
        assert_eq!(
            Mapper::Mbc5.rom_bank_writes(0x0ff),
            vec![(0x2000, 0xff), (0x3000, 0x00)]
        );
        assert_eq!(
            Mapper::Mbc5.rom_bank_writes(0x100),
            vec![(0x2000, 0x00), (0x3000, 0x01)]
        );
        assert_eq!(
            Mapper::Mbc5.rom_bank_writes(0x1ff),
            vec![(0x2000, 0xff), (0x3000, 0x01)]
        );
    }

    #[test]
    fn test_rom_bank_address() {
        for mapper in [
            Mapper::NoMbc,
            Mapper::Mbc1,
            Mapper::Mbc2,
            Mapper::Mbc3,
            Mapper::Mbc5,
        ] {
            assert_eq!(mapper.rom_bank_address(0), 0x0000);
            assert_eq!(mapper.rom_bank_address(1), ROM_BANK_ADDRESS);
        }
    }

    #[test]
    fn test_rom_bank_address_mbc1_upper_banks() {
        assert_eq!(Mapper::Mbc1.rom_bank_address(0x20), 0x0000);
        assert_eq!(Mapper::Mbc1.rom_bank_address(0x40), 0x0000);
        assert_eq!(Mapper::Mbc1.rom_bank_address(0x60), 0x0000);
        assert_eq!(Mapper::Mbc1.rom_bank_address(0x21), ROM_BANK_ADDRESS);
    }

    #[test]
    fn test_rom_bank_address_mbc5_upper_banks() {
        assert_eq!(Mapper::Mbc5.rom_bank_address(0x20), ROM_BANK_ADDRESS);
        assert_eq!(Mapper::Mbc5.rom_bank_address(0x100), ROM_BANK_ADDRESS);
    }

    #[test]
    fn test_ram_enable_writes() {
        assert_eq!(Mapper::NoMbc.ram_enable_writes(), vec![]);
        assert_eq!(
            Mapper::Mbc1.ram_enable_writes(),
            vec![(0x6000, 0x01), (0x0000, 0x0a)]
        );
        assert_eq!(Mapper::Mbc2.ram_enable_writes(), vec![(0x0000, 0x0a)]);
        assert_eq!(Mapper::Mbc3.ram_enable_writes(), vec![(0x0000, 0x0a)]);
        assert_eq!(Mapper::Mbc5.ram_enable_writes(), vec![(0x0000, 0x0a)]);
    }

    #[test]
    fn test_ram_bank_writes() {
        assert_eq!(Mapper::NoMbc.ram_bank_writes(0), vec![]);
        assert_eq!(Mapper::Mbc1.ram_bank_writes(3), vec![(0x4000, 0x03)]);
        assert_eq!(Mapper::Mbc2.ram_bank_writes(0), vec![]);
        assert_eq!(Mapper::Mbc3.ram_bank_writes(3), vec![(0x4000, 0x03)]);
        assert_eq!(Mapper::Mbc5.ram_bank_writes(15), vec![(0x4000, 0x0f)]);
    }

    #[test]
    fn test_ram_bank_writes_mbc30() {
        assert_eq!(Mapper::Mbc3.ram_bank_writes(4), vec![(0x4000, 0x04)]);
        assert_eq!(Mapper::Mbc3.ram_bank_writes(7), vec![(0x4000, 0x07)]);
    }

    #[test]
    fn test_ram_bank_writes_masked() {
        assert_eq!(Mapper::Mbc1.ram_bank_writes(4), vec![(0x4000, 0x00)]);
        assert_eq!(Mapper::Mbc3.ram_bank_writes(0x08), vec![(0x4000, 0x00)]);
        assert_eq!(Mapper::Mbc3.ram_bank_writes(0x0c), vec![(0x4000, 0x04)]);
        assert_eq!(Mapper::Mbc5.ram_bank_writes(16), vec![(0x4000, 0x00)]);
    }

    #[test]
    fn test_ram_disable_writes() {
        assert_eq!(Mapper::NoMbc.ram_disable_writes(), vec![]);
        assert_eq!(
            Mapper::Mbc1.ram_disable_writes(),
            vec![(0x0000, 0x00), (0x6000, 0x00)]
        );
        assert_eq!(Mapper::Mbc2.ram_disable_writes(), vec![(0x0000, 0x00)]);
        assert_eq!(Mapper::Mbc3.ram_disable_writes(), vec![(0x0000, 0x00)]);
        assert_eq!(Mapper::Mbc5.ram_disable_writes(), vec![(0x0000, 0x00)]);
    }

    #[test]
    fn test_ram_size() {
        assert_eq!(Mapper::Mbc1.ram_size(32768), 32768);
        assert_eq!(Mapper::Mbc5.ram_size(0), 0);
    }

    #[test]
    fn test_ram_size_mbc2() {
        assert_eq!(Mapper::Mbc2.ram_size(0), MBC2_RAM_SIZE);
        assert_eq!(Mapper::Mbc2.ram_size(8192), MBC2_RAM_SIZE);
    }

    #[test]
    fn test_ram_banks() {
        assert_eq!(Mapper::Mbc1.ram_banks(0), 0);
        assert_eq!(Mapper::Mbc2.ram_banks(512), 1);
        assert_eq!(Mapper::Mbc1.ram_banks(2048), 1);
        assert_eq!(Mapper::Mbc1.ram_banks(8192), 1);
        assert_eq!(Mapper::Mbc3.ram_banks(32768), 4);
        assert_eq!(Mapper::Mbc5.ram_banks(131072), 16);
    }

    #[test]
    fn test_max_rom_banks() {
        assert_eq!(Mapper::NoMbc.max_rom_banks(), 2);
        assert_eq!(Mapper::Mbc1.max_rom_banks(), 128);
        assert_eq!(Mapper::Mbc2.max_rom_banks(), 16);
        assert_eq!(Mapper::Mbc3.max_rom_banks(), 256);
        assert_eq!(Mapper::Mbc5.max_rom_banks(), 512);
    }

    #[test]
    fn test_max_ram_banks() {
        assert_eq!(Mapper::NoMbc.max_ram_banks(), 1);
        assert_eq!(Mapper::Mbc1.max_ram_banks(), 4);
        assert_eq!(Mapper::Mbc2.max_ram_banks(), 1);
        assert_eq!(Mapper::Mbc3.max_ram_banks(), 8);
        assert_eq!(Mapper::Mbc5.max_ram_banks(), 16);
    }
}
