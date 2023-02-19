#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::warnln;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PadSelection {
    Action,
    Direction,
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
            selection: PadSelection::Action,
            int_pad: false,
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x0000 => {
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
                };
                value |= if self.selection == PadSelection::Direction {
                    0x10
                } else {
                    0x00
                } | if self.selection == PadSelection::Action {
                    0x20
                } else {
                    0x00
                };
                value
            }
            _ => {
                warnln!("Reading from unknown Pad location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0x00ff {
            0x0000 => {
                self.selection = if value & 0x10 == 0x00 {
                    PadSelection::Direction
                } else {
                    PadSelection::Action
                }
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
        // handled as a key pressed has been done
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

    pub fn int_pad(&self) -> bool {
        self.int_pad
    }

    pub fn set_int_pad(&mut self, value: bool) {
        self.int_pad = value;
    }

    pub fn ack_pad(&mut self) {
        self.set_int_pad(false);
    }
}

impl Default for Pad {
    fn default() -> Self {
        Self::new()
    }
}
