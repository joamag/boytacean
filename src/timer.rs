//! Timer functions and structures.

use std::io::Cursor;

use boytacean_common::{
    data::{read_u8, read_u16, write_u8, write_u16},
    error::Error,
};

use crate::{
    consts::{DIV_ADDR, TAC_ADDR, TIMA_ADDR, TMA_ADDR},
    mmu::BusComponent,
    panic_gb,
    state::{StateComponent, StateFormat},
    warnln,
};

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

    pub fn reset(&mut self) {
        self.div = 0;
        self.tima = 0;
        self.tma = 0;
        self.tac = 0x0;
        self.div_clock = 0;
        self.tima_clock = 0;
        self.tima_enabled = false;
        self.tima_ratio = 1024;
        self.int_tima = false;
    }

    pub fn clock(&mut self, cycles: u16) {
        self.div_clock += cycles;
        while self.div_clock >= 256 {
            self.div = self.div.wrapping_add(1);
            self.div_clock -= 256;
        }

        if self.tima_enabled {
            self.tima_clock += cycles;
            while self.tima_clock >= self.tima_ratio {
                // in case TIMA value overflows must set the
                // interrupt and update the TIMA value to
                // the TMA one (reset operation)
                if self.tima == 0xff {
                    self.int_tima = true;
                    self.tima = self.tma;
                }
                // otherwise uses the normal add operation
                // and increments the TIMA value by one
                else {
                    self.tima = self.tima.wrapping_add(1);
                }

                self.tima_clock -= self.tima_ratio;
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0xFF04 — DIV: Divider register
            DIV_ADDR => self.div,
            // 0xFF05 — TIMA: Timer counter
            TIMA_ADDR => self.tima,
            // 0xFF06 — TMA: Timer modulo
            TMA_ADDR => self.tma,
            // 0xFF07 — TAC: Timer control
            TAC_ADDR => self.tac | 0xf8,
            _ => {
                warnln!("Reding from unknown Timer location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF04 — DIV: Divider register
            DIV_ADDR => self.div = 0,
            // 0xFF05 — TIMA: Timer counter
            TIMA_ADDR => self.tima = value,
            // 0xFF06 — TMA: Timer modulo
            TMA_ADDR => self.tma = value,
            // 0xFF07 — TAC: Timer control
            TAC_ADDR => {
                self.tac = value;
                match value & 0x03 {
                    0x00 => self.tima_ratio = 1024,
                    0x01 => self.tima_ratio = 16,
                    0x02 => self.tima_ratio = 64,
                    0x03 => self.tima_ratio = 256,
                    value => panic_gb!("Invalid TAC value 0x{:02x}", value),
                }
                self.tima_enabled = (value & 0x04) == 0x04;
            }
            _ => warnln!("Writing to unknown Timer location 0x{:04x}", addr),
        }
    }

    #[inline(always)]
    pub fn int_tima(&self) -> bool {
        self.int_tima
    }

    #[inline(always)]
    pub fn set_int_tima(&mut self, value: bool) {
        self.int_tima = value;
    }

    #[inline(always)]
    pub fn ack_tima(&mut self) {
        self.set_int_tima(false);
    }

    #[inline(always)]
    pub fn div(&self) -> u8 {
        self.div
    }

    #[inline(always)]
    pub fn set_div(&mut self, value: u8) {
        self.div = value;
    }

    #[inline(always)]
    pub fn div_clock(&self) -> u16 {
        self.div_clock
    }

    #[inline(always)]
    pub fn set_div_clock(&mut self, value: u16) {
        self.div_clock = value;
    }
}

impl BusComponent for Timer {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl StateComponent for Timer {
    fn state(&self, _format: Option<StateFormat>) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(vec![]);
        write_u8(&mut cursor, self.div)?;
        write_u8(&mut cursor, self.tima)?;
        write_u8(&mut cursor, self.tma)?;
        write_u8(&mut cursor, self.tac)?;
        write_u16(&mut cursor, self.div_clock)?;
        write_u16(&mut cursor, self.tima_clock)?;
        write_u8(&mut cursor, self.tima_enabled as u8)?;
        write_u16(&mut cursor, self.tima_ratio)?;
        write_u8(&mut cursor, self.int_tima as u8)?;
        Ok(cursor.into_inner())
    }

    fn set_state(&mut self, data: &[u8], _format: Option<StateFormat>) -> Result<(), Error> {
        let mut cursor = Cursor::new(data);
        self.div = read_u8(&mut cursor)?;
        self.tima = read_u8(&mut cursor)?;
        self.tma = read_u8(&mut cursor)?;
        self.tac = read_u8(&mut cursor)?;
        self.div_clock = read_u16(&mut cursor)?;
        self.tima_clock = read_u16(&mut cursor)?;
        self.tima_enabled = read_u8(&mut cursor)? != 0;
        self.tima_ratio = read_u16(&mut cursor)?;
        self.int_tima = read_u8(&mut cursor)? != 0;
        Ok(())
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Timer;

    use crate::state::StateComponent;

    #[test]
    fn test_state_and_set_state() {
        let timer = Timer {
            div: 0x12,
            tima: 0x34,
            tma: 0x56,
            tac: 0x78,
            div_clock: 0x9abc,
            tima_clock: 0xdef0,
            tima_enabled: true,
            tima_ratio: 0x1234,
            int_tima: true,
        };

        let state = timer.state(None).unwrap();
        assert_eq!(state.len(), 12);

        let mut new_timer = Timer::new();
        new_timer.set_state(&state, None).unwrap();

        assert_eq!(new_timer.div, 0x12);
        assert_eq!(new_timer.tima, 0x34);
        assert_eq!(new_timer.tma, 0x56);
        assert_eq!(new_timer.tac, 0x78);
        assert_eq!(new_timer.div_clock, 0x9abc);
        assert_eq!(new_timer.tima_clock, 0xdef0);
        assert!(new_timer.tima_enabled);
        assert_eq!(new_timer.tima_ratio, 0x1234);
        assert!(new_timer.int_tima);
    }
}
