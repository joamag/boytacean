use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameShark {
    /// Hash map that contains the complete set of Game Shark
    /// codes that have been registered for the current ROM.
    /// These codes are going to apply a series of patches to
    /// the RAM effectively allowing the user to cheat.
    codes: HashMap<u16, GameSharkCode>,
}

impl GameShark {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
        }
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

    pub fn contains_addr(&self, addr: u16) -> bool {
        self.codes.contains_key(&addr)
    }

    pub fn get_addr(&self, addr: u16) -> &GameSharkCode {
        self.codes.get(&addr).unwrap()
    }

    pub fn add_code(&mut self, code: &str) -> Result<&GameSharkCode, String> {
        let genie_code = match GameSharkCode::from_code(code, None) {
            Ok(genie_code) => genie_code,
            Err(message) => return Err(message),
        };
        let addr = genie_code.addr;
        self.codes.insert(addr, genie_code);
        Ok(self.get_addr(addr))
    }
}

impl Default for GameShark {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GameSharkCode {
    /// The Game Genie code that is going to be applied to the ROM.
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
    /// Creates a new Game Shark code structure from the provided string
    /// in the ABCDGHEF format.
    pub fn from_code(code: &str, _handle_additive: Option<bool>) -> Result<Self, String> {
        let code_length = code.len();

        if code_length != 8 {
            return Err(format!(
                "Invalid Game Shark code length: {} digits",
                code_length
            ));
        }

        let code_u = code.to_uppercase();

        let ram_bank_slice = &code_u[0..=1];
        let ram_bank = u8::from_str_radix(ram_bank_slice, 16).unwrap();

        let new_data_slice = &code_u[2..=3];
        let new_data = u8::from_str_radix(new_data_slice, 16).unwrap();

        let addr_slice = &code_u[4..=7];
        let addr = u16::from_str_radix(addr_slice, 16).unwrap();

        Ok(Self {
            code: code_u,
            ram_bank,
            new_data,
            addr,
        })
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
