pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    ratio: u16,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0x0,
            ratio: 1024
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        0x00
    }

    pub fn write(&mut self, addr: u16, value: u8) {
    }
}

