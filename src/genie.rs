use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameGenie {
    /// Hash map that contains the complete set of Game Genie
    /// codes that have been registered for the current ROM.
    /// These codes are going to apply a series of patches to
    /// the ROM effectively allowing the user to cheat.
    codes: HashMap<u16, GameGenieCode>,
}

impl GameGenie {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.codes.clear();
    }

    pub fn contains_addr(&self, addr: u16) -> bool {
        self.codes.contains_key(&addr)
    }

    pub fn get_addr(&self, addr: u16) -> &GameGenieCode {
        self.codes.get(&addr).unwrap()
    }

    pub fn add_code(&mut self, code: &str) -> Result<&GameGenieCode, String> {
        let genie_code = match GameGenieCode::from_code(code) {
            Ok(genie_code) => genie_code,
            Err(e) => return Err(e),
        };
        let addr = genie_code.addr;
        self.codes.insert(addr, genie_code);
        Ok(self.get_addr(addr))
    }
}

impl Default for GameGenie {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GameGenieCode {
    code: String,
    addr: u16,
    new_data: u8,
    old_data: u8,
}

impl GameGenieCode {
    pub fn from_code(code: &str) -> Result<Self, String> {
        if code.len() != 11 {
            return Err(String::from("Invalid Game Genie code length"));
        }

        let code_u = code.to_uppercase();

        let new_data_slice = &code_u[0..=1];
        let new_data = u8::from_str_radix(new_data_slice, 16).unwrap();

        let old_data_slice = format!("{}{}", &code_u[8..=8], &code_u[10..=10]);
        let old_data: u8 = u8::from_str_radix(old_data_slice.as_str(), 16)
            .unwrap()
            .rotate_right(2)
            ^ 0xba;

        let addr_slice = format!("{}{}{}", &code_u[6..=6], &code_u[2..=2], &code_u[4..=5]);
        let addr = u16::from_str_radix(addr_slice.as_str(), 16).unwrap() ^ 0xf000;

        Ok(Self {
            code: code_u,
            addr,
            new_data,
            old_data,
        })
    }

    pub fn is_valid(&self, value: u8) -> bool {
        self.old_data == value
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn set_code(&mut self, code: String) {
        self.code = code;
    }

    pub fn addr(&self) -> u16 {
        self.addr
    }

    pub fn set_addr(&mut self, addr: u16) {
        self.addr = addr;
    }

    pub fn new_data(&self) -> u8 {
        self.new_data
    }

    pub fn set_new_data(&mut self, new_data: u8) {
        self.new_data = new_data;
    }

    pub fn old_data(&self) -> u8 {
        self.old_data
    }

    pub fn set_old_data(&mut self, old_data: u8) {
        self.old_data = old_data;
    }

    pub fn short_description(&self) -> String {
        self.code.to_string()
    }

    pub fn description(&self) -> String {
        format!(
            "Code: {}, Address: 0x{:04x}, New Data: 0x{:04x}, Old Data: 0x{:04x}",
            self.code, self.addr, self.new_data, self.old_data
        )
    }
}

impl Display for GameGenieCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_description())
    }
}

#[cfg(test)]
mod tests {
    use crate::genie::GameGenieCode;

    #[test]
    fn test_from_code() {
        let code = "00A-17B-C49";
        let game_genie_code = GameGenieCode::from_code(code).unwrap();

        assert_eq!(game_genie_code.code, "00A-17B-C49");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0xc8);
    }
}
