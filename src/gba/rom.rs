//! GBA ROM header parsing, validation, and auto-detection.

use std::fmt::{self, Display, Formatter};

use boytacean_common::error::Error;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// GBA ROM header offsets.
const HEADER_TITLE: usize = 0x0A0;
const HEADER_GAME_CODE: usize = 0x0AC;
const HEADER_MAKER_CODE: usize = 0x0B0;
const HEADER_FIXED_VALUE: usize = 0x0B2;
const HEADER_SOFTWARE_VERSION: usize = 0x0BC;
const HEADER_CHECKSUM: usize = 0x0BD;

/// Minimum valid ROM size (must at least contain the header).
const MIN_ROM_SIZE: usize = 0xC0;

/// Nintendo logo stored in the cartridge header.
const NINTENDO_LOGO: [u8; 156] = [
    0x24, 0xFF, 0xAE, 0x51, 0x69, 0x9A, 0xA2, 0x21, 0x3D, 0x84, 0x82, 0x0A, 0x84, 0xE4, 0x09, 0xAD,
    0x11, 0x24, 0x8B, 0x98, 0xC0, 0x81, 0x7F, 0x21, 0xA3, 0x52, 0xBE, 0x19, 0x93, 0x09, 0xCE, 0x20,
    0x10, 0x46, 0x4A, 0x4A, 0xF8, 0x27, 0x31, 0xEC, 0x58, 0xC7, 0xE8, 0x33, 0x82, 0xE3, 0xCE, 0xBF,
    0x85, 0xF4, 0xDF, 0x94, 0xCE, 0x4B, 0x09, 0xC1, 0x94, 0x56, 0x8A, 0xC0, 0x13, 0x72, 0xA7, 0xFC,
    0x9F, 0x84, 0x4D, 0x73, 0xA3, 0xCA, 0x9A, 0x61, 0x58, 0x97, 0xA3, 0x27, 0xFC, 0x03, 0x98, 0x76,
    0x23, 0x1D, 0xC7, 0x61, 0x03, 0x04, 0xAE, 0x56, 0xBF, 0x38, 0x84, 0x00, 0x40, 0xA7, 0x0E, 0xFD,
    0xFF, 0x52, 0xFE, 0x03, 0x6F, 0x95, 0x30, 0xF1, 0x97, 0xFB, 0xC0, 0x85, 0x60, 0xD6, 0x80, 0x25,
    0xA9, 0x63, 0xBE, 0x03, 0x01, 0x4E, 0x38, 0xE2, 0xF9, 0xA2, 0x34, 0xFF, 0xBB, 0x3E, 0x03, 0x44,
    0x78, 0x00, 0x90, 0xCB, 0x88, 0x11, 0x3A, 0x94, 0x65, 0xC0, 0x7C, 0x63, 0x87, 0xF0, 0x3C, 0xAF,
    0xD6, 0x25, 0xE4, 0x8B, 0x38, 0x0A, 0xAC, 0x72, 0x21, 0xD4, 0xF8, 0x07,
];

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone)]
pub struct GbaRomInfo {
    title: String,
    game_code: String,
    maker_code: String,
    software_version: u8,
    header_checksum: u8,
    rom_size: usize,
}

impl GbaRomInfo {
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() < MIN_ROM_SIZE {
            return Err(Error::RomSize);
        }

        if data[HEADER_FIXED_VALUE] != 0x96 {
            return Err(Error::InvalidData);
        }

        let title = String::from_utf8_lossy(&data[HEADER_TITLE..HEADER_TITLE + 12])
            .trim_end_matches('\0')
            .to_string();

        let game_code = String::from_utf8_lossy(&data[HEADER_GAME_CODE..HEADER_GAME_CODE + 4])
            .trim_end_matches('\0')
            .to_string();

        let maker_code = String::from_utf8_lossy(&data[HEADER_MAKER_CODE..HEADER_MAKER_CODE + 2])
            .trim_end_matches('\0')
            .to_string();

        let software_version = data[HEADER_SOFTWARE_VERSION];
        let header_checksum = data[HEADER_CHECKSUM];

        Ok(Self {
            title,
            game_code,
            maker_code,
            software_version,
            header_checksum,
            rom_size: data.len(),
        })
    }

    /// Validates the header checksum against the computed value.
    pub fn validate_checksum(&self, data: &[u8]) -> bool {
        data.len() >= MIN_ROM_SIZE && compute_checksum(data) == self.header_checksum
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl GbaRomInfo {
    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn game_code(&self) -> String {
        self.game_code.clone()
    }

    pub fn maker_code(&self) -> String {
        self.maker_code.clone()
    }

    pub fn software_version(&self) -> u8 {
        self.software_version
    }

    pub fn header_checksum(&self) -> u8 {
        self.header_checksum
    }

    pub fn rom_size(&self) -> usize {
        self.rom_size
    }

    pub fn description(&self, column_length: usize) -> String {
        let title_l = format!("{:width$}", "Title", width = column_length);
        let code_l = format!("{:width$}", "Code", width = column_length);
        let maker_l = format!("{:width$}", "Maker", width = column_length);
        let size_l = format!("{:width$}", "Size", width = column_length);
        format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {} KB",
            title_l,
            self.title,
            code_l,
            self.game_code,
            maker_l,
            self.maker_code,
            size_l,
            self.rom_size / 1024
        )
    }
}

