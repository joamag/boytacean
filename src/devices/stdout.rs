use std::io::{stdout, Write};

use crate::serial::SerialDevice;

pub struct StdoutDevice {
    flush: bool,
}

impl StdoutDevice {
    pub fn new(flush: bool) -> Self {
        Self { flush }
    }
}

impl SerialDevice for StdoutDevice {
    fn send(&mut self) -> u8 {
        0xff
    }

    fn receive(&mut self, byte: u8) {
        print!("{}", byte as char);
        if self.flush {
            stdout().flush().unwrap();
        }
    }

    fn allow_slave(&self) -> bool {
        false
    }
}

impl Default for StdoutDevice {
    fn default() -> Self {
        Self::new(true)
    }
}
