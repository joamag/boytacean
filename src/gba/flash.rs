//! GBA cartridge save media emulation.
//!
//! Supports SRAM (direct byte read/write) and Flash (64KB/128KB)
//! with the standard command protocol used by Sanyo/SST chips.

use crate::gba::consts::SRAM_SIZE;

/// save media type detected from ROM strings
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveType {
    /// no backup media
    None,
    /// 32KB SRAM (direct byte access)
    Sram,
    /// 64KB flash (Sanyo LE39FW512)
    Flash64,
    /// 128KB flash (Sanyo LE39FW1024, bank-switched)
    Flash128,
}

/// flash command state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum FlashState {
    /// no command in progress
    Ready,
    /// received 0xAA at 0x5555
    CmdStep1,
    /// received 0x55 at 0x2AAA, waiting for command byte at 0x5555
    CmdStep2,
    /// 0xA0 issued, next byte write programs a single byte
    Write,
    /// 0x80 issued, waiting for second command sequence (erase type)
    EraseStep1,
    /// erase: received 0xAA at 0x5555
    EraseStep2,
    /// erase: received 0x55 at 0x2AAA, waiting for 0x10 or 0x30
    EraseStep3,
    /// 0x90 issued, reads return chip ID instead of data
    ChipId,
    /// 0xB0 issued, next write selects the flash bank (128KB only)
    BankSelect,
}

/// cartridge save media controller
pub struct SaveMedia {
    /// backing storage
    pub data: Vec<u8>,
    /// detected save type
    save_type: SaveType,
    /// flash command state
    state: FlashState,
    /// active bank for 128KB flash (0 or 1)
    bank: u8,
}

impl SaveMedia {
    pub fn new() -> Self {
        Self {
            data: vec![0xFFu8; SRAM_SIZE],
            save_type: SaveType::None,
            state: FlashState::Ready,
            bank: 0,
        }
    }

    /// detects save type from ROM identification strings
    pub fn detect_save_type(&mut self, rom: &[u8]) {
        self.save_type = detect_from_rom(rom);
    }

    pub fn save_type(&self) -> SaveType {
        self.save_type
    }

    /// reads a byte from the save region
    pub fn read8(&self, addr: u32) -> u8 {
        let offset = (addr & 0xFFFF) as usize;
        match self.save_type {
            SaveType::None => 0xFF,
            SaveType::Sram => {
                if offset < self.data.len() {
                    self.data[offset]
                } else {
                    0
                }
            }
            SaveType::Flash64 | SaveType::Flash128 => {
                if self.state == FlashState::ChipId && offset < 2 {
                    self.chip_id(offset)
                } else {
                    let real_offset = self.flash_offset(offset);
                    if real_offset < self.data.len() {
                        self.data[real_offset]
                    } else {
                        0xFF
                    }
                }
            }
        }
    }

    /// writes a byte to the save region
    pub fn write8(&mut self, addr: u32, value: u8) {
        let offset = (addr & 0xFFFF) as usize;
        match self.save_type {
            SaveType::None => {}
            SaveType::Sram => {
                if offset < self.data.len() {
                    self.data[offset] = value;
                }
            }
            SaveType::Flash64 | SaveType::Flash128 => {
                self.flash_write(offset, value);
            }
        }
    }

    pub fn reset(&mut self) {
        self.data.fill(0xFF);
        self.state = FlashState::Ready;
        self.bank = 0;
    }

    /// computes the actual data offset accounting for bank switching
    fn flash_offset(&self, offset: usize) -> usize {
        if self.save_type == SaveType::Flash128 {
            (self.bank as usize) * 0x10000 + offset
        } else {
            offset
        }
    }

    /// returns the manufacturer/device ID for chip ID mode
    fn chip_id(&self, offset: usize) -> u8 {
        match self.save_type {
            SaveType::Flash64 => match offset {
                0 => 0x62, // manufacturer (Sanyo)
                1 => 0x13, // device (LE39FW512)
                _ => 0,
            },
            SaveType::Flash128 => match offset {
                0 => 0x62, // manufacturer (Sanyo)
                1 => 0x13, // device (LE39FW1024)
                _ => 0,
            },
            _ => 0,
        }
    }

