use std::fmt::{self, Display, Formatter};

use crate::{ppu::PaletteAlpha, serial::SerialDevice, warnln};

const PRINTER_PALETTE: PaletteAlpha = [
    [0xff, 0xff, 0xff, 0xff],
    [0xaa, 0xaa, 0xaa, 0xff],
    [0x55, 0x55, 0x55, 0xff],
    [0x00, 0x00, 0x00, 0xff],
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrinterState {
    MagicBytes1 = 0x00,
    MagicBytes2 = 0x01,
    Identification = 0x02,
    Compression = 0x03,
    LengthLow = 0x04,
    LengthHigh = 0x05,
    Data = 0x06,
    ChecksumLow = 0x07,
    ChecksumHigh = 0x08,
    KeepAlive = 0x09,
    Status = 0x0a,
    Other = 0xff,
}

impl PrinterState {
    pub fn description(&self) -> &'static str {
        match self {
            PrinterState::MagicBytes1 => "Magic Bytes 1",
            PrinterState::MagicBytes2 => "Magic Bytes 2",
            PrinterState::Identification => "Identification",
            PrinterState::Compression => "Compression",
            PrinterState::LengthLow => "Length Low",
            PrinterState::LengthHigh => "Length High",
            PrinterState::Data => "Data",
            PrinterState::ChecksumLow => "Checksum Low",
            PrinterState::ChecksumHigh => "Checksum High",
            PrinterState::KeepAlive => "Keep Alive",
            PrinterState::Status => "Status",
            PrinterState::Other => "Other",
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0x00 => PrinterState::MagicBytes1,
            0x01 => PrinterState::MagicBytes2,
            0x02 => PrinterState::Identification,
            0x03 => PrinterState::Compression,
            0x04 => PrinterState::LengthLow,
            0x05 => PrinterState::LengthHigh,
            0x06 => PrinterState::Data,
            0x07 => PrinterState::ChecksumLow,
            0x08 => PrinterState::ChecksumHigh,
            0x09 => PrinterState::KeepAlive,
            0x0a => PrinterState::Status,
            _ => PrinterState::Other,
        }
    }
}

impl Display for PrinterState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for PrinterState {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrinterCommand {
    Init = 0x01,
    Print = 0x02,
    Data = 0x04,
    Status = 0x0f,
    Other = 0xff,
}

impl PrinterCommand {
    pub fn description(&self) -> &'static str {
        match self {
            PrinterCommand::Init => "Init",
            PrinterCommand::Print => "Print",
            PrinterCommand::Data => "Data",
            PrinterCommand::Status => "Status",
            PrinterCommand::Other => "Other",
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0x01 => PrinterCommand::Init,
            0x02 => PrinterCommand::Print,
            0x04 => PrinterCommand::Data,
            0x0f => PrinterCommand::Status,
            _ => PrinterCommand::Other,
        }
    }
}

impl Display for PrinterCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for PrinterCommand {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

pub struct PrinterDevice {
    state: PrinterState,
    command: PrinterCommand,
    compression: bool,
    command_length: u16,
    length_left: u16,
    checksum: u16,
    status: u8,
    byte_out: u8,
    data: [u8; 0x280],
    image: [u8; 160 * 200],
    image_offset: u16,
    callback: fn(image_buffer: &Vec<u8>),
}

impl PrinterDevice {
    pub fn new() -> Self {
        Self {
            state: PrinterState::MagicBytes1,
            command: PrinterCommand::Other,
            compression: false,
            command_length: 0,
            length_left: 0,
            checksum: 0x0,
            status: 0x0,
            byte_out: 0x0,
            data: [0x00; 0x280],
            image: [0x00; 160 * 200],
            image_offset: 0,
            callback: |_| {},
        }
    }

    pub fn reset(&mut self) {
        self.state = PrinterState::MagicBytes1;
        self.command = PrinterCommand::Other;
        self.compression = false;
        self.command_length = 0;
        self.length_left = 0;
        self.checksum = 0x0;
        self.status = 0x0;
        self.byte_out = 0x0;
        self.data = [0x00; 0x280];
        self.image = [0x00; 160 * 200];
        self.image_offset = 0;
    }

    pub fn set_callback(&mut self, callback: fn(image_buffer: &Vec<u8>)) {
        self.callback = callback;
    }

