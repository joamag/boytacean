use crate::warnln;

pub struct Serial {
    data: u8,
    control: u8,
    shift_clock: bool,
    clock_speed: bool,
    transferring: bool,
}

impl Serial {
    pub fn new() -> Self {
        Self {
            data: 0x0,
            control: 0x0,
            shift_clock: false,
            clock_speed: false,
            transferring: false,
        }
    }

    pub fn reset(&mut self) {
        self.data = 0x0;
        self.control = 0x0;
        self.shift_clock = false;
        self.clock_speed = false;
        self.transferring = false;
    }

    pub fn clock(&mut self, cycles: u8) {}

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x01 => self.data,
            0x02 => {
                (if self.shift_clock { 0x01 } else { 0x00 }
                    | if self.clock_speed { 0x02 } else { 0x00 }
                    | if self.transferring { 0x80 } else { 0x00 })
            }
            _ => {
                warnln!("Reding from unknown Timer location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0x00ff {
            0x01 => self.data = value,
            0x02 => {
                self.shift_clock = value & 0x01 == 0x01;
                self.clock_speed = value & 0x02 == 0x02;
                self.transferring = value & 0x80 == 0x80;
            }
            _ => warnln!("Writing to unknown Serial location 0x{:04x}", addr),
        }
    }

    fn send(&self) -> bool {
        if self.shift_clock {
            true
        } else {
            self.data & 0x80 == 0x80
        }
    }

    fn receive(&self, bit: bool) {
        if !self.shift_clock {
            //data = (data << 1) | bit;
            //check_transfer();
        }
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self::new()
    }
}
