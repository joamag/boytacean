//! Flash chips of flash cartridges, described by their CFI (Common
//! Flash Interface) table and driven with the AMD command set.
//!
//! Flash cartridges hold a flash chip instead of a mask ROM, this module
//! only knows the commands of the chip and how its CFI table describes
//! it, not how the commands reach the cartridge.

use std::fmt::{self, Display, Formatter};

use boytacean_common::error::Error;

use crate::mapper::BusWrite;

/// The size in bytes of the CFI table read from the chip, in which
/// every value shows up twice.
pub const CFI_SIZE: usize = 256;

/// The value read from a sector of the chip once it is erased.
pub const ERASED: [u8; 2] = [0xff, 0xff];

/// The maximum time in milliseconds to erase a sector, used when the
/// CFI table of the chip does not declare it.
pub const ERASE_TIMEOUT: u32 = 4096;

/// The number of banks that the MBC5 of flash cartridges can select,
/// through the nine bits of the [`bank_writes`].
pub const MAX_BANKS: usize = 512;

/// The AMD (and Fujitsu) command set, as declared in the CFI table.
const COMMAND_SET_AMD: u16 = 0x0002;

/// The writes that reset the chip, back to reading its contents.
pub fn reset_writes() -> Vec<BusWrite> {
    vec![(0x0000, 0xf0)]
}

/// The writes that make the chip reply with its CFI table.
pub fn cfi_writes() -> Vec<BusWrite> {
    vec![(0x00aa, 0x98)]
}

/// If the provided bytes are the CFI table of a flash chip, starting
/// with `QRY`, as cartridges without one reply with their ROM instead.
pub fn is_cfi(data: &[u8]) -> bool {
    data.len() == CFI_SIZE && [data[0x20], data[0x22], data[0x24]] == *b"QRY"
}

/// The writes that erase the sector at the provided address.
pub fn sector_erase_writes(address: u16) -> Vec<BusWrite> {
    vec![
        (0x0aaa, 0xaa),
        (0x0555, 0x55),
        (0x0aaa, 0x80),
        (0x0aaa, 0xaa),
        (0x0555, 0x55),
        (address, 0x30),
    ]
}

/// The commands (address and value) of the buffered programming of the
/// chip, the zero ones being filled in by the firmware of the device.
pub fn program_commands() -> Vec<(u16, u16)> {
    vec![
        (0x0aaa, 0xaa),
        (0x0555, 0x55),
        (0x0000, 0x25),
        (0x0000, 0x00),
        (0x0000, 0x00),
        (0x0000, 0x29),
    ]
}

/// The writes that select a bank of the chip in the switchable window,
/// through the MBC5 of flash cartridges.
pub fn bank_writes(bank: usize) -> Vec<BusWrite> {
    vec![(0x3000, (bank >> 8) as u8 & 0x01), (0x2100, bank as u8)]
}

/// A flash chip, as described by its CFI table.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Chip {
    size: usize,
    buffer_size: usize,
    sectors: Vec<usize>,
    erase_timeout: u32,
}

