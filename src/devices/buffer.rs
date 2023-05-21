use crate::serial::SerialDevice;

use std::fmt::{self, Display, Formatter};

pub struct BufferDevice {
    buffer: Vec<u8>,
    callback: fn(image_buffer: &Vec<u8>),
}

impl BufferDevice {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            callback: |_| {},
        }
    }

    pub fn set_callback(&mut self, callback: fn(image_buffer: &Vec<u8>)) {
        self.callback = callback;
    }

    pub fn buffer(&self) -> &Vec<u8> {
        &self.buffer
    }
}

impl SerialDevice for BufferDevice {
    fn send(&mut self) -> u8 {
        0xff
    }

    fn receive(&mut self, byte: u8) {
        self.buffer.push(byte);
        let data = vec![byte];
        (self.callback)(&data);
    }

    fn allow_slave(&self) -> bool {
        false
    }

    fn description(&self) -> String {
        String::from("Buffer")
    }

    fn state(&self) -> String {
        let buffer = self.buffer.clone();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for BufferDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for BufferDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer")
    }
}
