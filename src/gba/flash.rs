//! GBA cartridge save media emulation.
//!
//! Supports SRAM (direct byte read/write), Flash (64KB/128KB)
//! with the standard command protocol used by Sanyo/SST chips,
//! and EEPROM (512B/8KB) with the serial DMA-based protocol.

use crate::gba::consts::SRAM_SIZE;

/// EEPROM size constants
const EEPROM_SIZE_4K: usize = 512;
const EEPROM_SIZE_64K: usize = 8192;

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
    /// EEPROM (512B or 8KB, serial DMA protocol)
    Eeprom,
}

/// EEPROM serial protocol state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum EepromState {
    /// waiting for command and address bits
    AcceptingCommand,
    /// reading data bits (4 dummy + 64 data)
    ReadingData,
    /// collecting 64 data bits + stop bit for write
    CollectingWriteData,
    /// write pending, reads return 0 until ready
    WritePending,
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
    /// EEPROM state machine
    eeprom_state: EepromState,
    /// EEPROM bit buffer for incoming serial data
    eeprom_buffer: u64,
    /// number of bits received in current command
    eeprom_bits: u8,
    /// EEPROM address (block index) for current read/write
    eeprom_addr: u16,
    /// EEPROM address width: 6 (512B) or 14 (8KB), 0 = auto-detect
    eeprom_addr_width: u8,
    /// EEPROM read output shift register (68 bits: 4 dummy + 64 data)
    eeprom_read_buffer: u64,
    /// number of bits remaining to output during read
    eeprom_read_bits: u8,
    /// ROM size (used to determine EEPROM bus address range)
    rom_size: usize,
}

impl SaveMedia {
    pub fn new() -> Self {
        Self {
            data: vec![0xFFu8; SRAM_SIZE],
            save_type: SaveType::None,
            state: FlashState::Ready,
            bank: 0,
            eeprom_state: EepromState::AcceptingCommand,
            eeprom_buffer: 0,
            eeprom_bits: 0,
            eeprom_addr: 0,
            eeprom_addr_width: 0,
            eeprom_read_buffer: 0,
            eeprom_read_bits: 0,
            rom_size: 0,
        }
    }