impl Chip {
    /// Builds the chip from its CFI table, failing for cartridges
    /// without a flash chip and for chips that can't be flashed.
    pub fn from_cfi(data: &[u8]) -> Result<Self, Error> {
        if data.len() != CFI_SIZE {
            return Err(Error::DataError(format!(
                "Invalid CFI table size: {} bytes",
                data.len()
            )));
        }
        let value = |offset: usize| data[offset * 2];
        let word = |offset: usize| u16::from_le_bytes([value(offset), value(offset + 1)]);
        if !is_cfi(data) {
            return Err(Error::CustomError(String::from(
                "No flash chip found, the cartridge can't be flashed",
            )));
        }
        let command_set = word(0x13);
        if command_set != COMMAND_SET_AMD {
            return Err(Error::CustomError(format!(
                "Unsupported flash command set: 0x{command_set:04x}"
            )));
        }
        if word(0x2a) == 0 {
            return Err(Error::CustomError(String::from(
                "Unsupported flash chip, without buffered programming",
            )));
        }

        // the sizes of the chip and of its write buffer are powers of two,
        // that must fit in the address space and in the 2 byte variable of
        // the device respectively
        let size = 1usize
            .checked_shl(value(0x27) as u32)
            .ok_or(Error::DataError(String::from(
                "Invalid flash chip size in the CFI table",
            )))?;
        let buffer_size =
            1u16.checked_shl(word(0x2a) as u32)
                .ok_or(Error::DataError(String::from(
                    "Invalid write buffer size in the CFI table",
                )))?;

        // every erase region declares the number of its sectors (minus
        // one) and their size (in units of 256 bytes), all of them within
        // the table and each checked against the rest of the chip before
        // its sectors are listed
        let invalid_regions =
            || Error::DataError(String::from("Invalid erase regions in the CFI table"));
        let regions = value(0x2c) as usize;
        if 0x2d + regions * 4 > CFI_SIZE / 2 {
            return Err(invalid_regions());
        }
        let mut sectors = vec![];
        let mut remaining = size;
        for region in 0..regions {
            let offset = 0x2d + region * 4;
            let count = word(offset) as usize + 1;
            let sector_size = match word(offset + 2) {
                0 => 128,
                units => units as usize * 256,
            };
            remaining = count
                .checked_mul(sector_size)
                .and_then(|region_size| remaining.checked_sub(region_size))
                .ok_or_else(invalid_regions)?;
            sectors.extend(vec![sector_size; count]);
        }
        if remaining != 0 {
            return Err(invalid_regions());
        }

        // the typical time to erase a sector and the factor of the
        // maximum time over it, both as powers of two
        let erase_timeout = match value(0x21) {
            0 => ERASE_TIMEOUT,
            typical => 1u32
                .checked_shl(typical as u32 + value(0x25) as u32)
                .ok_or(Error::DataError(String::from(
                    "Invalid erase timeout in the CFI table",
                )))?,
        };

        Ok(Self {
            size,
            buffer_size: buffer_size as usize,
            sectors,
            erase_timeout,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn sectors(&self) -> &[usize] {
        &self.sectors
    }

    /// The maximum time in milliseconds to erase a sector.
    pub fn erase_timeout(&self) -> u32 {
        self.erase_timeout
    }

    /// The addresses of the sectors that start in the provided range of
    /// addresses, so that a sector covering several banks is only erased
    /// from the first of them.
    pub fn sectors_in(&self, start: usize, end: usize) -> Vec<usize> {
        let mut address = 0;
        let mut addresses = vec![];
        for sector_size in &self.sectors {
            if address >= start && address < end {
                addresses.push(address);
            }
            address += sector_size;
        }
        addresses
    }
}

impl Display for Chip {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} KiB flash chip ({} sectors, {} byte write buffer)",
            self.size / 1024,
            self.sectors.len(),
            self.buffer_size
        )
    }
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{
        bank_writes, cfi_writes, is_cfi, program_commands, reset_writes, sector_erase_writes, Chip,
        CFI_SIZE, ERASE_TIMEOUT,
    };

    /// The CFI values from offset `0x10` of the 4 MiB flash chip of a
    /// flash cartridge (CFI ID `01 7E`), as captured from the device.
    const CFI_4MB: [u8; 37] = [
        0x51, 0x52, 0x59, 0x02, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, 0x36, 0x00, 0x00,
        0x03, 0x04, 0x08, 0x0e, 0x03, 0x05, 0x03, 0x03, 0x16, 0x02, 0x00, 0x05, 0x00, 0x02, 0x07,
        0x00, 0x20, 0x00, 0x3e, 0x00, 0x00, 0x01,
    ];

    /// Builds the CFI table as read from the device, with every value
    /// showing up twice, from the provided values from offset `0x10`.
    fn build_cfi(values: &[u8]) -> Vec<u8> {
        let mut data = vec![0x00; CFI_SIZE];
        for (offset, value) in values.iter().enumerate() {
            data[(0x10 + offset) * 2] = *value;
            data[(0x10 + offset) * 2 + 1] = *value;
        }
        data
    }

    #[test]
    fn test_reset_writes() {
        assert_eq!(reset_writes(), vec![(0x0000, 0xf0)]);
    }

    #[test]
    fn test_cfi_writes() {
        assert_eq!(cfi_writes(), vec![(0x00aa, 0x98)]);
    }

    #[test]
    fn test_is_cfi() {
        assert!(is_cfi(&build_cfi(&CFI_4MB)));
        let mut values = CFI_4MB;
        values[2] = 0x58;
        assert!(!is_cfi(&build_cfi(&values)));
        assert!(!is_cfi(&[0xff; CFI_SIZE]));
        assert!(!is_cfi(&[0x00; CFI_SIZE]));
        assert!(!is_cfi(&build_cfi(&CFI_4MB)[..CFI_SIZE - 1]));
        assert!(!is_cfi(&[]));
    }

    #[test]
    fn test_sector_erase_writes() {
        assert_eq!(
            sector_erase_writes(0x6000),
            vec![
                (0x0aaa, 0xaa),
                (0x0555, 0x55),
                (0x0aaa, 0x80),
                (0x0aaa, 0xaa),
                (0x0555, 0x55),
                (0x6000, 0x30)
            ]
        );
    }

