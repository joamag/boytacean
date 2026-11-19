//! Sans-IO dumper of Game Boy cartridges through a GBxCart RW.
//!
//! The [`Dumper`] never touches the device, it hands out the next
//! [`Exchange`], the bytes to be sent and the number of bytes expected
//! back, and consumes the reply. This way every frontend runs the same
//! dumping logic, whatever its transport to the device is.
//!
//! The dump is read-only, the cartridge bus is only written to drive
//! the bank switching of the MBC, never its ROM nor its RAM.

use std::{
    collections::VecDeque,
    io::{Read, Write},
    mem::take,
};

use boytacean_common::error::Error;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::{
    gbxcart::{
        cart_write, set_variable, Firmware, ACCESS_MODE_RAM_READ, ACCESS_MODE_ROM_READ, ACK,
        ADDRESS, CART_MODE, DMG_ACCESS_MODE, DMG_CART_READ, DMG_MBC_RESET, DMG_READ_CS_PULSE,
        DMG_READ_METHOD, DMG_WRITE_CS_PULSE, QUERY_FW_INFO, SET_ADDR_AS_INPUTS, SET_MODE_DMG,
        SET_VOLTAGE_5V, TRANSFER_SIZE,
    },
    header::{Header, HEADER_ADDRESS, HEADER_SIZE, RAM_BANK_SIZE, ROM_BANK_SIZE},
    mapper::{BusWrite, Mapper, RAM_BANK_ADDRESS},
};

/// The number of bytes read from the cartridge by each read command.
pub const CHUNK_SIZE: usize = 64;

/// Where the reply to a command goes once it is received.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    None,
    Firmware,
    Ack,
    Header,
    Rom,
    Ram,
}

struct Command {
    data: Vec<u8>,
    reply: usize,
    target: Target,
}

impl Command {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            reply: 0,
            target: Target::None,
        }
    }

    fn with_reply(data: Vec<u8>, reply: usize, target: Target) -> Self {
        Self {
            data,
            reply,
            target,
        }
    }
}

/// The bytes to be sent to the device, followed by the read of the
/// number of bytes it is expected to reply with (possibly none).
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Exchange {
    send: Vec<u8>,
    expect: usize,
}

