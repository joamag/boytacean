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
}

#[derive(Clone, Copy, PartialEq)]
pub enum PadSelection {
    Action,
    Direction,
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
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x0000 => {
                let mut value;
                match self.selection {
                    PadSelection::Action => {
                        value = if self.a { 0x00 } else { 0x01 }
                            | if self.b { 0x00 } else { 0x02 }
                            | if self.select { 0x00 } else { 0x04 }
                            | if self.start { 0x00 } else { 0x08 }
                    }
                    PadSelection::Direction => {
                        value = if self.right { 0x00 } else { 0x01 }
                            | if self.left { 0x00 } else { 0x02 }
                            | if self.up { 0x00 } else { 0x04 }
                            | if self.down { 0x00 } else { 0x08 }
                    }
                }
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
            addr => panic!("Reading from unknown Pad location 0x{:04x}", addr),
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
            addr => panic!("Writing to unknown Pad location 0x{:04x}", addr),
        }
    }
}
