use std::{
    fmt::{self, Display, Formatter},
    io::{stdout, Write},
};

use crate::serial::SerialDevice;

pub struct StdoutDevice {
    flush: bool,
    callback: fn(buffer: &Vec<u8>),
}

impl StdoutDevice {
    pub fn new(flush: bool) -> Self {
        Self {
            flush,
            callback: |_| {},
        }
    }

    pub fn set_callback(&mut self, callback: fn(buffer: &Vec<u8>)) {
        self.callback = callback;
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
        let data = vec![byte];
        (self.callback)(&data);
    }

    fn allow_slave(&self) -> bool {
        false
    }

    fn description(&self) -> String {
        String::from("Stdout")
    }

    fn state(&self) -> String {
        String::from("")
    }
}

impl Default for StdoutDevice {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Display for StdoutDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Stdout")
    }
}