    #[test]
    fn test_program_commands() {
        assert_eq!(
            program_commands(),
            vec![
                (0x0aaa, 0xaa),
                (0x0555, 0x55),
                (0x0000, 0x25),
                (0x0000, 0x00),
                (0x0000, 0x00),
                (0x0000, 0x29)
            ]
        );
    }

    #[test]
    fn test_bank_writes() {
        assert_eq!(bank_writes(0x00), vec![(0x3000, 0x00), (0x2100, 0x00)]);
        assert_eq!(bank_writes(0xfc), vec![(0x3000, 0x00), (0x2100, 0xfc)]);
        assert_eq!(bank_writes(0x101), vec![(0x3000, 0x01), (0x2100, 0x01)]);
        assert_eq!(bank_writes(0x1ff), vec![(0x3000, 0x01), (0x2100, 0xff)]);
    }

    #[test]
    fn test_chip_from_cfi() {
        let chip = Chip::from_cfi(&build_cfi(&CFI_4MB)).unwrap();
        assert_eq!(chip.size(), 4 * 1024 * 1024);
        assert_eq!(chip.buffer_size(), 32);
        assert_eq!(chip.sectors().len(), 71);
        assert_eq!(&chip.sectors()[..8], &[0x2000; 8]);
        assert_eq!(chip.sectors()[8], 0x10000);
        assert_eq!(chip.sectors()[70], 0x10000);
        assert_eq!(chip.erase_timeout(), 2048);
    }

    #[test]
    fn test_chip_from_cfi_size() {
        assert_eq!(
            Chip::from_cfi(&[0x00; CFI_SIZE - 1]),
            Err(Error::DataError(String::from(
                "Invalid CFI table size: 255 bytes"
            )))
        );
    }

    #[test]
    fn test_chip_from_cfi_no_chip() {
        // a cartridge without a flash chip replies with its ROM, as
        // its MBC ignores the CFI query
        let mut data = vec![0xff; CFI_SIZE];
        data[0x20..0x26].copy_from_slice(&[0x51, 0x51, 0x52, 0x52, 0x58, 0x58]);
        for data in [vec![0xff; CFI_SIZE], vec![0x00; CFI_SIZE], data] {
            assert_eq!(
                Chip::from_cfi(&data),
                Err(Error::CustomError(String::from(
                    "No flash chip found, the cartridge can't be flashed"
                )))
            );
        }
    }

    #[test]
    fn test_chip_from_cfi_command_set() {
        let mut values = CFI_4MB;
        values[3] = 0x01;
        assert_eq!(
            Chip::from_cfi(&build_cfi(&values)),
            Err(Error::CustomError(String::from(
                "Unsupported flash command set: 0x0001"
            )))
        );
    }

    #[test]
    fn test_chip_from_cfi_no_buffer() {
        let mut values = CFI_4MB;
        values[0x1a] = 0x00;
        assert_eq!(
            Chip::from_cfi(&build_cfi(&values)),
            Err(Error::CustomError(String::from(
                "Unsupported flash chip, without buffered programming"
            )))
        );
    }

    #[test]
    fn test_chip_from_cfi_sizes() {
        // the size of the chip must fit in the address space and the size
        // of its write buffer in 2 bytes, the largest buffer included
        for (index, value, message) in [
            (0x17, 0x40, "Invalid flash chip size in the CFI table"),
            (0x17, 0xff, "Invalid flash chip size in the CFI table"),
            (0x1a, 0x10, "Invalid write buffer size in the CFI table"),
            (0x1b, 0x01, "Invalid write buffer size in the CFI table"),
        ] {
            let mut values = CFI_4MB;
            values[index] = value;
            assert_eq!(
                Chip::from_cfi(&build_cfi(&values)),
                Err(Error::DataError(String::from(message)))
            );
        }
        let mut values = CFI_4MB;
        values[0x1a] = 0x0f;
        let chip = Chip::from_cfi(&build_cfi(&values)).unwrap();
        assert_eq!(chip.buffer_size(), 0x8000);
    }

