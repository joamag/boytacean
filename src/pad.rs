//! Gamepad related functions and structures.

use std::{
    fmt::{self, Display, Formatter},
    io::Cursor,
};

use crate::{
    mmu::BusComponent,
    state::{StateComponent, StateFormat},
    warnln,
};

use boytacean_common::{
    data::{read_u8, write_u8},
    error::Error,
};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadSelection {
    None,
    Action,
    Direction,
}

impl PadSelection {
    pub fn description(&self) -> &'static str {
        match self {
            PadSelection::None => "None",
            PadSelection::Action => "Action",
            PadSelection::Direction => "Direction",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => PadSelection::None,
            0x01 => PadSelection::Action,
            0x02 => PadSelection::Direction,
            _ => panic!("Invalid pad selection value: {value}"),
        }
    }

    pub fn into_u8(self) -> u8 {
        match self {
            PadSelection::None => 0x00,
            PadSelection::Action => 0x01,
            PadSelection::Direction => 0x02,
        }
    }
}

impl Display for PadSelection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for PadSelection {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<PadSelection> for u8 {
    fn from(value: PadSelection) -> Self {
        value.into_u8()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum PadKey {
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
    A,
    B,
}

impl PadKey {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => PadKey::Up,
            2 => PadKey::Down,
            3 => PadKey::Left,
            4 => PadKey::Right,
            5 => PadKey::Start,
            6 => PadKey::Select,
            7 => PadKey::A,
            8 => PadKey::B,
            _ => panic!("Invalid pad key value: {value}"),
        }
    }
}

impl From<u8> for PadKey {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

pub struct Pad {
    down: bool,
    up: bool,
    left: bool,
    right: bool,
    start: bool,
    select: bool,
    b: bool,
    a: bool,
    selection: PadSelection,
    int_pad: bool,
}

impl Pad {
    pub fn new() -> Self {
        Self {
            down: false,
            up: false,
            left: false,
            right: false,
            start: false,
            select: false,
            b: false,
            a: false,
            selection: PadSelection::None,
            int_pad: false,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0xFF00 — P1/JOYP: Joypad
            0xff00 => {
                let mut value = match self.selection {
                    PadSelection::Action =>
                    {
                        #[allow(clippy::bool_to_int_with_if)]
                        (if self.a { 0x00 } else { 0x01 }
                            | if self.b { 0x00 } else { 0x02 }
                            | if self.select { 0x00 } else { 0x04 }
                            | if self.start { 0x00 } else { 0x08 })
                    }
                    PadSelection::Direction =>
                    {
                        #[allow(clippy::bool_to_int_with_if)]
                        (if self.right { 0x00 } else { 0x01 }
                            | if self.left { 0x00 } else { 0x02 }
                            | if self.up { 0x00 } else { 0x04 }
                            | if self.down { 0x00 } else { 0x08 })
                    }
                    PadSelection::None => 0x0f,
                };
                value |= match self.selection {
                    PadSelection::Action => 0x10,
                    PadSelection::Direction => 0x20,
                    PadSelection::None => 0x30,
                };
                value
            }
            _ => {
                warnln!("Reading from unknown Pad location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF00 — P1/JOYP: Joypad
            0xff00 => {
                self.selection = match value & 0x30 {
                    0x10 => PadSelection::Action,
                    0x20 => PadSelection::Direction,
                    0x30 => PadSelection::None,
                    _ => PadSelection::None,
                };
            }
            _ => warnln!("Writing to unknown Pad location 0x{:04x}", addr),
        }
    }

    pub fn key_press(&mut self, key: PadKey) {
        match key {
            PadKey::Up => self.up = true,
            PadKey::Down => self.down = true,
            PadKey::Left => self.left = true,
            PadKey::Right => self.right = true,
            PadKey::Start => self.start = true,
            PadKey::Select => self.select = true,
            PadKey::A => self.a = true,
            PadKey::B => self.b = true,
        }

        // signals that a JoyPad interrupt is pending to be
        // handled as a key press has been performed
        self.int_pad = true;
    }

    pub fn key_lift(&mut self, key: PadKey) {
        match key {
            PadKey::Up => self.up = false,
            PadKey::Down => self.down = false,
            PadKey::Left => self.left = false,
            PadKey::Right => self.right = false,
            PadKey::Start => self.start = false,
            PadKey::Select => self.select = false,
            PadKey::A => self.a = false,
            PadKey::B => self.b = false,
        }
    }

    #[inline(always)]
    pub fn int_pad(&self) -> bool {
        self.int_pad
    }

    #[inline(always)]
    pub fn set_int_pad(&mut self, value: bool) {
        self.int_pad = value;
    }

    #[inline(always)]
    pub fn ack_pad(&mut self) {
        self.set_int_pad(false);
    }

    /// Packs all 8 buttons into a single byte for network transmission.
    ///
    /// Bit layout:
    /// - Bit 0: Up
    /// - Bit 1: Down
    /// - Bit 2: Left
    /// - Bit 3: Right
    /// - Bit 4: Start
    /// - Bit 5: Select
    /// - Bit 6: A
    /// - Bit 7: B
    pub fn pack_input(&self) -> u8 {
        let mut packed = 0u8;
        if self.up {
            packed |= 0x01;
        }
        if self.down {
            packed |= 0x02;
        }
        if self.left {
            packed |= 0x04;
        }
        if self.right {
            packed |= 0x08;
        }
        if self.start {
            packed |= 0x10;
        }
        if self.select {
            packed |= 0x20;
        }
        if self.a {
            packed |= 0x40;
        }
        if self.b {
            packed |= 0x80;
        }
        packed
    }

    /// Unpacks a byte into button states.
    ///
    /// This sets all button states according to the packed byte,
    /// useful for applying remote player inputs in netplay.
    pub fn unpack_input(&mut self, packed: u8) {
        self.up = packed & 0x01 != 0;
        self.down = packed & 0x02 != 0;
        self.left = packed & 0x04 != 0;
        self.right = packed & 0x08 != 0;
        self.start = packed & 0x10 != 0;
        self.select = packed & 0x20 != 0;
        self.a = packed & 0x40 != 0;
        self.b = packed & 0x80 != 0;
    }

    /// Apply packed input and trigger interrupt if any button is pressed.
    pub fn apply_packed_input(&mut self, packed: u8) {
        let had_buttons = self.pack_input() != 0;
        self.unpack_input(packed);
        let has_buttons = packed != 0;

        // Trigger interrupt on new button press
        if has_buttons && !had_buttons {
            self.int_pad = true;
        }
    }
}

impl BusComponent for Pad {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl StateComponent for Pad {
    fn state(&self, _format: Option<StateFormat>) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(vec![]);
        write_u8(&mut cursor, self.down as u8)?;
        write_u8(&mut cursor, self.up as u8)?;
        write_u8(&mut cursor, self.left as u8)?;
        write_u8(&mut cursor, self.right as u8)?;
        write_u8(&mut cursor, self.start as u8)?;
        write_u8(&mut cursor, self.select as u8)?;
        write_u8(&mut cursor, self.b as u8)?;
        write_u8(&mut cursor, self.a as u8)?;
        write_u8(&mut cursor, self.selection.into())?;
        write_u8(&mut cursor, self.int_pad as u8)?;
        Ok(cursor.into_inner())
    }

    fn set_state(&mut self, data: &[u8], _format: Option<StateFormat>) -> Result<(), Error> {
        let mut cursor: Cursor<&[u8]> = Cursor::new(data);
        self.down = read_u8(&mut cursor)? != 0;
        self.up = read_u8(&mut cursor)? != 0;
        self.left = read_u8(&mut cursor)? != 0;
        self.right = read_u8(&mut cursor)? != 0;
        self.start = read_u8(&mut cursor)? != 0;
        self.select = read_u8(&mut cursor)? != 0;
        self.b = read_u8(&mut cursor)? != 0;
        self.a = read_u8(&mut cursor)? != 0;
        self.selection = read_u8(&mut cursor)?.into();
        self.int_pad = read_u8(&mut cursor)? != 0;
        Ok(())
    }
}

impl Default for Pad {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::state::StateComponent;

    use super::{Pad, PadSelection};

    #[test]
    fn test_state_and_set_state() {
        let pad = Pad {
            down: true,
            up: false,
            left: true,
            right: false,
            start: true,
            select: false,
            b: true,
            a: false,
            selection: PadSelection::Action,
            int_pad: true,
        };

        let state = pad.state(None).unwrap();
        assert_eq!(state.len(), 10);

        let mut new_pad = Pad::new();
        new_pad.set_state(&state, None).unwrap();

        assert!(new_pad.down);
        assert!(!new_pad.up);
        assert!(new_pad.left);
        assert!(!new_pad.right);
        assert!(new_pad.start);
        assert!(!new_pad.select);
        assert!(new_pad.b);
        assert!(!new_pad.a);
        assert_eq!(new_pad.selection, PadSelection::Action);
        assert!(new_pad.int_pad);
    }

    #[test]
    fn test_pack_unpack_input() {
        let mut pad = Pad::new();

        pad.up = true;
        pad.down = false;
        pad.left = true;
        pad.right = false;
        pad.start = true;
        pad.select = false;
        pad.a = true;
        pad.b = false;

        let packed = pad.pack_input();
        assert_eq!(packed, 0b01010101); // A, Start, Left, Up

        // Create new pad and unpack
        let mut new_pad = Pad::new();
        new_pad.unpack_input(packed);

        assert!(new_pad.up);
        assert!(!new_pad.down);
        assert!(new_pad.left);
        assert!(!new_pad.right);
        assert!(new_pad.start);
        assert!(!new_pad.select);
        assert!(new_pad.a);
        assert!(!new_pad.b);
    }

    #[test]
    fn test_pack_all_buttons() {
        let mut pad = Pad::new();

        pad.up = true;
        pad.down = true;
        pad.left = true;
        pad.right = true;
        pad.start = true;
        pad.select = true;
        pad.a = true;
        pad.b = true;

        let packed = pad.pack_input();
        assert_eq!(packed, 0xff);

        let empty_pad = Pad::new();
        assert_eq!(empty_pad.pack_input(), 0x00);
    }

    #[test]
    fn test_apply_packed_input() {
        let mut pad = Pad::new();

        pad.apply_packed_input(0x40);
        assert!(pad.a);
        assert!(pad.int_pad);

        pad.ack_pad();

        pad.apply_packed_input(0x40);
        assert!(!pad.int_pad);

        pad.apply_packed_input(0x00);
        pad.apply_packed_input(0x80); // B button
        assert!(pad.b);
        assert!(pad.int_pad);
    }
}
