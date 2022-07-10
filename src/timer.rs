use crate::warnln;

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    div_clock: u16,
    tima_clock: u16,
    tima_enabled: bool,
    tima_ratio: u16,
    int_tima: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0x0,
            div_clock: 0,
            tima_clock: 0,
            tima_enabled: false,
            tima_ratio: 1024,
            int_tima: false,
        }
    }

    pub fn clock(&mut self, cycles: u8) {
        self.div_clock += cycles as u16;
        while self.div_clock >= 256 {
            self.div = self.div.wrapping_add(1);
            self.div_clock -= 256;
        }

        if self.tima_enabled {
            self.tima_clock += cycles as u16;
            while self.tima_clock >= self.tima_ratio {
                if self.tima == 0xff {
                    self.int_tima = true;
                    self.tima = self.tma;
                }

                self.tima = self.tima.wrapping_add(1);
                self.tima_clock -= self.tima_ratio;
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr & 0x00ff {
            0x04 => self.div,
            0x05 => self.tima,
            0x06 => self.tma,
            0x07 => self.tac,
            _ => {
                warnln!("Reding from unknown Timer location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr & 0x00ff {
            0x04 => self.div = 0,
            0x05 => self.tima = value,
            0x06 => self.tma = value,
            0x07 => {
                self.tac = value;
                match value & 0x03 {
                    0x00 => self.tima_ratio = 1024,
                    0x01 => self.tima_ratio = 16,
                    0x02 => self.tima_ratio = 64,
                    0x03 => self.tima_ratio = 256,
                    value => panic!("Invalid TAC value 0x{:02x}", value),
                }
                self.tima_enabled = (value & 0x04) == 0x04;
            }
            _ => warnln!("Writing to unknown Timer location 0x{:04x}", addr),
        }
    }

    pub fn int_tima(&self) -> bool {
        self.int_tima
    }

    pub fn set_int_tima(&mut self, value: bool) {
        self.int_tima = value;
    }

    pub fn ack_tima(&mut self) {
        self.set_int_tima(false);
    }
}
