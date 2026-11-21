//! Cartridge header parsing, limited to what is needed to dump a cartridge.
//!
//! The header lives at `0x0100` to `0x014F` of the first ROM bank and
//! describes the title, the cartridge (mapper) type and the size of
//! both the ROM and the RAM of the cartridge.

use boytacean_common::error::Error;

/// The address of the first byte of the header in the cartridge.
pub const HEADER_ADDRESS: u16 = 0x0100;

/// The size in bytes of the header.
pub const HEADER_SIZE: usize = 0x50;

/// The size in bytes of a ROM bank.
pub const ROM_BANK_SIZE: usize = 16384;

/// The size in bytes of a RAM bank.
pub const RAM_BANK_SIZE: usize = 8192;

/// The Nintendo logo, that every cartridge holds at `0x0104` to
/// `0x0133` for the boot ROM to check it.
pub const NINTENDO_LOGO: [u8; 48] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d,
    0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99,
    0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];

pub struct Header {
    data: [u8; HEADER_SIZE],
}

impl Header {
    /// Builds a header from the bytes read from the cartridge,
    /// starting at [`HEADER_ADDRESS`].
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() != HEADER_SIZE {
            return Err(Error::DataError(format!(
                "Invalid header size: {} bytes",
                data.len()
            )));
        }
        let mut header = Self {
            data: [0x00; HEADER_SIZE],
        };
        header.data.copy_from_slice(data);
        Ok(header)
    }

    /// The title of the game, which is shorter for CGB games as
    /// its last byte is used as the CGB flag.
    pub fn title(&self) -> String {
        let end = if self.data[0x43] & 0x80 == 0x80 {
            0x43
        } else {
            0x44
        };
        self.data[0x34..end]
            .iter()
            .take_while(|&&byte| byte != 0x00)
            .map(|&byte| byte as char)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    pub fn cartridge_type(&self) -> u8 {
        self.data[0x47]
    }

    /// The number of ROM banks of the cartridge, as declared
    /// by the ROM size byte of the header.
    pub fn rom_banks(&self) -> Result<u16, Error> {
        match self.data[0x48] {
            value @ 0x00..=0x08 => Ok(2 << value),
            _ => Err(Error::RomSize),
        }
    }

    /// The size in bytes of the RAM of the cartridge, as declared
    /// by the RAM size byte of the header.
    pub fn ram_size(&self) -> Result<usize, Error> {
        match self.data[0x49] {
            0x00 => Ok(0),
            0x01 => Ok(2048),
            0x02 => Ok(RAM_BANK_SIZE),
            0x03 => Ok(RAM_BANK_SIZE * 4),
            0x04 => Ok(RAM_BANK_SIZE * 16),
            0x05 => Ok(RAM_BANK_SIZE * 8),
            value => Err(Error::DataError(format!("Invalid RAM size: 0x{value:02x}"))),
        }
    }

    /// If the RAM (if any) of the cartridge keeps its contents
    /// using a battery, meaning that it holds a save.
    pub fn has_battery(&self) -> bool {
        matches!(
            self.cartridge_type(),
            0x03 | 0x06 | 0x09 | 0x0d | 0x0f | 0x10 | 0x13 | 0x1b | 0x1e | 0x22 | 0xff
        )
    }

    /// If the cartridge has a rumble motor, which takes the highest bit
    /// of the RAM bank number of its MBC5.
    pub fn has_rumble(&self) -> bool {
        matches!(self.cartridge_type(), 0x1c..=0x1e)
    }

    /// Checks the Nintendo logo and the header checksum, which fail when
    /// there's no cartridge or when the connection to it is not stable.
    pub fn is_valid(&self) -> bool {
        let checksum = self.data[0x34..0x4d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        self.data[0x04..0x34] == NINTENDO_LOGO && checksum == self.data[0x4d]
    }

    pub fn global_checksum(&self) -> u16 {
        u16::from_be_bytes([self.data[0x4e], self.data[0x4f]])
    }

    /// Checks the global checksum of the header against the complete
    /// ROM, which is often wrong in reproduction cartridges.
    pub fn matches_rom(&self, rom: &[u8]) -> bool {
        let checksum = rom
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 0x014e && *index != 0x014f)
            .fold(0u16, |sum, (_, &byte)| sum.wrapping_add(byte as u16));
        checksum == self.global_checksum()
    }
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{Header, HEADER_SIZE, NINTENDO_LOGO};

    /// Builds a header with the provided title, cartridge type and
    /// ROM and RAM size bytes, with a valid header checksum.
    fn build_header(title: &[u8], cartridge_type: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
        let mut data = vec![0x00; HEADER_SIZE];
        data[0x04..0x34].copy_from_slice(&NINTENDO_LOGO);
        data[0x34..0x34 + title.len()].copy_from_slice(title);
        data[0x47] = cartridge_type;
        data[0x48] = rom_size;
        data[0x49] = ram_size;
        data[0x4d] = data[0x34..0x4d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        data
    }

    #[test]
    fn test_from_data() {
        let data = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "TETRIS");
    }

    #[test]
    fn test_from_data_short() {
        let result = Header::from_data(&[0x00; HEADER_SIZE - 1]);
        assert_eq!(
            result.err(),
            Some(Error::DataError(String::from(
                "Invalid header size: 79 bytes"
            )))
        );
    }

    #[test]
    fn test_from_data_long() {
        let result = Header::from_data(&[0x00; HEADER_SIZE + 1]);
        assert_eq!(
            result.err(),
            Some(Error::DataError(String::from(
                "Invalid header size: 81 bytes"
            )))
        );
    }

    #[test]
    fn test_title_full() {
        let data = build_header(b"POKEMON RED\0\0\0\0\0", 0x13, 0x05, 0x03);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "POKEMON RED");
    }

    #[test]
    fn test_title_sixteen_bytes() {
        let data = build_header(b"ABCDEFGHIJKLMNOP", 0x00, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "ABCDEFGHIJKLMNOP");
    }

    #[test]
    fn test_title_cgb() {
        let data = build_header(b"PETAL LANES\0\0\0\0\x80", 0x03, 0x00, 0x02);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "PETAL LANES");
    }

    #[test]
    fn test_title_cgb_only() {
        let data = build_header(b"ABCDEFGHIJKLMNO\xc0", 0x1b, 0x06, 0x03);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "ABCDEFGHIJKLMNO");
    }

    #[test]
    fn test_title_trailing_spaces() {
        let data = build_header(b"HIVE    ", 0x00, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "HIVE");
    }

    #[test]
    fn test_title_empty() {
        let data = build_header(b"", 0x00, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.title(), "");
    }

    #[test]
    fn test_cartridge_type() {
        let data = build_header(b"GAME", 0x1b, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.cartridge_type(), 0x1b);
    }

    #[test]
    fn test_rom_banks() {
        for (value, banks) in [(0x00, 2), (0x01, 4), (0x05, 64), (0x08, 512)] {
            let data = build_header(b"GAME", 0x19, value, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert_eq!(header.rom_banks(), Ok(banks));
        }
    }

    #[test]
    fn test_rom_banks_invalid() {
        for value in [0x09, 0x52, 0xff] {
            let data = build_header(b"GAME", 0x19, value, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert_eq!(header.rom_banks(), Err(Error::RomSize));
        }
    }

    #[test]
    fn test_ram_size() {
        for (value, size) in [
            (0x00, 0),
            (0x01, 2048),
            (0x02, 8192),
            (0x03, 32768),
            (0x04, 131072),
            (0x05, 65536),
        ] {
            let data = build_header(b"GAME", 0x1b, 0x00, value);
            let header = Header::from_data(&data).unwrap();
            assert_eq!(header.ram_size(), Ok(size));
        }
    }

    #[test]
    fn test_ram_size_invalid() {
        let data = build_header(b"GAME", 0x1b, 0x00, 0x06);
        let header = Header::from_data(&data).unwrap();
        assert_eq!(
            header.ram_size(),
            Err(Error::DataError(String::from("Invalid RAM size: 0x06")))
        );
    }

    #[test]
    fn test_has_battery() {
        for cartridge_type in [0x03, 0x06, 0x09, 0x0f, 0x10, 0x13, 0x1b, 0x1e] {
            let data = build_header(b"GAME", cartridge_type, 0x00, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert!(header.has_battery());
        }
    }

    #[test]
    fn test_has_battery_none() {
        for cartridge_type in [0x00, 0x01, 0x02, 0x05, 0x08, 0x11, 0x12, 0x19, 0x1a] {
            let data = build_header(b"GAME", cartridge_type, 0x00, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert!(!header.has_battery());
        }
    }

    #[test]
    fn test_has_rumble() {
        for cartridge_type in [0x1c, 0x1d, 0x1e] {
            let data = build_header(b"GAME", cartridge_type, 0x00, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert!(header.has_rumble());
        }
    }

    #[test]
    fn test_has_rumble_none() {
        for cartridge_type in [0x00, 0x03, 0x13, 0x19, 0x1a, 0x1b, 0x22] {
            let data = build_header(b"GAME", cartridge_type, 0x00, 0x00);
            let header = Header::from_data(&data).unwrap();
            assert!(!header.has_rumble());
        }
    }

    #[test]
    fn test_is_valid() {
        let data = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        let header = Header::from_data(&data).unwrap();
        assert!(header.is_valid());
    }

    #[test]
    fn test_is_valid_mismatch() {
        let mut data = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        data[0x4d] = data[0x4d].wrapping_add(1);
        let header = Header::from_data(&data).unwrap();
        assert!(!header.is_valid());
    }

    #[test]
    fn test_is_valid_logo() {
        let mut data = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        data[0x18] ^= 0x01;
        let header = Header::from_data(&data).unwrap();
        assert!(!header.is_valid());
    }

    #[test]
    fn test_is_valid_no_cartridge() {
        let header = Header::from_data(&[0xff; HEADER_SIZE]).unwrap();
        assert!(!header.is_valid());
    }

    #[test]
    fn test_global_checksum() {
        let mut data = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        data[0x4e] = 0x40;
        data[0x4f] = 0xf3;
        let header = Header::from_data(&data).unwrap();
        assert_eq!(header.global_checksum(), 0x40f3);
    }

    #[test]
    fn test_matches_rom() {
        let mut rom = vec![0x00; 0x8000];
        rom[0x0000] = 0x31;
        rom[0x7fff] = 0xff;
        let mut header = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        let checksum = rom
            .iter()
            .chain(header.iter())
            .fold(0u16, |sum, &byte| sum.wrapping_add(byte as u16));
        header[0x4e] = (checksum >> 8) as u8;
        header[0x4f] = checksum as u8;
        rom[0x0100..0x0150].copy_from_slice(&header);
        let header = Header::from_data(&rom[0x0100..0x0150]).unwrap();
        assert!(header.matches_rom(&rom));
    }

    #[test]
    fn test_matches_rom_mismatch() {
        let mut rom = vec![0x00; 0x8000];
        let header = build_header(b"TETRIS", 0x00, 0x00, 0x00);
        rom[0x0100..0x0150].copy_from_slice(&header);
        rom[0x4000] = 0x01;
        let header = Header::from_data(&rom[0x0100..0x0150]).unwrap();
        assert!(!header.matches_rom(&rom));
    }
}
