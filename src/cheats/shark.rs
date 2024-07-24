use boytacean_common::error::Error;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::rom::RomType;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Implementation of the GameShark cheat code system
/// that "patches" RAM entries, making use of the V-Blank
/// time to do that.
///
/// The codes in the GameShark system are in an hexadecimal
/// ASCII format in the form of "ABCDGHEF" where:
/// AB = RAM bank
/// CD = New data
/// GH = Address LSB
/// EF = Address MSB
///
/// [Wikipedia - GameShark](https://en.wikipedia.org/wiki/GameShark)
#[derive(Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameShark {
    /// Hash map that contains the complete set of GameShark
    /// codes that have been registered for the current ROM.
    /// These codes are going to apply a series of patches to
    /// the RAM effectively allowing the user to cheat.
    codes: HashMap<u16, GameSharkCode>,

    /// The kind of ROM (Cartridge) that is going to be patched.
    /// Relevant for some operations.
    rom_type: RomType,
}

impl GameShark {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
            rom_type: RomType::RomOnly,
        }
    }

    pub fn set_rom_type(&mut self, rom_type: RomType) {
        self.rom_type = rom_type;
    }

    pub fn is_code(code: &str) -> bool {
        if code.len() != 8 {
            return false;
        }
        if code.contains('-') || code.contains('+') {
            return false;
        }
        true
    }

    pub fn reset(&mut self) {
        self.codes.clear();
    }

    pub fn get_addr(&self, addr: u16) -> Result<&GameSharkCode, Error> {
        match self.codes.get(&addr) {
            Some(code) => Ok(code),
            None => Err(Error::CustomError(format!("Invalid address: {}", addr))),
        }
    }

    pub fn add_code(&mut self, code: &str) -> Result<&GameSharkCode, Error> {
        let shark_code = GameSharkCode::from_code(code, &self.rom_type)?;
        let addr = shark_code.addr;
        self.codes.insert(addr, shark_code);
        self.get_addr(addr)
    }

    pub fn writes(&self) -> Vec<(u16, u16, u8)> {
        let mut writes = vec![];
        for code in self.codes.values() {
            // calculates the real RAM address using both
            // the base RAM address and the RAM bank offset
            let (base_addr, addr) = if code.addr <= 0xc000 {
                (
                    0xa000,
                    code.addr - 0xa000 + (0x1000 * (code.ram_bank - 1) as u16),
                )
            } else {
                (0xc000, code.addr - 0xc000)
            };
            writes.push((base_addr, addr, code.new_data));
        }
        writes
    }
}

impl Default for GameShark {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GameSharkCode {
    /// The GameShark code that is going to be applied to the ROM.
    code: String,

    /// The RAM bank that the cheat code is going to be applied to,
    /// allowing advanced MBCs to be patched.
    ram_bank: u8,

    /// The new data that is going to be written to the address.
    new_data: u8,

    /// Address of the data that is going to be patched.
    addr: u16,
}

impl GameSharkCode {
    /// Creates a new GameShark code structure from the provided string
    /// in the ABCDGHEF format.
    pub fn from_code(code: &str, rom_type: &RomType) -> Result<Self, Error> {
        let code_length = code.len();

        if code_length != 8 {
            return Err(Error::CustomError(format!(
                "Invalid GameShark code length: {} digits",
                code_length
            )));
        }

        let code_u = code.to_uppercase();

        let ram_bank_slice = &code_u[0..=1];
        let mut ram_bank = u8::from_str_radix(ram_bank_slice, 16)
            .map_err(|e| Error::CustomError(format!("Invalid RAM bank: {}", e)))?
            & rom_type.mbc_type().ram_bank_mask();
        ram_bank = if ram_bank == 0x00 { 0x01 } else { ram_bank };

        let new_data_slice = &code_u[2..=3];
        let new_data = u8::from_str_radix(new_data_slice, 16)
            .map_err(|e| Error::CustomError(format!("Invalid new data: {}", e)))?;

        let addr_slice = format!("{}{}", &code_u[6..=7], &code_u[4..=5]);
        let addr = u16::from_str_radix(&addr_slice, 16)
            .map_err(|e| Error::CustomError(format!("Invalid address: {}", e)))?;

        if !(0xa000..=0xdfff).contains(&addr) {
            return Err(Error::CustomError(format!(
                "Invalid cheat address: 0x{:04x}",
                addr
            )));
        }

        Ok(Self {
            code: code_u,
            ram_bank,
            new_data,
            addr,
        })
    }

    /// Tests whether the provided value is valid for the current
    /// GameShark code
    pub fn is_valid(&self, _value: u8) -> bool {
        true
    }

    pub fn patch_data(&self, _value: u8) -> u8 {
        self.new_data()
    }

    pub fn code(&self) -> &str {
        &self.code
    }
    pub fn set_code(&mut self, code: String) {
        self.code = code;
    }

    pub fn ram_bank(&self) -> u8 {
        self.ram_bank
    }

    pub fn set_ram_bank(&mut self, ram_bank: u8) {
        self.ram_bank = ram_bank;
    }

    pub fn new_data(&self) -> u8 {
        self.new_data
    }

    pub fn set_new_data(&mut self, new_data: u8) {
        self.new_data = new_data;
    }

    pub fn addr(&self) -> u16 {
        self.addr
    }

    pub fn set_addr(&mut self, addr: u16) {
        self.addr = addr;
    }

    pub fn short_description(&self) -> String {
        self.code.to_string()
    }

    pub fn description(&self) -> String {
        format!(
            "Code: {}, RAM Bank: 0x{:02x}, New Data: 0x{:02x}, Address: 0x{:04x}",
            self.code, self.ram_bank, self.new_data, self.addr
        )
    }
}

impl Display for GameSharkCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_description())
    }
}