    /// processes a flash write through the command state machine
    fn flash_write(&mut self, offset: usize, value: u8) {
        match self.state {
            FlashState::Ready => {
                if offset == 0x5555 && value == 0xAA {
                    self.state = FlashState::CmdStep1;
                }
            }
            FlashState::CmdStep1 => {
                if offset == 0x2AAA && value == 0x55 {
                    self.state = FlashState::CmdStep2;
                } else {
                    self.state = FlashState::Ready;
                }
            }
            FlashState::CmdStep2 => {
                if offset == 0x5555 {
                    match value {
                        0xA0 => self.state = FlashState::Write,
                        0x80 => self.state = FlashState::EraseStep1,
                        0x90 => self.state = FlashState::ChipId,
                        0xB0 => self.state = FlashState::BankSelect,
                        0xF0 => self.state = FlashState::Ready,
                        _ => self.state = FlashState::Ready,
                    }
                } else {
                    self.state = FlashState::Ready;
                }
            }
            FlashState::Write => {
                // program a single byte (can only clear bits, not set them)
                let real_offset = self.flash_offset(offset);
                if real_offset < self.data.len() {
                    self.data[real_offset] &= value;
                }
                self.state = FlashState::Ready;
            }
            FlashState::EraseStep1 => {
                if offset == 0x5555 && value == 0xAA {
                    self.state = FlashState::EraseStep2;
                } else {
                    self.state = FlashState::Ready;
                }
            }
            FlashState::EraseStep2 => {
                if offset == 0x2AAA && value == 0x55 {
                    self.state = FlashState::EraseStep3;
                } else {
                    self.state = FlashState::Ready;
                }
            }
            FlashState::EraseStep3 => {
                match value {
                    0x10 if offset == 0x5555 => {
                        // chip erase: set all bytes to 0xFF
                        self.data.fill(0xFF);
                    }
                    0x30 => {
                        // sector erase: 4KB sector aligned
                        let sector_base = self.flash_offset(offset & !0xFFF);
                        let sector_end = (sector_base + 0x1000).min(self.data.len());
                        if sector_base < self.data.len() {
                            self.data[sector_base..sector_end].fill(0xFF);
                        }
                    }
                    _ => {}
                }
                self.state = FlashState::Ready;
            }
            FlashState::ChipId => {
                if value == 0xF0 {
                    self.state = FlashState::Ready;
                } else if offset == 0x5555 && value == 0xAA {
                    self.state = FlashState::CmdStep1;
                }
            }
            FlashState::BankSelect => {
                if offset == 0x0000 && self.save_type == SaveType::Flash128 {
                    self.bank = value & 1;
                }
                self.state = FlashState::Ready;
            }
        }
    }
}

impl Default for SaveMedia {
    fn default() -> Self {
        Self::new()
    }
}

/// scans ROM data for save type identification strings
fn detect_from_rom(rom: &[u8]) -> SaveType {
    let haystack = rom;
    if find_bytes(haystack, b"FLASH1M_V").is_some() {
        SaveType::Flash128
    } else if find_bytes(haystack, b"FLASH512_V").is_some()
        || find_bytes(haystack, b"FLASH_V").is_some()
    {
        SaveType::Flash64
    } else if find_bytes(haystack, b"SRAM_V").is_some() {
        SaveType::Sram
    } else {
        SaveType::None
    }
}