    fn run_command(&mut self, command: PrinterCommand) {
        match command {
            PrinterCommand::Init => {
                self.status = 0x00;
                self.byte_out = self.status;
                self.image_offset = 0;
            }
            PrinterCommand::Print => {
                let mut image_buffer = Vec::new();
                let palette_index = self.data[2];

                for index in 0..self.image_offset {
                    let value = self.image[index as usize];
                    let pixel_offset = (palette_index >> (value << 1)) & 0x03;
                    let pixel = PRINTER_PALETTE[pixel_offset as usize];
                    image_buffer.push(pixel[0]);
                    image_buffer.push(pixel[1]);
                    image_buffer.push(pixel[2]);
                    image_buffer.push(pixel[3]);
                }

                (self.callback)(&image_buffer);

                self.byte_out = self.status;
                self.status = 0x06;
            }
            PrinterCommand::Data => {
                if self.command_length == 0x280 {
                    self.flush_image();
                }
                // in case the command is of size 0 we assume this is
                // an EOF and we ignore this data operation
                else if self.command_length == 0x0 {
                } else {
                    warnln!(
                        "Printer: Wrong size for data: {:04x} bytes",
                        self.command_length
                    );
                }
                self.status = 0x08;
                self.byte_out = self.status;
            }
            PrinterCommand::Status => {
                self.byte_out = self.status;

                // in case the current status is printing let's
                // mark it as done, resetting the status back to
                // the original value
                if self.status == 0x06 {
                    self.status = 0x00;
                }
            }
            PrinterCommand::Other => {
                warnln!("Printer: Invalid command: {:02x}", self.state as u8);
            }
        }
    }

    fn flush_image(&mut self) {
        // sets the initial value of the index that will point to
        // the data that is going to be copied to the image buffer
        let mut index = 0;

        // iterates over the two rows that are going to be printed
        // keep in mind that the printer only allows 2 lines at each
        // time to be printed
        for _ in 0..2 {
            for col in 0..20 {
                for y in 0..8 {
                    let mut first = self.data[index];
                    let mut second = self.data[index + 1];
                    for x in 0..8 {
                        let offset = self.image_offset as usize + (col * 8) + (y * 160) + x;
                        self.image[offset] = (first >> 7) | ((second >> 6) & 0x02);

                        first <<= 1;
                        second <<= 1;
                    }
                    index += 2
                }
            }

            self.image_offset += 160 * 8;
        }
    }
}

impl SerialDevice for PrinterDevice {
    fn send(&mut self) -> u8 {
        self.byte_out
    }

    fn receive(&mut self, byte: u8) {
        self.byte_out = 0x00;

        match self.state {
            PrinterState::MagicBytes1 => {
                if byte != 0x88 {
                    warnln!("Printer: Invalid magic byte 1: {:02x}", byte);
                    #[allow(unreachable_code)]
                    {
                        return;
                    }
                }
                self.command = PrinterCommand::Other;
                self.command_length = 0;
            }
            PrinterState::MagicBytes2 => {
                if byte != 0x33 {
                    if byte != 0x88 {
                        self.state = PrinterState::MagicBytes1;
                    }
                    warnln!("Printer: Invalid magic byte 2: {:02x}", byte);
                    #[allow(unreachable_code)]
                    {
                        return;
                    }
                }
            }
            PrinterState::Identification => self.command = PrinterCommand::from_u8(byte),
            PrinterState::Compression => {
                self.compression = byte & 0x01 == 0x01;
                if self.compression {
                    warnln!("Printer: Using compressed data, currently unsupported");
                }
            }
            PrinterState::LengthLow => self.length_left = byte as u16,
            PrinterState::LengthHigh => self.length_left |= (byte as u16) << 8,
            PrinterState::Data => {
                self.data[self.command_length as usize] = byte;
                self.command_length += 1;
                self.length_left -= 1;
            }
            PrinterState::ChecksumLow => self.checksum = byte as u16,
            PrinterState::ChecksumHigh => {
                self.checksum |= (byte as u16) << 8;
                self.byte_out = 0x81;
            }
            PrinterState::KeepAlive => {
                self.run_command(self.command);
            }
            PrinterState::Status => {
                self.state = PrinterState::MagicBytes1;
                return;
            }
            PrinterState::Other => {
                warnln!("Printer: Invalid state: {:02x}", self.state as u8);
                #[allow(unreachable_code)]
                {
                    return;
                }
            }
        }

        if self.state != PrinterState::Data {
            self.state = PrinterState::from_u8(self.state as u8 + 1);
        }

        if self.state == PrinterState::Data && self.length_left == 0 {
            self.state = PrinterState::from_u8(self.state as u8 + 1);
        }
    }

    fn allow_slave(&self) -> bool {
        false
    }

    fn description(&self) -> String {
        format!("Printer [{}]", self.command)
    }

    fn state(&self) -> String {
        self.command.to_string()
    }
}

impl Default for PrinterDevice {
    fn default() -> Self {
        Self::new()
    }
}
