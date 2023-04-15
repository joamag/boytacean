use crate::serial::SerialDevice;

pub struct PrinterDevice {
}

impl PrinterDevice {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PrinterDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialDevice for PrinterDevice {
    fn send(&mut self) -> u8 {
        0xff
    }

    fn receive(&mut self, byte: u8) {
        print!("{}", byte as char);
        // @TODO: implement this one
    }
}
