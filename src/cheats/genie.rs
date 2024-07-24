use boytacean_common::error::Error;
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

    pub fn is_code(code: &str) -> bool {
        if code.len() != 11 && code.len() != 7 {
            return false;
        }
        if !code.contains('-') && !code.contains('+') {
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

    pub fn get_addr(&self, addr: u16) -> Result<&GameGenieCode, Error> {
        match self.codes.get(&addr) {
            Some(code) => Ok(code),
            None => Err(Error::CustomError(format!("Invalid address: {}", addr))),
        }
    }

    pub fn add_code(&mut self, code: &str) -> Result<&GameGenieCode, Error> {
        let genie_code = GameGenieCode::from_code(code, None)?;
        let addr = genie_code.addr;
        self.codes.insert(addr, genie_code);
        self.get_addr(addr)
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

    /// A boolean value indicating whether the provided cheat code
    /// is additive or not. If the code is additive, the new data
    /// will be added to the old data, otherwise the new data will
    /// replace the old data.
    additive: bool,

    /// A boolean value indicating whether the provided cheat code
    /// was condensed (7 characters) or extended (11 characters).
    condensed: bool,
}

impl GameGenieCode {
    /// Creates a new Game Genie code structure from the provided string
    /// in the ABC-DEF-GHI or ABC-DEF format.
    /// Note that the additive mode (ex: ABC+DEF+GHI) can be optionally
    /// handled or ignored using the `handle_additive` parameter.
    pub fn from_code(code: &str, handle_additive: Option<bool>) -> Result<Self, Error> {
        let code_length = code.len();

        if code_length != 11 && code_length != 7 {
            return Err(Error::CustomError(format!(
                "Invalid Game Genie code length: {} digits",
                code_length
            )));
        }

        let code_u = code.to_uppercase();

        let additive = if handle_additive.unwrap_or(false) {
            code_u.chars().nth(3).unwrap() == '+'
        } else {
            false
        };
        let condensed = code_length == 7;

        let new_data_slice = &code_u[0..=1];
        let new_data = u8::from_str_radix(new_data_slice, 16)
            .map_err(|e| Error::CustomError(format!("Invalid new data: {}", e)))?;

        let old_data = if code_length == 11 {
            let old_data_slice: String = format!("{}{}", &code_u[8..=8], &code_u[10..=10]);
            u8::from_str_radix(old_data_slice.as_str(), 16)
                .map_err(|e| Error::CustomError(format!("Invalid old data: {}", e)))?
                .rotate_right(2)
                ^ 0xba
        } else {
            0x00
        };

        let addr_slice = format!("{}{}{}", &code_u[6..=6], &code_u[2..=2], &code_u[4..=5]);
        let addr = u16::from_str_radix(addr_slice.as_str(), 16)
            .map_err(|e| Error::CustomError(format!("Invalid address: {}", e)))?
            ^ 0xf000;

        if addr > 0x7fff {
            return Err(Error::CustomError(format!(
                "Invalid cheat address: 0x{:04x}",
                addr
            )));
        }

        Ok(Self {
            code: code_u,
            addr,
            new_data,
            old_data,
            additive,
            condensed,
        })
    }

    /// Tests whether the provided value is valid for the current
    /// Game Genie code. A value is valid if it matches the old
    /// data or if the code is condensed.
    pub fn is_valid(&self, value: u8) -> bool {
        self.condensed || self.old_data == value
    }

    /// Patches the provided value with the new data according to
    /// the Game Genie code. If the code is additive, the new data
    /// is added to the current value otherwise the new data is
    /// returned (simple and normal patching operation).
    pub fn patch_data(&self, value: u8) -> u8 {
        if self.additive() {
            value.saturating_add(self.new_data())
        } else {
            self.new_data()
        }
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

    pub fn additive(&self) -> bool {
        self.additive
    }

    pub fn set_additive(&mut self, additive: bool) {
        self.additive = additive;
    }

    pub fn short_description(&self) -> String {
        self.code.to_string()
    }

    pub fn description(&self) -> String {
        format!(
            "Code: {}, Address: 0x{:04x}, New Data: 0x{:02x}, Old Data: 0x{:02x}",
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
    use super::GameGenieCode;

    #[test]
    fn test_from_code() {
        let mut game_genie_code = GameGenieCode::from_code("00A-17B-C49", None).unwrap();
        assert_eq!(game_genie_code.code, "00A-17B-C49");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0xc8);
        assert!(!game_genie_code.additive);
        assert!(!game_genie_code.condensed);
        assert!(game_genie_code.is_valid(0xc8));
        assert!(!game_genie_code.is_valid(0xc9));
        assert_eq!(game_genie_code.patch_data(0x12), 0x00);

        game_genie_code = GameGenieCode::from_code("00A+17B+C49", None).unwrap();
        assert_eq!(game_genie_code.code, "00A+17B+C49");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0xc8);
        assert!(!game_genie_code.additive);
        assert!(!game_genie_code.condensed);
        assert!(game_genie_code.is_valid(0xc8));
        assert!(!game_genie_code.is_valid(0xc9));
        assert_eq!(game_genie_code.patch_data(0x12), 0x00);

        game_genie_code = GameGenieCode::from_code("00A+17B+C49", Some(true)).unwrap();
        assert_eq!(game_genie_code.code, "00A+17B+C49");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0xc8);
        assert!(game_genie_code.additive);
        assert!(!game_genie_code.condensed);
        assert!(game_genie_code.is_valid(0xc8));
        assert!(!game_genie_code.is_valid(0xc9));
        assert_eq!(game_genie_code.patch_data(0x12), 0x12);

        game_genie_code = GameGenieCode::from_code("00A+17B", None).unwrap();
        assert_eq!(game_genie_code.code, "00A+17B");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0x00);
        assert!(!game_genie_code.additive);
        assert!(game_genie_code.condensed);
        assert!(game_genie_code.is_valid(0xc8));
        assert!(game_genie_code.is_valid(0xc9));
        assert_eq!(game_genie_code.patch_data(0x12), 0x00);

        game_genie_code = GameGenieCode::from_code("00A+17B", Some(true)).unwrap();
        assert_eq!(game_genie_code.code, "00A+17B");
        assert_eq!(game_genie_code.addr, 0x4a17);
        assert_eq!(game_genie_code.new_data, 0x00);
        assert_eq!(game_genie_code.old_data, 0x00);
        assert!(game_genie_code.additive);
        assert!(game_genie_code.condensed);
        assert!(game_genie_code.is_valid(0xc8));
        assert!(game_genie_code.is_valid(0xc9));
        assert_eq!(game_genie_code.patch_data(0x12), 0x012);
    }
}