impl Exchange {
    pub fn send(&self) -> &[u8] {
        &self.send
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Exchange {
    pub fn expect(&self) -> usize {
        self.expect
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Exchange {
    pub fn send_eager(&self) -> Vec<u8> {
        self.send.clone()
    }
}

/// The result of dumping a cartridge, its ROM and the contents of
/// its battery backed RAM (if any).
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Dump {
    title: String,
    rom_data: Vec<u8>,
    ram_data: Option<Vec<u8>>,
    checksum_valid: bool,
}

impl Dump {
    pub fn rom_data(&self) -> &Vec<u8> {
        &self.rom_data
    }

    pub fn ram_data(&self) -> Option<&Vec<u8>> {
        self.ram_data.as_ref()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Dump {
    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn has_ram(&self) -> bool {
        self.ram_data.is_some()
    }

    /// If the global checksum of the header matches the ROM, which is
    /// often not the case for reproduction cartridges.
    pub fn checksum_valid(&self) -> bool {
        self.checksum_valid
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Dump {
    pub fn rom_data_eager(&self) -> Vec<u8> {
        self.rom_data.clone()
    }

    pub fn ram_data_eager(&self) -> Vec<u8> {
        self.ram_data.clone().unwrap_or_default()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Dumper {
    commands: VecDeque<Command>,
    pending: Option<(Target, usize)>,
    firmware: Option<Firmware>,
    headers: Vec<Vec<u8>>,
    header: Option<Header>,
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    rom_size: usize,
    ram_size: usize,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Dumper {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        let mut commands = VecDeque::new();
        commands.push_back(Command::with_reply(
            vec![QUERY_FW_INFO],
            Firmware::REPLY_SIZE,
            Target::Firmware,
        ));
        Self {
            commands,
            pending: None,
            firmware: None,
            headers: vec![],
            header: None,
            rom_data: vec![],
            ram_data: vec![],
            rom_size: 0,
            ram_size: 0,
        }
    }

    /// Obtains the next exchange with the device, batching the commands
    /// without reply with the next one that has a reply.
    ///
    /// Returns `None` when the dump is finished, or when the reply to
    /// the previous exchange has not been fed yet.
    pub fn next_exchange(&mut self) -> Option<Exchange> {
        if self.pending.is_some() {
            return None;
        }
        let mut send = vec![];
        let mut expect = (Target::None, 0);
        while let Some(command) = self.commands.pop_front() {
            send.extend(command.data);
            if command.reply > 0 {
                expect = (command.target, command.reply);
                break;
            }
        }
        if send.is_empty() {
            return None;
        }
        self.pending = Some(expect);
        Some(Exchange {
            send,
            expect: expect.1,
        })
    }

    /// The progress of the dump, from `0.0` to `1.0`, which is only
    /// known once the header has been read.
    pub fn progress(&self) -> f32 {
        let total = self.rom_size + self.ram_size;
        if total == 0 {
            return 0.0;
        }
        (self.rom_data.len() + self.ram_data.len()) as f32 / total as f32
    }

    /// If the dump is complete, with every command run and all of the
    /// ROM and RAM of the cartridge read.
    pub fn is_finished(&self) -> bool {
        self.header.is_some()
            && self.commands.is_empty()
            && self.pending.is_none()
            && self.rom_data.len() == self.rom_size
            && self.ram_data.len() == self.ram_size
    }

    /// The description of the device, once its firmware is known.
    pub fn device(&self) -> Option<String> {
        self.firmware.as_ref().map(|firmware| format!("{firmware}"))
    }
}

impl Dumper {
    /// Feeds the reply to the last exchange, which may plan the next
    /// commands (eg: the reading of the ROM once the header is known).
    pub fn feed(&mut self, data: &[u8]) -> Result<(), Error> {
        let (target, expect) = self.pending.take().ok_or(Error::CustomError(String::from(
            "No exchange with the device in progress",
        )))?;
        if data.len() != expect {
            return Err(Error::DataError(format!(
                "Expected {expect} bytes from the device, got {}",
                data.len()
            )));
        }
        match target {
            Target::None => {}
            Target::Firmware => self.feed_firmware(data)?,
            Target::Ack => {
                if data != [ACK] {
                    return Err(Error::DataError(String::from(
                        "Invalid acknowledgement from the device",
                    )));
                }
            }
            Target::Header => self.feed_header(data)?,
            Target::Rom => self.rom_data.extend_from_slice(data),
            Target::Ram => self.ram_data.extend_from_slice(data),
        }
        Ok(())
    }

    /// Finishes the dump, handing out the ROM and the RAM read from the
    /// cartridge, a dumper is spent after being finished.
    pub fn finish(&mut self) -> Result<Dump, Error> {
        if !self.is_finished() {
            return Err(Error::CustomError(String::from(
                "The dump of the cartridge is not complete",
            )));
        }
        let header = self.header.take().unwrap();
        let rom_data = take(&mut self.rom_data);
        let ram_data = take(&mut self.ram_data);
        Ok(Dump {
            title: header.title(),
            checksum_valid: header.matches_rom(&rom_data),
            rom_data,
            ram_data: if self.ram_size > 0 {
                Some(ram_data)
            } else {
                None
            },
        })
    }

    /// Runs the complete dump over a blocking transport to the device
    /// (eg: a serial port), calling back with the progress after each
    /// exchange.
    pub fn run<T: Read + Write, F: FnMut(f32)>(
        &mut self,
        transport: &mut T,
        mut callback: F,
    ) -> Result<Dump, Error> {
        while let Some(exchange) = self.next_exchange() {
            transport.write_all(exchange.send())?;
            transport.flush()?;
            let mut data = vec![0x00; exchange.expect()];
            transport.read_exact(&mut data)?;
            self.feed(&data)?;
            callback(self.progress());
        }
        self.finish()
    }

    fn feed_firmware(&mut self, data: &[u8]) -> Result<(), Error> {
        let firmware = Firmware::from_data(data)?;
        if !firmware.is_supported() {
            return Err(Error::CustomError(format!(
                "Unsupported firmware {}{}, only the L firmware is supported",
                firmware.kind(),
                firmware.version()
            )));
        }
        self.firmware = Some(firmware);

        // sets the slot in Game Boy mode at 5V, resets the MBC and
        // reads the header twice, so that an unstable connection to
        // the cartridge is caught before the ROM is read
        self.push(vec![SET_MODE_DMG, SET_VOLTAGE_5V]);
        self.push(set_variable(DMG_READ_METHOD, 0x01));
        self.push(set_variable(CART_MODE, 0x01));
        self.push(set_variable(DMG_READ_CS_PULSE, 0x00));
        self.push(set_variable(DMG_WRITE_CS_PULSE, 0x00));
        self.commands
            .push_back(Command::with_reply(vec![DMG_MBC_RESET], 1, Target::Ack));
        for _ in 0..2 {
            self.push_read(HEADER_ADDRESS, HEADER_SIZE, ACCESS_MODE_ROM_READ);
            self.push_reads(1, HEADER_SIZE, Target::Header);
        }
        Ok(())
    }

    fn feed_header(&mut self, data: &[u8]) -> Result<(), Error> {
        self.headers.push(data.to_vec());
        if self.headers.len() < 2 {
            return Ok(());
        }
        if self.headers[0] != self.headers[1] {
            return Err(Error::CustomError(String::from(
                "Unstable connection to the cartridge, clean its pins and reinsert it",
            )));
        }
        let header = Header::from_data(data)?;
        if !header.is_valid() {
            return Err(Error::CustomError(String::from(
                "Invalid cartridge header, check that a cartridge is inserted and clean its pins",
            )));
        }
        let mapper = Mapper::from_cartridge_type(header.cartridge_type())?;
        let rom_banks = header.rom_banks()?;
        if rom_banks > mapper.max_rom_banks() {
            return Err(Error::CustomError(format!(
                "Unsupported ROM size for {}: {rom_banks} banks",
                mapper.description()
            )));
        }
        let ram_size = if header.has_battery() {
            mapper.ram_size(header.ram_size()?)
        } else {
            0
        };
        let ram_banks = mapper.ram_banks(ram_size);
        if ram_banks > mapper.max_ram_banks() {
            return Err(Error::CustomError(format!(
                "Unsupported RAM size for {}: {ram_banks} banks",
                mapper.description()
            )));
        }

        // reads every ROM bank through the bank switching of the MBC
        for bank in 0..rom_banks {
            self.push_writes(&mapper.rom_bank_writes(bank));
            self.push_read(
                mapper.rom_bank_address(bank),
                CHUNK_SIZE,
                ACCESS_MODE_ROM_READ,
            );
            self.push_reads(ROM_BANK_SIZE / CHUNK_SIZE, CHUNK_SIZE, Target::Rom);
        }

        // reads the battery backed RAM (if any), the chip select line
        // must be pulsed for the RAM to reply
        if ram_size > 0 {
            self.push_writes(&mapper.ram_enable_writes());
            for bank in 0..ram_banks {
                let size = (ram_size - bank as usize * RAM_BANK_SIZE).min(RAM_BANK_SIZE);
                self.push_writes(&mapper.ram_bank_writes(bank));
                self.push_read(RAM_BANK_ADDRESS, CHUNK_SIZE, ACCESS_MODE_RAM_READ);
                self.push(set_variable(DMG_READ_CS_PULSE, 0x01));
                self.push_reads(size / CHUNK_SIZE, CHUNK_SIZE, Target::Ram);
                self.push(set_variable(DMG_READ_CS_PULSE, 0x00));
            }
            self.push_writes(&mapper.ram_disable_writes());
        }

        self.push(vec![SET_ADDR_AS_INPUTS]);

        self.rom_size = rom_banks as usize * ROM_BANK_SIZE;
        self.ram_size = ram_size;
        self.header = Some(header);
        Ok(())
    }

    fn push(&mut self, data: Vec<u8>) {
        self.commands.push_back(Command::new(data));
    }

    fn push_writes(&mut self, writes: &[BusWrite]) {
        for (address, value) in writes {
            self.push(cart_write(*address, *value));
        }
    }

    fn push_read(&mut self, address: u16, size: usize, access_mode: u32) {
        self.push(set_variable(TRANSFER_SIZE, size as u32));
        self.push(set_variable(ADDRESS, address as u32));
        self.push(set_variable(DMG_ACCESS_MODE, access_mode));
    }

    fn push_reads(&mut self, count: usize, size: usize, target: Target) {
        for _ in 0..count {
            self.commands
                .push_back(Command::with_reply(vec![DMG_CART_READ], size, target));
        }
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Dumper {
    pub fn feed_wa(&mut self, data: &[u8]) -> Result<(), String> {
        self.feed(data)?;
        Ok(())
    }

    pub fn finish_wa(&mut self) -> Result<Dump, String> {
        Ok(self.finish()?)
    }
}

impl Default for Dumper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        io::{self, Read, Write},
    };

    use boytacean_common::error::Error;

    use super::{Dump, Dumper, Exchange, CHUNK_SIZE};
    use crate::{
        gbxcart::{
            ACCESS_MODE_RAM_READ, DMG_CART_READ, DMG_CART_WRITE, DMG_MBC_RESET, QUERY_FW_INFO,
            SET_ADDR_AS_INPUTS, SET_MODE_DMG, SET_VARIABLE, SET_VOLTAGE_5V,
        },
        header::{NINTENDO_LOGO, RAM_BANK_SIZE, ROM_BANK_SIZE},
        mapper::Mapper,
    };

    /// The reply to the firmware query of a GBxCart RW v1.3 running
    /// the L1 firmware, as captured from the device.
    const FIRMWARE_L1: [u8; 9] = [0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea];

    /// A fake GBxCart RW running the L firmware, with a cartridge whose
    /// MBC is simulated independently of [`Mapper`], so that a wrong
    /// bank switching produces a wrong dump.
    struct FakeDevice {
        firmware: Vec<u8>,
        mapper: Mapper,
        rom: Vec<u8>,
        ram: Vec<u8>,
        ram_enabled: bool,
        rom_bank: u16,
        upper: u8,
        mode: u8,
        ram_bank: u8,
        transfer_size: usize,
        address: u32,
        access_mode: u32,
        cs_pulse: bool,
        input: Vec<u8>,
        output: VecDeque<u8>,
        writes: Vec<(u16, u8)>,
        commands: Vec<u8>,
        broken_reads: usize,
    }

    impl FakeDevice {
        fn new(mapper: Mapper, rom: Vec<u8>, ram: Vec<u8>) -> Self {
            Self {
                firmware: FIRMWARE_L1.to_vec(),
                mapper,
                rom,
                ram,
                ram_enabled: false,
                rom_bank: 1,
                upper: 0,
                mode: 0,
                ram_bank: 0,
                transfer_size: 1,
                address: 0,
                access_mode: 0,
                cs_pulse: false,
                input: vec![],
                output: VecDeque::new(),
                writes: vec![],
                commands: vec![],
                broken_reads: usize::MAX,
            }
        }

        fn reset(&mut self) {
            self.ram_enabled = false;
            self.rom_bank = 1;
            self.upper = 0;
            self.mode = 0;
            self.ram_bank = 0;
        }

        fn write_bus(&mut self, address: u16, value: u8) {
            self.writes.push((address, value));
            match (self.mapper, address) {
                (Mapper::NoMbc, _) => {}
                (Mapper::Mbc1, 0x0000..=0x1fff) => self.ram_enabled = value & 0x0f == 0x0a,
                (Mapper::Mbc1, 0x2000..=0x3fff) => self.rom_bank = (value & 0x1f) as u16,
                (Mapper::Mbc1, 0x4000..=0x5fff) => self.upper = value & 0x03,
                (Mapper::Mbc1, 0x6000..=0x7fff) => self.mode = value & 0x01,
                (Mapper::Mbc2, 0x0000..=0x3fff) if address & 0x0100 == 0 => {
                    self.ram_enabled = value & 0x0f == 0x0a
                }
                (Mapper::Mbc2, 0x0000..=0x3fff) => self.rom_bank = (value & 0x0f) as u16,
                (Mapper::Mbc3, 0x0000..=0x1fff) => self.ram_enabled = value & 0x0f == 0x0a,
                (Mapper::Mbc3, 0x2000..=0x3fff) => self.rom_bank = value as u16,
                (Mapper::Mbc3, 0x4000..=0x5fff) => self.ram_bank = value,
                (Mapper::Mbc5, 0x0000..=0x1fff) => self.ram_enabled = value & 0x0f == 0x0a,
                (Mapper::Mbc5, 0x2000..=0x2fff) => {
                    self.rom_bank = (self.rom_bank & 0x100) | value as u16
                }
                (Mapper::Mbc5, 0x3000..=0x3fff) => {
                    self.rom_bank = (self.rom_bank & 0x0ff) | ((value as u16 & 0x01) << 8)
                }
                (Mapper::Mbc5, 0x4000..=0x5fff) => self.ram_bank = value & 0x0f,
                _ => {}
            }
        }

        fn read_rom(&self, address: u32) -> u8 {
            let banks = self.rom.len() / ROM_BANK_SIZE;
            let offset = address as usize & 0x3fff;
            let bank = if address < 0x4000 {
                match self.mapper {
                    Mapper::Mbc1 if self.mode == 1 => (self.upper as usize) << 5,
                    _ => 0,
                }
            } else {
                match self.mapper {
                    Mapper::NoMbc => 1,
                    Mapper::Mbc1 => ((self.upper as usize) << 5) | (self.rom_bank as usize).max(1),
                    Mapper::Mbc2 | Mapper::Mbc3 => (self.rom_bank as usize).max(1),
                    Mapper::Mbc5 => self.rom_bank as usize,
                }
            };
            self.rom[(bank % banks) * ROM_BANK_SIZE + offset]
        }

        fn read_ram(&self, address: u32) -> u8 {
            let enabled = self.ram_enabled || self.mapper == Mapper::NoMbc;
            if !self.cs_pulse || !enabled || self.ram.is_empty() {
                return 0xff;
            }
            let offset = address as usize - 0xa000;
            match self.mapper {
                Mapper::Mbc2 => (self.ram[offset % self.ram.len()] & 0x0f) | 0xf0,
                _ => {
                    let bank = match self.mapper {
                        Mapper::Mbc1 if self.mode == 1 => self.upper as usize,
                        Mapper::Mbc3 | Mapper::Mbc5 => self.ram_bank as usize,
                        _ => 0,
                    };
                    self.ram[(bank * RAM_BANK_SIZE + offset) % self.ram.len()]
                }
            }
        }

        fn process(&mut self) {
            loop {
                let size = match self.input.first() {
                    None => return,
                    Some(&SET_VARIABLE) => 10,
                    Some(&DMG_CART_WRITE) => 6,
                    Some(_) => 1,
                };
                if self.input.len() < size {
                    return;
                }
                let command: Vec<u8> = self.input.drain(..size).collect();
                self.commands.push(command[0]);
                match command[0] {
                    QUERY_FW_INFO => self.output.extend(self.firmware.clone()),
                    SET_MODE_DMG | SET_VOLTAGE_5V | SET_ADDR_AS_INPUTS => {}
                    SET_VARIABLE => {
                        let key =
                            u32::from_be_bytes([command[2], command[3], command[4], command[5]]);
                        let value =
                            u32::from_be_bytes([command[6], command[7], command[8], command[9]]);
                        match (command[1], key) {
                            (2, 0x00) => self.transfer_size = value as usize,
                            (4, 0x00) => self.address = value,
                            (1, 0x01) => self.access_mode = value,
                            (1, 0x08) => self.cs_pulse = value == 1,
                            _ => {}
                        }
                    }
                    DMG_CART_WRITE => {
                        let address =
                            u32::from_be_bytes([command[1], command[2], command[3], command[4]]);
                        self.write_bus(address as u16, command[5]);
                    }
                    DMG_MBC_RESET => {
                        self.reset();
                        self.output.push_back(0x01);
                    }
                    DMG_CART_READ => {
                        if self.broken_reads == 0 {
                            return;
                        }
                        self.broken_reads -= 1;
                        for index in 0..self.transfer_size as u32 {
                            let byte = if self.access_mode == ACCESS_MODE_RAM_READ {
                                self.read_ram(self.address + index)
                            } else {
                                self.read_rom(self.address + index)
                            };
                            self.output.push_back(byte);
                        }
                        self.address += self.transfer_size as u32;
                    }
                    command => panic!("Unexpected command 0x{command:02x}"),
                }
            }
        }
    }

    impl Write for FakeDevice {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.input.extend_from_slice(buf);
            self.process();
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for FakeDevice {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let count = buf.len().min(self.output.len());
            for byte in buf.iter_mut().take(count) {
                *byte = self.output.pop_front().unwrap();
            }
            Ok(count)
        }
    }

    /// Builds a ROM with the provided header values, each byte holding
    /// its bank number mixed with its offset so that reading a wrong
    /// bank is caught, with valid header and global checksums.
    fn build_rom(cartridge_type: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
        let banks = 2usize << rom_size;
        let mut rom: Vec<u8> = (0..banks * ROM_BANK_SIZE)
            .map(|index| ((index / ROM_BANK_SIZE) as u8) ^ ((index >> 4) as u8))
            .collect();

        // the number of each bank in its first bytes, so that no two
        // banks are the same (eg: the banks 0x001 and 0x101 of MBC5)
        for bank in 0..banks {
            let offset = bank * ROM_BANK_SIZE;
            rom[offset..offset + 2].copy_from_slice(&(bank as u16).to_be_bytes());
        }
        rom[0x0100..0x0150].fill(0x00);
        rom[0x0104..0x0134].copy_from_slice(&NINTENDO_LOGO);
        rom[0x0134..0x0138].copy_from_slice(b"GAME");
        rom[0x0147] = cartridge_type;
        rom[0x0148] = rom_size;
        rom[0x0149] = ram_size;
        rom[0x014d] = rom[0x0134..0x014d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        let checksum = rom
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 0x014e && *index != 0x014f)
            .fold(0u16, |sum, (_, &byte)| sum.wrapping_add(byte as u16));
        rom[0x014e..0x0150].copy_from_slice(&checksum.to_be_bytes());
        rom
    }

    fn build_ram(size: usize) -> Vec<u8> {
        (0..size)
            .map(|index| (index * 7 + index / 251) as u8)
            .collect()
    }

    /// Runs the complete dump of the cartridge in the provided device.
    fn dump(device: &mut FakeDevice) -> Result<Dump, Error> {
        Dumper::new().run(device, |_| {})
    }

    /// Sends the exchange to the device and feeds its reply.
    fn complete(dumper: &mut Dumper, device: &mut FakeDevice, exchange: Exchange) {
        device.write_all(exchange.send()).unwrap();
        let mut data = vec![0x00; exchange.expect()];
        device.read_exact(&mut data).unwrap();
        dumper.feed(&data).unwrap();
    }

    /// Runs the dumper until the exchange that expects the provided
    /// number of bytes, returning it without running it.
    fn advance(dumper: &mut Dumper, device: &mut FakeDevice, expect: usize) -> Exchange {
        loop {
            let exchange = dumper.next_exchange().unwrap();
            if exchange.expect() == expect {
                return exchange;
            }
            complete(dumper, device, exchange);
        }
    }

    #[test]
    fn test_exchange_send() {
        let mut dumper = Dumper::new();
        assert_eq!(dumper.next_exchange().unwrap().send(), &[QUERY_FW_INFO]);
    }

    #[test]
    fn test_exchange_expect() {
        let mut dumper = Dumper::new();
        assert_eq!(dumper.next_exchange().unwrap().expect(), 9);
    }

    #[test]
    fn test_dump_rom_data() {
        let rom = build_rom(0x00, 0x00, 0x00);
        let mut device = FakeDevice::new(Mapper::NoMbc, rom.clone(), vec![]);
        assert_eq!(dump(&mut device).unwrap().rom_data(), &rom);
    }

    #[test]
    fn test_dump_ram_data() {
        let ram = build_ram(RAM_BANK_SIZE);
        let mut device = FakeDevice::new(Mapper::Mbc1, build_rom(0x03, 0x01, 0x02), ram.clone());
        assert_eq!(dump(&mut device).unwrap().ram_data(), Some(&ram));
    }

    #[test]
    fn test_dump_ram_data_none() {
        let mut device = FakeDevice::new(Mapper::Mbc1, build_rom(0x01, 0x01, 0x00), vec![]);
        assert_eq!(dump(&mut device).unwrap().ram_data(), None);
    }

    #[test]
    fn test_dump_title() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        assert_eq!(dump(&mut device).unwrap().title(), "GAME");
    }

    #[test]
    fn test_dump_has_ram() {
        let mut device = FakeDevice::new(
            Mapper::Mbc1,
            build_rom(0x03, 0x01, 0x02),
            build_ram(RAM_BANK_SIZE),
        );
        assert!(dump(&mut device).unwrap().has_ram());
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        assert!(!dump(&mut device).unwrap().has_ram());
    }

    #[test]
    fn test_dump_checksum_valid() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        assert!(dump(&mut device).unwrap().checksum_valid());
    }

    #[test]
    fn test_dump_checksum_valid_mismatch() {
        let mut rom = build_rom(0x00, 0x00, 0x00);
        rom[0x7000] = rom[0x7000].wrapping_add(1);
        let mut device = FakeDevice::new(Mapper::NoMbc, rom.clone(), vec![]);
        let dump = dump(&mut device).unwrap();
        assert!(!dump.checksum_valid());
        assert_eq!(dump.rom_data(), &rom);
    }

    #[test]
    fn test_new() {
        let mut dumper = Dumper::new();
        assert_eq!(
            dumper.next_exchange(),
            Some(Exchange {
                send: vec![QUERY_FW_INFO],
                expect: 9
            })
        );
        assert!(!dumper.is_finished());
        assert_eq!(dumper.device(), None);
    }

    #[test]
    fn test_next_exchange_batches() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        dumper.feed(&FIRMWARE_L1).unwrap();
        let exchange = dumper.next_exchange().unwrap();
        assert_eq!(&exchange.send()[0..2], &[SET_MODE_DMG, SET_VOLTAGE_5V]);
        assert_eq!(exchange.send().last(), Some(&DMG_MBC_RESET));
        assert_eq!(exchange.expect(), 1);
        device.write_all(exchange.send()).unwrap();
        assert_eq!(device.output.len(), 1);
    }

    #[test]
    fn test_next_exchange_pending() {
        let mut dumper = Dumper::new();
        assert!(dumper.next_exchange().is_some());
        assert_eq!(dumper.next_exchange(), None);
    }

    #[test]
    fn test_next_exchange_finished() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        dumper.run(&mut device, |_| {}).unwrap();
        assert_eq!(dumper.next_exchange(), None);
    }

    #[test]
    fn test_next_exchange_release() {
        let mut device = FakeDevice::new(
            Mapper::Mbc1,
            build_rom(0x03, 0x01, 0x02),
            build_ram(RAM_BANK_SIZE),
        );
        dump(&mut device).unwrap();
        assert_eq!(device.commands.last(), Some(&SET_ADDR_AS_INPUTS));
        assert!(device.input.is_empty());
        assert!(device.output.is_empty());
    }

    #[test]
    fn test_progress() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        assert_eq!(dumper.progress(), 0.0);
        let exchange = advance(&mut dumper, &mut device, CHUNK_SIZE);
        assert_eq!(dumper.progress(), 0.0);
        complete(&mut dumper, &mut device, exchange);
        for _ in 1..256 {
            let exchange = dumper.next_exchange().unwrap();
            complete(&mut dumper, &mut device, exchange);
        }
        assert_eq!(dumper.progress(), 0.5);
    }

    #[test]
    fn test_progress_callback() {
        let mut device = FakeDevice::new(
            Mapper::Mbc1,
            build_rom(0x03, 0x01, 0x02),
            build_ram(RAM_BANK_SIZE),
        );
        let mut values = vec![];
        Dumper::new()
            .run(&mut device, |progress| values.push(progress))
            .unwrap();
        assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(values.last(), Some(&1.0));
    }

    #[test]
    fn test_is_finished() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        assert!(!dumper.is_finished());
        let exchange = advance(&mut dumper, &mut device, CHUNK_SIZE);
        assert!(!dumper.is_finished());
        complete(&mut dumper, &mut device, exchange);
        while let Some(exchange) = dumper.next_exchange() {
            assert!(!dumper.is_finished());
            complete(&mut dumper, &mut device, exchange);
        }
        assert!(dumper.is_finished());
    }

    #[test]
    fn test_device() {
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        dumper.feed(&FIRMWARE_L1).unwrap();
        assert_eq!(
            dumper.device(),
            Some(String::from("GBxCart RW v1.3 (firmware L1)"))
        );
    }

    #[test]
    fn test_feed_no_exchange() {
        let mut dumper = Dumper::new();
        assert_eq!(
            dumper.feed(&FIRMWARE_L1),
            Err(Error::CustomError(String::from(
                "No exchange with the device in progress"
            )))
        );
    }

    #[test]
    fn test_feed_short_reply() {
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        assert_eq!(
            dumper.feed(&FIRMWARE_L1[..4]),
            Err(Error::DataError(String::from(
                "Expected 9 bytes from the device, got 4"
            )))
        );
    }

    #[test]
    fn test_feed_unsupported_firmware() {
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        let mut data = FIRMWARE_L1;
        data[1] = b'R';
        data[3] = 0x1f;
        assert_eq!(
            dumper.feed(&data),
            Err(Error::CustomError(String::from(
                "Unsupported firmware R31, only the L firmware is supported"
            )))
        );
    }

    #[test]
    fn test_feed_invalid_firmware() {
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        assert!(dumper.feed(&[0xff; 9]).is_err());
    }

    #[test]
    fn test_feed_invalid_ack() {
        let mut dumper = Dumper::new();
        dumper.next_exchange().unwrap();
        dumper.feed(&FIRMWARE_L1).unwrap();
        dumper.next_exchange().unwrap();
        assert_eq!(
            dumper.feed(&[0x00]),
            Err(Error::DataError(String::from(
                "Invalid acknowledgement from the device"
            )))
        );
    }

    #[test]
    fn test_feed_unstable_header() {
        let rom = build_rom(0x00, 0x00, 0x00);
        let mut device = FakeDevice::new(Mapper::NoMbc, rom.clone(), vec![]);
        let mut dumper = Dumper::new();
        let exchange = advance(&mut dumper, &mut device, 0x50);
        complete(&mut dumper, &mut device, exchange);
        let exchange = dumper.next_exchange().unwrap();
        assert_eq!(exchange.expect(), 0x50);
        let mut unstable = rom[0x0100..0x0150].to_vec();
        unstable[0x10] ^= 0x01;
        assert_eq!(
            dumper.feed(&unstable),
            Err(Error::CustomError(String::from(
                "Unstable connection to the cartridge, clean its pins and reinsert it"
            )))
        );
    }

    #[test]
    fn test_feed_no_cartridge() {
        let mut device = FakeDevice::new(Mapper::NoMbc, vec![0xff; 0x8000], vec![]);
        assert_eq!(
            dump(&mut device).err(),
            Some(Error::CustomError(String::from(
                "Invalid cartridge header, check that a cartridge is inserted and clean its pins"
            )))
        );
    }

    #[test]
    fn test_feed_unsupported_mapper() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x20, 0x00, 0x00), vec![]);
        assert_eq!(
            dump(&mut device).err(),
            Some(Error::CustomError(String::from(
                "Unsupported cartridge type: 0x20"
            )))
        );
    }

    #[test]
    fn test_feed_invalid_rom_size() {
        let mut rom = build_rom(0x00, 0x00, 0x00);
        rom[0x0148] = 0x52;
        rom[0x014d] = rom[0x0134..0x014d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        let mut device = FakeDevice::new(Mapper::NoMbc, rom, vec![]);
        assert_eq!(dump(&mut device).err(), Some(Error::RomSize));
    }

    #[test]
    fn test_feed_invalid_ram_size() {
        let mut rom = build_rom(0x03, 0x00, 0x00);
        rom[0x0149] = 0x06;
        rom[0x014d] = rom[0x0134..0x014d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        let mut device = FakeDevice::new(Mapper::Mbc1, rom, vec![]);
        assert_eq!(
            dump(&mut device).err(),
            Some(Error::DataError(String::from("Invalid RAM size: 0x06")))
        );
    }

    #[test]
    fn test_feed_unsupported_rom_size() {
        let mut device = FakeDevice::new(Mapper::Mbc2, build_rom(0x06, 0x04, 0x00), vec![]);
        assert_eq!(
            dump(&mut device).err(),
            Some(Error::CustomError(String::from(
                "Unsupported ROM size for MBC2: 32 banks"
            )))
        );
    }

    #[test]
    fn test_feed_unsupported_ram_size() {
        let mut device = FakeDevice::new(Mapper::Mbc1, build_rom(0x03, 0x00, 0x05), vec![]);
        assert_eq!(
            dump(&mut device).err(),
            Some(Error::CustomError(String::from(
                "Unsupported RAM size for MBC1: 8 banks"
            )))
        );
    }

    #[test]
    fn test_finish_incomplete() {
        let mut dumper = Dumper::new();
        assert_eq!(
            dumper.finish().err(),
            Some(Error::CustomError(String::from(
                "The dump of the cartridge is not complete"
            )))
        );
    }

    #[test]
    fn test_finish_failed_reply() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        let exchange = advance(&mut dumper, &mut device, CHUNK_SIZE);
        device.write_all(exchange.send()).unwrap();
        let mut data = vec![0x00; CHUNK_SIZE];
        device.read_exact(&mut data).unwrap();
        assert_eq!(
            dumper.feed(&data[1..]).err(),
            Some(Error::DataError(String::from(
                "Expected 64 bytes from the device, got 63"
            )))
        );

        // a caller that carries on after the failed reply must not get
        // a dump that misses the bytes of that reply
        while let Some(exchange) = dumper.next_exchange() {
            complete(&mut dumper, &mut device, exchange);
        }
        assert!(!dumper.is_finished());
        assert_eq!(
            dumper.finish().err(),
            Some(Error::CustomError(String::from(
                "The dump of the cartridge is not complete"
            )))
        );
    }

    #[test]
    fn test_finish_spent() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        let mut dumper = Dumper::new();
        dumper.run(&mut device, |_| {}).unwrap();
        assert!(dumper.finish().is_err());
    }

    #[test]
    fn test_run_no_mbc() {
        let rom = build_rom(0x00, 0x00, 0x00);
        let mut device = FakeDevice::new(Mapper::NoMbc, rom.clone(), vec![]);
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data(), &rom);
        assert!(device.writes.is_empty());
    }

