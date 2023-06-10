use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

#[derive(Clone)]
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

    pub fn contains_addr(&self, addr: u16) -> bool {
        self.codes.contains_key(&addr)
    }

    pub fn get_addr(&self, addr: u16) -> &GameGenieCode {
        self.codes.get(&addr).unwrap()
    }

    pub fn add_code(&mut self, code: &str) -> Result<&GameGenieCode, &str> {
        if code.len() != 11 {
            return Err("Invalid Game Genie code length");
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

        let genie_code = GameGenieCode {
            code: code_u,
            addr,
            new_data,
            old_data,
        };

        self.codes.insert(addr, genie_code);
        Ok(self.codes.get(&addr).unwrap())
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
    pub fn is_valid(&self, value: u8) -> bool {
        self.old_data == value
    }

    pub fn new_data(&self) -> u8 {
        self.new_data
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