    /// detects save type from ROM identification strings
    pub fn detect_save_type(&mut self, rom: &[u8]) {
        self.save_type = detect_from_rom(rom);
        self.rom_size = rom.len();
        if self.save_type == SaveType::Eeprom {
            // start with 512B; auto-detect 8KB on first access
            self.data = vec![0xFFu8; EEPROM_SIZE_4K];
        }
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
            SaveType::Eeprom => 0xFF, // EEPROM uses 16-bit DMA access
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
            SaveType::Eeprom => {} // EEPROM uses 16-bit DMA access
        }
    }

    /// checks if an address in the 0x0D region maps to EEPROM
    pub fn is_eeprom_addr(&self, addr: u32) -> bool {
        if self.save_type != SaveType::Eeprom {
            return false;
        }
        // for ROMs <= 16MB, the entire 0x0D region is EEPROM
        // for ROMs > 16MB, only 0x0DFFFF00-0x0DFFFFFF
        if self.rom_size > 16 * 1024 * 1024 {
            addr >= 0x0DFF_FF00
        } else {
            (addr >> 24) == 0x0D
        }
    }

    /// reads a bit from the EEPROM serial interface (bit 0 of return value)
    pub fn eeprom_read(&mut self) -> u16 {
        match self.eeprom_state {
            EepromState::ReadingData => {
                if self.eeprom_read_bits > 0 {
                    self.eeprom_read_bits -= 1;
                    let bit = if self.eeprom_read_bits >= 64 {
                        // dummy bits (first 4 of 68)
                        0
                    } else {
                        // data bits, MSB first
                        ((self.eeprom_read_buffer >> self.eeprom_read_bits) & 1) as u16
                    };
                    // transition back after last bit
                    if self.eeprom_read_bits == 0 {
                        self.eeprom_state = EepromState::AcceptingCommand;
                        self.eeprom_bits = 0;
                        self.eeprom_buffer = 0;
                    }
                    bit
                } else {
                    // already done, return ready
                    self.eeprom_state = EepromState::AcceptingCommand;
                    self.eeprom_bits = 0;
                    self.eeprom_buffer = 0;
                    1
                }
            }
            EepromState::WritePending => {
                // write completed (instant in emulation), return ready
                self.eeprom_state = EepromState::AcceptingCommand;
                self.eeprom_bits = 0;
                self.eeprom_buffer = 0;
                1
            }
            _ => 1,
        }
    }

    /// writes a bit to the EEPROM serial interface (bit 0 of value)
    pub fn eeprom_write(&mut self, value: u16) {
        let bit = value & 1;

        match self.eeprom_state {
            EepromState::AcceptingCommand => {
                self.eeprom_buffer = (self.eeprom_buffer << 1) | bit as u64;
                self.eeprom_bits += 1;

                // need at least 2 bits for the command type
                if self.eeprom_bits < 2 {
                    return;
                }

                let cmd = (self.eeprom_buffer >> (self.eeprom_bits - 2)) & 0x03;
                let addr_bits = self.eeprom_bits - 2;

                if cmd == 0x03 {
                    // read: "11" + N address bits + "0" stop bit
                    let expected = self.eeprom_expected_addr_width();
                    if addr_bits == expected + 1 {
                        self.eeprom_addr =
                            ((self.eeprom_buffer >> 1) & ((1u64 << expected) - 1)) as u16;
                        self.eeprom_start_read();
                    }
                } else if cmd == 0x02 {
                    // write: "10" + N address bits, then transition to data collection
                    let expected = self.eeprom_expected_addr_width();
                    if addr_bits == expected {
                        self.eeprom_addr = (self.eeprom_buffer & ((1u64 << expected) - 1)) as u16;
                        self.eeprom_state = EepromState::CollectingWriteData;
                        self.eeprom_buffer = 0;
                        self.eeprom_bits = 0;
                    }
                } else if self.eeprom_bits >= 2 {
                    // invalid command, reset
                    self.eeprom_bits = 0;
                    self.eeprom_buffer = 0;
                }
            }
            EepromState::CollectingWriteData => {
                if self.eeprom_bits < 64 {
                    self.eeprom_buffer = (self.eeprom_buffer << 1) | bit as u64;
                }
                self.eeprom_bits += 1;

                // 64 data bits + 1 stop bit = 65 bits
                if self.eeprom_bits == 65 {
                    self.eeprom_commit_write(self.eeprom_buffer);
                }
            }
            EepromState::ReadingData | EepromState::WritePending => {
                // ignore writes during read/write operations
            }
        }
    }

    /// returns the expected EEPROM address width, auto-detecting on first use
    fn eeprom_expected_addr_width(&mut self) -> u8 {
        if self.eeprom_addr_width != 0 {
            return self.eeprom_addr_width;
        }
        // auto-detect based on ROM size:
        // ROMs > 16Mbit (2MB) use 14-bit addressing (8KB EEPROM)
        // ROMs <= 16Mbit use 6-bit addressing (512B EEPROM)
        if self.rom_size > 2 * 1024 * 1024 {
            self.eeprom_addr_width = 14;
            self.data.resize(EEPROM_SIZE_64K, 0xFF);
            14
        } else {
            self.eeprom_addr_width = 6;
            6
        }
    }

    /// loads 8 bytes from the EEPROM at the current address into the read buffer
    fn eeprom_start_read(&mut self) {
        let byte_offset = self.eeprom_addr as usize * 8;
        let mut value = 0u64;
        for i in 0..8 {
            let b = if byte_offset + i < self.data.len() {
                self.data[byte_offset + i]
            } else {
                0xFF
            };
            value = (value << 8) | b as u64;
        }
        self.eeprom_read_buffer = value;
        self.eeprom_read_bits = 68; // 4 dummy + 64 data
        self.eeprom_state = EepromState::ReadingData;
    }

    /// writes 8 bytes of data to the EEPROM at the current address
    fn eeprom_commit_write(&mut self, data: u64) {
        let byte_offset = self.eeprom_addr as usize * 8;

        for i in 0..8 {
            let b = ((data >> (56 - i * 8)) & 0xFF) as u8;
            if byte_offset + i < self.data.len() {
                self.data[byte_offset + i] = b;
            }
        }

        self.eeprom_state = EepromState::WritePending;
        self.eeprom_bits = 0;
        self.eeprom_buffer = 0;
    }

    pub fn reset(&mut self) {
        self.data.fill(0xFF);
        self.state = FlashState::Ready;
        self.bank = 0;
        self.eeprom_state = EepromState::AcceptingCommand;
        self.eeprom_buffer = 0;
        self.eeprom_bits = 0;
        self.eeprom_addr = 0;
        self.eeprom_read_buffer = 0;
        self.eeprom_read_bits = 0;
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
    if find_bytes(haystack, b"EEPROM_V").is_some() {
        SaveType::Eeprom
    } else if find_bytes(haystack, b"FLASH1M_V").is_some() {
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
    haystack.windows(needle.len()).position(|w| w == needle)
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

    // --- EEPROM detection tests ---

    #[test]
    fn test_detect_save_type_eeprom() {
        let mut rom = vec![0u8; 512];
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Eeprom);
    }

    #[test]
    fn test_detect_eeprom_takes_priority() {
        let mut rom = vec![0u8; 1024];
        rom[0x100..0x106].copy_from_slice(b"SRAM_V");
        rom[0x200..0x208].copy_from_slice(b"EEPROM_V");
        assert_eq!(detect_from_rom(&rom), SaveType::Eeprom);
    }

    #[test]
    fn test_detect_eeprom_initializes_data() {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 512];
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        save.detect_save_type(&rom);
        assert_eq!(save.save_type(), SaveType::Eeprom);
        assert_eq!(save.data.len(), EEPROM_SIZE_4K);
    }

    #[test]
    fn test_detect_eeprom_stores_rom_size() {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 4 * 1024 * 1024];
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        save.detect_save_type(&rom);
        assert_eq!(save.rom_size, 4 * 1024 * 1024);
    }

    // --- EEPROM address detection tests ---

    #[test]
    fn test_is_eeprom_addr_not_eeprom() {
        let save = SaveMedia::new();
        assert!(!save.is_eeprom_addr(0x0D00_0000));
    }

    #[test]
    fn test_is_eeprom_addr_small_rom() {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 4 * 1024 * 1024]; // 4MB ROM
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        save.detect_save_type(&rom);
        // entire 0x0D region is EEPROM for ROMs <= 16MB
        assert!(save.is_eeprom_addr(0x0D00_0000));
        assert!(save.is_eeprom_addr(0x0DFF_FFFF));
        // 0x0C is still ROM
        assert!(!save.is_eeprom_addr(0x0C00_0000));
    }

    #[test]
    fn test_is_eeprom_addr_large_rom() {
        let mut save = SaveMedia::new();
        save.save_type = SaveType::Eeprom;
        save.rom_size = 32 * 1024 * 1024; // 32MB ROM
                                          // only top of 0x0D region is EEPROM
        assert!(save.is_eeprom_addr(0x0DFF_FF00));
        assert!(save.is_eeprom_addr(0x0DFF_FFFE));
        assert!(!save.is_eeprom_addr(0x0D00_0000));
    }

    // --- EEPROM read/write protocol tests ---

    /// creates a small-ROM EEPROM save (6-bit addressing, 512B)
    fn make_eeprom_save_6bit() -> SaveMedia {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 1 * 1024 * 1024]; // 1MB ROM -> 6-bit
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        save.detect_save_type(&rom);
        save
    }

    /// creates a large-ROM EEPROM save (14-bit addressing, 8KB)
    fn make_eeprom_save_14bit() -> SaveMedia {
        let mut save = SaveMedia::new();
        let mut rom = vec![0u8; 4 * 1024 * 1024]; // 4MB ROM -> 14-bit
        rom[0x100..0x108].copy_from_slice(b"EEPROM_V");
        save.detect_save_type(&rom);
        save
    }

    /// sends a sequence of bits to the EEPROM (MSB first)
    fn send_bits(save: &mut SaveMedia, value: u64, count: u8) {
        for i in (0..count).rev() {
            save.eeprom_write(((value >> i) & 1) as u16);
        }
    }

    #[test]
    fn test_eeprom_write_and_read_back() {
        let mut save = make_eeprom_save_6bit();

        // write command: "10" + 6-bit addr (0) + 64 data bits + 1 stop bit
        send_bits(&mut save, 0b10, 2); // write command
        send_bits(&mut save, 0, 6); // address 0

        // now in CollectingWriteData state, send 64 data bits + stop
        let test_data: u64 = 0xDEADBEEF_CAFEBABE;
        send_bits(&mut save, test_data, 64);
        send_bits(&mut save, 0, 1); // stop bit

        // write should be pending, read returns 1 (ready)
        assert_eq!(save.eeprom_read(), 1);

        // now read back: "11" + 6-bit addr (0) + 1 stop bit
        send_bits(&mut save, 0b11, 2); // read command
        send_bits(&mut save, 0, 6); // address 0
        send_bits(&mut save, 0, 1); // stop bit

        // read 68 bits: 4 dummy + 64 data
        let mut result = 0u64;
        for _ in 0..4 {
            save.eeprom_read(); // dummy bits
        }
        for _ in 0..64 {
            result = (result << 1) | save.eeprom_read() as u64;
        }

        assert_eq!(result, test_data);
    }

    #[test]
    fn test_eeprom_read_unwritten_returns_ff() {
        let mut save = make_eeprom_save_6bit();

        // read address 1 (never written)
        send_bits(&mut save, 0b11, 2); // read command
        send_bits(&mut save, 1, 6); // address 1
        send_bits(&mut save, 0, 1); // stop bit

        // skip 4 dummy bits
        for _ in 0..4 {
            save.eeprom_read();
        }
        // read 64 data bits - should all be 1 (0xFF fill)
        let mut result = 0u64;
        for _ in 0..64 {
            result = (result << 1) | save.eeprom_read() as u64;
        }
        assert_eq!(result, 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn test_eeprom_write_different_addresses() {
        let mut save = make_eeprom_save_6bit();

        // write 0xAAAA... to address 0
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0xAAAA_AAAA_AAAA_AAAA, 64);
        send_bits(&mut save, 0, 1);
        save.eeprom_read(); // ack write

        // write 0x5555... to address 1
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 1, 6);
        send_bits(&mut save, 0x5555_5555_5555_5555, 64);
        send_bits(&mut save, 0, 1);
        save.eeprom_read(); // ack write

        // verify address 0
        send_bits(&mut save, 0b11, 2);
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0, 1);
        for _ in 0..4 {
            save.eeprom_read();
        }
        let mut result = 0u64;
        for _ in 0..64 {
            result = (result << 1) | save.eeprom_read() as u64;
        }
        assert_eq!(result, 0xAAAA_AAAA_AAAA_AAAA);

        // verify address 1
        send_bits(&mut save, 0b11, 2);
        send_bits(&mut save, 1, 6);
        send_bits(&mut save, 0, 1);
        for _ in 0..4 {
            save.eeprom_read();
        }
        let mut result2 = 0u64;
        for _ in 0..64 {
            result2 = (result2 << 1) | save.eeprom_read() as u64;
        }
        assert_eq!(result2, 0x5555_5555_5555_5555);
    }

    #[test]
    fn test_eeprom_reset_clears_state() {
        let mut save = make_eeprom_save_6bit();

        // start a write command
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 0, 6);
        // reset mid-operation
        save.reset();

        assert_eq!(save.eeprom_state, EepromState::AcceptingCommand);
        assert_eq!(save.eeprom_bits, 0);
        assert_eq!(save.eeprom_read_bits, 0);
    }

    #[test]
    fn test_eeprom_idle_read_returns_1() {
        let mut save = make_eeprom_save_6bit();
        // reading before any command returns 1
        assert_eq!(save.eeprom_read(), 1);
    }

    #[test]
    fn test_eeprom_write_overwrite() {
        let mut save = make_eeprom_save_6bit();

        // write data to address 0
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0x1234_5678_9ABC_DEF0, 64);
        send_bits(&mut save, 0, 1);
        save.eeprom_read();

        // overwrite address 0 with different data
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0xFEDC_BA98_7654_3210, 64);
        send_bits(&mut save, 0, 1);
        save.eeprom_read();

        // read back - should be the new data
        send_bits(&mut save, 0b11, 2);
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0, 1);
        for _ in 0..4 {
            save.eeprom_read();
        }
        let mut result = 0u64;
        for _ in 0..64 {
            result = (result << 1) | save.eeprom_read() as u64;
        }
        assert_eq!(result, 0xFEDC_BA98_7654_3210);
    }

    #[test]
    fn test_eeprom_14bit_write_and_read_back() {
        let mut save = make_eeprom_save_14bit();
        // data starts at 512B, gets resized to 8KB on first EEPROM access

        // write command: "10" + 14-bit addr (0) + 64 data bits + 1 stop bit
        send_bits(&mut save, 0b10, 2);
        send_bits(&mut save, 0, 14); // address 0
        let test_data: u64 = 0xDEADBEEF_CAFEBABE;
        send_bits(&mut save, test_data, 64);
        send_bits(&mut save, 0, 1); // stop bit
        assert_eq!(save.eeprom_read(), 1); // ack

        // read back: "11" + 14-bit addr (0) + 1 stop bit
        send_bits(&mut save, 0b11, 2);
        send_bits(&mut save, 0, 14);
        send_bits(&mut save, 0, 1);
        for _ in 0..4 {
            save.eeprom_read(); // dummy
        }
        let mut result = 0u64;
        for _ in 0..64 {
            result = (result << 1) | save.eeprom_read() as u64;
        }
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_eeprom_addr_width_auto_detect_small_rom() {
        let mut save = make_eeprom_save_6bit();
        assert_eq!(save.eeprom_addr_width, 0); // not yet detected
                                               // trigger detection via first command
        send_bits(&mut save, 0b11, 2); // read command
        send_bits(&mut save, 0, 6);
        send_bits(&mut save, 0, 1);
        assert_eq!(save.eeprom_addr_width, 6);
    }

    #[test]
    fn test_eeprom_addr_width_auto_detect_large_rom() {
        let mut save = make_eeprom_save_14bit();
        assert_eq!(save.eeprom_addr_width, 0); // not yet detected
        send_bits(&mut save, 0b11, 2); // read command
        send_bits(&mut save, 0, 14);
        send_bits(&mut save, 0, 1);
        assert_eq!(save.eeprom_addr_width, 14);
    }
}