impl Display for GbaRomInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GBA ROM: {} ({}) by {} v{} [{}KB]",
            self.title,
            self.game_code,
            self.maker_code,
            self.software_version,
            self.rom_size / 1024
        )
    }
}

/// Checks the fixed value, Nintendo logo, and checksum to detect a GBA ROM.
pub fn is_gba_rom(data: &[u8]) -> bool {
    data.len() >= MIN_ROM_SIZE
        && data[HEADER_FIXED_VALUE] == 0x96
        && data[0x04..0xA0] == NINTENDO_LOGO
        && compute_checksum(data) == data[HEADER_CHECKSUM]
}

/// Computes the header checksum for validation.
fn compute_checksum(data: &[u8]) -> u8 {
    let mut checksum: u8 = 0;
    for &byte in &data[0xA0..0xBD] {
        checksum = checksum.wrapping_sub(byte);
    }
    checksum.wrapping_sub(0x19)
}

#[cfg(test)]
mod tests {
    use super::{compute_checksum, is_gba_rom, GbaRomInfo, NINTENDO_LOGO};

    fn make_gba_rom(title: &str, game_code: &str, maker_code: &str) -> Vec<u8> {
        let mut data = vec![0u8; 0x200];
        data[0x04..0xA0].copy_from_slice(&NINTENDO_LOGO);
        data[0xB2] = 0x96; // fixed value

        // write title (12 bytes at 0xA0)
        for (i, b) in title.bytes().take(12).enumerate() {
            data[0xA0 + i] = b;
        }
        // write game code (4 bytes at 0xAC)
        for (i, b) in game_code.bytes().take(4).enumerate() {
            data[0xAC + i] = b;
        }
        // write maker code (2 bytes at 0xB0)
        for (i, b) in maker_code.bytes().take(2).enumerate() {
            data[0xB0 + i] = b;
        }
        // set software version
        data[0xBC] = 1;
        // compute and set checksum
        data[0xBD] = compute_checksum(&data);
        data
    }

    #[test]
    fn test_is_gba_rom_too_small() {
        let data = vec![0u8; 32];
        assert!(!is_gba_rom(&data));
    }

    #[test]
    fn test_is_gba_rom_no_fixed_value() {
        let data = vec![0u8; 0xC0];
        assert!(!is_gba_rom(&data));
    }

    #[test]
    fn test_is_gba_rom_valid() {
        let data = make_gba_rom("TESTGAME", "ATST", "01");
        assert!(is_gba_rom(&data));
    }

    #[test]
    fn test_is_gba_rom_invalid_header() {
        let data = make_gba_rom("TESTGAME", "ATST", "01");
        for offset in [0x04, 0x9F, 0xA0, 0xB2, 0xBD] {
            let mut invalid = data.clone();
            invalid[offset] ^= 1;
            assert!(!is_gba_rom(&invalid));
        }
        let mut gb = include_bytes!("../../res/roms/demo/pocket.gb").to_vec();
        gb[0xB2] = 0x96;
        assert!(!is_gba_rom(&gb));
    }

    #[test]
    fn test_from_data() {
        let data = make_gba_rom("TESTGAME", "ATST", "01");
        let info = GbaRomInfo::from_data(&data).unwrap();
        assert_eq!(info.title(), "TESTGAME");
        assert_eq!(info.game_code(), "ATST");
        assert_eq!(info.maker_code(), "01");
        assert_eq!(info.software_version(), 1);
        assert_eq!(info.rom_size(), 0x200);
    }

    #[test]
    fn test_from_data_too_small() {
        let data = vec![0u8; 32];
        assert!(GbaRomInfo::from_data(&data).is_err());
    }

    #[test]
    fn test_from_data_not_gba() {
        let data = vec![0u8; 0xC0];
        assert!(GbaRomInfo::from_data(&data).is_err());
    }

    #[test]
    fn test_from_data_homebrew() {
        let mut data = vec![0u8; 0xC0];
        data[0xB2] = 0x96;
        assert!(GbaRomInfo::from_data(&data).is_ok());
        assert!(!is_gba_rom(&data));
    }

    #[test]
    fn test_validate_checksum() {
        let data = make_gba_rom("TESTGAME", "ATST", "01");
        let info = GbaRomInfo::from_data(&data).unwrap();
        assert!(info.validate_checksum(&data));
        for size in [0, 0xA0, 0xBD, 0xBF] {
            assert!(!info.validate_checksum(&data[..size]));
        }
    }

    #[test]
    fn test_display() {
        let data = make_gba_rom("MYGAME", "AMGP", "01");
        let info = GbaRomInfo::from_data(&data).unwrap();
        let display = format!("{}", info);
        assert!(display.contains("MYGAME"));
        assert!(display.contains("AMGP"));
    }
}