/// simple byte pattern search
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let save = SaveMedia::new();
        assert_eq!(save.save_type(), SaveType::None);
        assert_eq!(save.data.len(), SRAM_SIZE);
        assert!(save.data.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_detect_save_type_none() {
        let rom = vec![0u8; 512];
        assert_eq!(detect_from_rom(&rom), SaveType::None);
    }

    #[test]
    fn test_detect_save_type_sram() {
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Sram);
    }

    #[test]
    fn test_detect_save_type_flash64_flash_v() {
        let mut rom = vec![0u8; 512];
        rom[0x100..0x107].copy_from_slice(b"FLASH_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Flash64);
    }

    #[test]
    fn test_detect_save_type_flash64_flash512_v() {
        let mut rom = vec![0u8; 512];
        rom[0x100..0x10A].copy_from_slice(b"FLASH512_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Flash64);
    }

    #[test]
    fn test_detect_save_type_flash128() {
        let mut rom = vec![0u8; 512];
        rom[0x100..0x109].copy_from_slice(b"FLASH1M_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Flash128);
    }

    #[test]
    fn test_detect_save_type_updates_field() {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        save.detect_save_type(&rom);
        assert_eq!(save.save_type(), SaveType::Sram);
    }

    #[test]
    fn test_read8_none_returns_ff() {
        let save = SaveMedia::new();
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_sram_read8_uninitialized() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_sram_read8_after_write() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.read8(0x0E00_0000), 0x42);
    }

    #[test]
    fn test_sram_read8_addr_masking() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        save.write8(0x0E00_0010, 0xAB);
        // addr & 0xFFFF = 0x0010
        assert_eq!(save.read8(0x0F00_0010), 0xAB);
    }

    #[test]
    fn test_write8_none_ignored() {
        let mut save = SaveMedia::new();
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_sram_write8_direct() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.data[0], 0x42);
    }

    #[test]
    fn test_sram_write8_mirror() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        save.write8(0x0E00_0000, 0x01);
        // mirrors at +0x10000 (addr & 0xFFFF wraps)
        assert_eq!(save.read8(0x0E01_0000), 0x01);
    }

    #[test]
    fn test_flash64_write_requires_command() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // raw write without command sequence should not modify data
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_flash64_write_byte() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.read8(0x0E00_0000), 0x42);
    }

    #[test]
    fn test_flash64_write_only_clears_bits() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // write 0x0F (clears upper nibble from 0xFF)
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x0F);
        assert_eq!(save.read8(0x0E00_0000), 0x0F);
        // writing 0xF0 should AND with 0x0F -> 0x00
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0xF0);
        assert_eq!(save.read8(0x0E00_0000), 0x00);
    }

    #[test]
    fn test_flash64_write_single_byte_only() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // first byte write after 0xA0 programs, second goes nowhere
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x11);
        save.write8(0x0E00_0001, 0x22); // should be ignored (state back to Ready)
        assert_eq!(save.read8(0x0E00_0000), 0x11);
        assert_eq!(save.read8(0x0E00_0001), 0xFF);
    }

    #[test]
    fn test_flash64_chip_erase() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // write two bytes at different offsets
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x00);

        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_1000, 0x00);

        assert_eq!(save.read8(0x0E00_0000), 0x00);
        assert_eq!(save.read8(0x0E00_1000), 0x00);

        // chip erase: 0x80 + 0x10
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x80);
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x10);

        assert_eq!(save.read8(0x0E00_0000), 0xFF);
        assert_eq!(save.read8(0x0E00_1000), 0xFF);
    }

    #[test]
    fn test_flash64_sector_erase() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // write bytes in sector 0 (0x0000) and sector 1 (0x1000)
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x00);

        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_1000, 0x00);

        // sector erase sector 0 only: 0x80 + 0x30 @ sector base
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x80);
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_0000, 0x30);

        assert_eq!(save.read8(0x0E00_0000), 0xFF);
        // sector 1 untouched
        assert_eq!(save.read8(0x0E00_1000), 0x00);
    }

    #[test]
    fn test_flash64_chip_id_enter() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x90);
        assert_eq!(save.read8(0x0E00_0000), 0x62);
        assert_eq!(save.read8(0x0E00_0001), 0x13);
    }

    #[test]
    fn test_flash64_chip_id_exit_f0() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x90);
        // exit with raw 0xF0
        save.write8(0x0E00_0000, 0xF0);
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_flash64_chip_id_exit_command() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x90);
        // exit via full command sequence
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xF0);
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_flash64_chip_id_data_still_readable() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // write a byte at offset 0x10
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0010, 0x42);
        // enter chip ID mode
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x90);
        // offsets >= 2 still read flash data
        assert_eq!(save.read8(0x0E00_0010), 0x42);
    }

    #[test]
    fn test_flash64_incomplete_command_resets() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // start a command but send wrong second byte
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_0000, 0x55); // wrong address, should reset
        // now try a normal write - should still require full command
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x42);
        assert_eq!(save.read8(0x0E00_0000), 0x42);
    }

    #[test]
    fn test_reset_clears_data() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Sram;
        save.write8(0x0E00_0000, 0x42);
        save.reset();
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_reset_clears_flash_state() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash64;
        // enter chip ID mode
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0x90);
        assert_eq!(save.read8(0x0E00_0000), 0x62);
        save.reset();
        // should be back to normal reads
        assert_eq!(save.read8(0x0E00_0000), 0xFF);
    }

    #[test]
    fn test_flash128_bank_select() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Flash128;
        save.data = vec![0xFFu8; 0x20000]; // 128KB

        // write byte to bank 0
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x11);

        // switch to bank 1
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xB0);
        save.write8(0x0E00_0000, 0x01);

        // bank 1 should still be 0xFF
        assert_eq!(save.read8(0x0E00_0000), 0xFF);

        // write to bank 1
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xA0);
        save.write8(0x0E00_0000, 0x22);
        assert_eq!(save.read8(0x0E00_0000), 0x22);

        // switch back to bank 0
        save.write8(0x0E00_5555, 0xAA);
        save.write8(0x0E00_2AAA, 0x55);
        save.write8(0x0E00_5555, 0xB0);
        save.write8(0x0E00_0000, 0x00);

        assert_eq!(save.read8(0x0E00_0000), 0x11);
    }

    #[test]
    fn test_default() {
        let save = SaveMedia::default();
        assert_eq!(save.save_type(), SaveType::None);
    }

    #[test]
    fn test_detect_flash1m_takes_priority_over_flash512() {
        let mut rom = vec![0u8; 1024];
        rom[0x100..0x10A].copy_from_slice(b"FLASH512_V");
        rom[0x200..0x209].copy_from_slice(b"FLASH1M_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Flash128);
    }
}
