use std::{
    any::Any,
    fmt::{self, Display, Formatter},
    io::{stdout, Write},
};

use crate::serial::SerialDevice;

pub struct StdoutDevice {
    /// Whether to flush the stdout after each write.
    flush: bool,

    /// Callback to call when a byte is received, useful
    /// to attach to external output devices.
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

    fn description(&self) -> String {
        String::from("Stdout")
    }

    fn state(&self) -> String {
        String::from("")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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