    #[test]
    fn test_run_no_mbc_ram() {
        let rom = build_rom(0x09, 0x00, 0x02);
        let ram = build_ram(RAM_BANK_SIZE);
        let mut device = FakeDevice::new(Mapper::NoMbc, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data(), &rom);
        assert_eq!(dump.ram_data(), Some(&ram));
    }

    #[test]
    fn test_run_mbc1() {
        let rom = build_rom(0x01, 0x04, 0x00);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom.clone(), vec![]);
        assert_eq!(dump(&mut device).unwrap().rom_data(), &rom);
    }

    #[test]
    fn test_run_mbc1_upper_banks() {
        let rom = build_rom(0x01, 0x06, 0x00);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom.clone(), vec![]);
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data().len(), 128 * ROM_BANK_SIZE);
        for bank in [0x20, 0x40, 0x60] {
            assert_eq!(
                &dump.rom_data()[bank * ROM_BANK_SIZE..(bank + 1) * ROM_BANK_SIZE],
                &rom[bank * ROM_BANK_SIZE..(bank + 1) * ROM_BANK_SIZE]
            );
        }
        assert_eq!(dump.rom_data(), &rom);
    }

    #[test]
    fn test_run_mbc1_ram() {
        let rom = build_rom(0x03, 0x04, 0x03);
        let ram = build_ram(RAM_BANK_SIZE * 4);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data(), &rom);
        assert_eq!(dump.ram_data(), Some(&ram));
        assert!(!device.ram_enabled);
        assert_eq!(device.mode, 0);
    }

    #[test]
    fn test_run_mbc2() {
        let rom = build_rom(0x06, 0x03, 0x00);
        let ram = build_ram(512);
        let mut device = FakeDevice::new(Mapper::Mbc2, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data(), &rom);
        let nibbles: Vec<u8> = ram.iter().map(|byte| (byte & 0x0f) | 0xf0).collect();
        assert_eq!(dump.ram_data(), Some(&nibbles));
    }

    #[test]
    fn test_run_mbc3() {
        let rom = build_rom(0x13, 0x06, 0x03);
        let ram = build_ram(RAM_BANK_SIZE * 4);
        let mut device = FakeDevice::new(Mapper::Mbc3, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data(), &rom);
        assert_eq!(dump.ram_data(), Some(&ram));
    }

    #[test]
    fn test_run_mbc30() {
        let rom = build_rom(0x10, 0x07, 0x05);
        let ram = build_ram(RAM_BANK_SIZE * 8);
        let mut device = FakeDevice::new(Mapper::Mbc3, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data().len(), 256 * ROM_BANK_SIZE);
        assert_eq!(dump.rom_data(), &rom);
        assert_eq!(dump.ram_data(), Some(&ram));
    }

    #[test]
    fn test_run_mbc5() {
        let rom = build_rom(0x1b, 0x08, 0x04);
        let ram = build_ram(RAM_BANK_SIZE * 16);
        let mut device = FakeDevice::new(Mapper::Mbc5, rom.clone(), ram.clone());
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.rom_data().len(), 512 * ROM_BANK_SIZE);
        assert_eq!(dump.rom_data(), &rom);
        assert_eq!(dump.ram_data(), Some(&ram));
    }

    #[test]
    fn test_run_ram_without_battery() {
        let rom = build_rom(0x02, 0x01, 0x02);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom, build_ram(RAM_BANK_SIZE));
        let dump = dump(&mut device).unwrap();
        assert_eq!(dump.ram_data(), None);
        assert_ne!(device.access_mode, ACCESS_MODE_RAM_READ);
        assert!(!device.writes.contains(&(0x0000, 0x0a)));
    }

    #[test]
    fn test_run_read_only() {
        let rom = build_rom(0x1b, 0x05, 0x03);
        let mut device = FakeDevice::new(Mapper::Mbc5, rom, build_ram(RAM_BANK_SIZE * 4));
        dump(&mut device).unwrap();
        assert!(device.writes.iter().all(|(address, _)| *address < 0x8000));
        assert!(!device.commands.contains(&0xb3));
    }

    #[test]
    fn test_run_cs_pulse() {
        let rom = build_rom(0x03, 0x01, 0x02);
        let ram = build_ram(RAM_BANK_SIZE);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom, ram);
        dump(&mut device).unwrap();
        assert!(!device.cs_pulse);
    }

    #[test]
    fn test_run_disconnected() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        device.broken_reads = 10;
        let result = dump(&mut device);
        assert!(matches!(result, Err(Error::IoError(_))));
    }

    #[test]
    fn test_run_no_device() {
        let mut device = FakeDevice::new(Mapper::NoMbc, build_rom(0x00, 0x00, 0x00), vec![]);
        device.firmware = vec![];
        assert!(matches!(dump(&mut device), Err(Error::IoError(_))));
    }

    #[test]
    fn test_run_reads() {
        let rom = build_rom(0x03, 0x01, 0x02);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom, build_ram(RAM_BANK_SIZE));
        dump(&mut device).unwrap();
        let reads = device
            .commands
            .iter()
            .filter(|&&command| command == DMG_CART_READ)
            .count();
        assert_eq!(
            reads,
            2 + 4 * ROM_BANK_SIZE / CHUNK_SIZE + RAM_BANK_SIZE / CHUNK_SIZE
        );
    }

    #[test]
    fn test_run_variables() {
        let rom = build_rom(0x03, 0x01, 0x02);
        let mut device = FakeDevice::new(Mapper::Mbc1, rom, build_ram(RAM_BANK_SIZE));
        dump(&mut device).unwrap();
        assert_eq!(device.transfer_size, CHUNK_SIZE);
        assert_eq!(device.access_mode, ACCESS_MODE_RAM_READ);
        assert_eq!(device.address, 0xa000 + RAM_BANK_SIZE as u32);
    }
}
