pub struct Apu {
}

impl Apu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0xff26 => 1 as u8, // todo implement this
            _ => 0x00
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xff26 => {
                // @todo implement the logic here
            },
            _ => {}
        }
    }
}