    #[test]
    fn test_chip_from_cfi_regions() {
        // the sectors of the regions must cover the whole chip, either
        // short of it (a missing region) or past it (a wrong count)
        for (index, value) in [(0x1c, 0x01), (0x1c, 0x00), (0x1d, 0x08), (0x17, 0x17)] {
            let mut values = CFI_4MB;
            values[index] = value;
            assert_eq!(
                Chip::from_cfi(&build_cfi(&values)),
                Err(Error::DataError(String::from(
                    "Invalid erase regions in the CFI table"
                )))
            );
        }

        // as does a region past the rest of the chip (of 65536 sectors of
        // 64 KiB, 4 GiB) once the others cover it, never listing its sectors
        let mut values = CFI_4MB.to_vec();
        values[0x1c] = 0x03;
        values.extend([0xff, 0xff, 0x00, 0x01]);
        assert_eq!(
            Chip::from_cfi(&build_cfi(&values)),
            Err(Error::DataError(String::from(
                "Invalid erase regions in the CFI table"
            )))
        );
    }

    #[test]
    fn test_chip_from_cfi_regions_bounds() {
        // a region count past the end of the table is not read past it
        let mut values = CFI_4MB;
        values[0x1c] = 0xff;
        assert_eq!(
            Chip::from_cfi(&build_cfi(&values)),
            Err(Error::DataError(String::from(
                "Invalid erase regions in the CFI table"
            )))
        );

        // even when the regions within the table cover the whole chip, as
        // 20 regions do (32749 sectors of 128 bytes and 19 of one), unlike
        // the 21 they would be declared as
        let mut values = CFI_4MB.to_vec();
        values.resize(CFI_SIZE / 2 - 0x10, 0x00);
        values[0x1d..0x25].copy_from_slice(&[0xec, 0x7f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        values[0x1c] = 20;
        let chip = Chip::from_cfi(&build_cfi(&values)).unwrap();
        assert_eq!(chip.sectors(), &[128; 32768]);
        values[0x1c] = 21;
        assert_eq!(
            Chip::from_cfi(&build_cfi(&values)),
            Err(Error::DataError(String::from(
                "Invalid erase regions in the CFI table"
            )))
        );
    }

    #[test]
    fn test_chip_from_cfi_small_sectors() {
        // a sector size of zero units stands for 128 bytes
        let mut values = CFI_4MB;
        values[0x17] = 0x0f;
        values[0x1c] = 0x01;
        values[0x1d] = 0xff;
        values[0x1e] = 0x00;
        values[0x1f] = 0x00;
        values[0x20] = 0x00;
        let chip = Chip::from_cfi(&build_cfi(&values)).unwrap();
        assert_eq!(chip.size(), 128 * 256);
        assert_eq!(chip.sectors(), &[128; 256]);
    }

    #[test]
    fn test_chip_from_cfi_erase_timeout() {
        let mut values = CFI_4MB;
        values[0x11] = 0x00;
        let chip = Chip::from_cfi(&build_cfi(&values)).unwrap();
        assert_eq!(chip.erase_timeout(), ERASE_TIMEOUT);

        // the timeout must fit in 32 bits, never wrapping around to zero
        values[0x11] = 0x1e;
        values[0x15] = 0x01;
        let chip = Chip::from_cfi(&build_cfi(&values)).unwrap();
        assert_eq!(chip.erase_timeout(), 1 << 31);
        for (typical, max) in [(0x1f, 0x01), (0x20, 0x00), (0xff, 0xff)] {
            values[0x11] = typical;
            values[0x15] = max;
            assert_eq!(
                Chip::from_cfi(&build_cfi(&values)),
                Err(Error::DataError(String::from(
                    "Invalid erase timeout in the CFI table"
                )))
            );
        }
    }

    #[test]
    fn test_chip_sectors_in() {
        let chip = Chip::from_cfi(&build_cfi(&CFI_4MB)).unwrap();
        assert_eq!(chip.sectors_in(0x0000, 0x4000), vec![0x0000, 0x2000]);
        assert_eq!(chip.sectors_in(0x4000, 0x8000), vec![0x4000, 0x6000]);
        assert_eq!(chip.sectors_in(0x10000, 0x14000), vec![0x10000]);
        assert_eq!(chip.sectors_in(0x14000, 0x18000), Vec::<usize>::new());
        assert_eq!(chip.sectors_in(0x3fc000, 0x400000), Vec::<usize>::new());
        assert_eq!(chip.sectors_in(0x3f0000, 0x3f4000), vec![0x3f0000]);
        assert_eq!(chip.sectors_in(0x400000, 0x404000), Vec::<usize>::new());
    }

    #[test]
    fn test_chip_display() {
        let chip = Chip::from_cfi(&build_cfi(&CFI_4MB)).unwrap();
        assert_eq!(
            format!("{chip}"),
            "4096 KiB flash chip (71 sectors, 32 byte write buffer)"
        );
    }
}
