//! Sans-IO flasher of ROMs into flash cartridges through a GBxCart RW.
//!
//! As the [`crate::dumper::Dumper`], the [`Flasher`] never touches the
//! device, it hands out the next [`Exchange`] and consumes the reply, so
//! that any frontend runs the same flashing logic over its transport.
//!
//! Flashing erases and programs the flash chip of the cartridge, which
//! can't be undone, so only cartridges whose flash chip describes itself
//! through CFI with the AMD command set are flashed, and every byte is
//! read back and compared once programmed.

use std::{
    collections::VecDeque,
    io::{Read, Write},
};

use boytacean_common::error::Error;

use crate::{
    dumper::{Exchange, CHUNK_SIZE},
    flash::{
        bank_writes, cfi_writes, program_commands, reset_writes, sector_erase_writes, Chip,
        CFI_SIZE, ERASED,
    },
    gbxcart::{
        cart_write, flash_program, flash_write, set_flash_commands, set_variable, Firmware,
        ACCESS_MODE_ROM_READ, ACK, ADDRESS, CART_MODE, DMG_ACCESS_MODE, DMG_CART_READ,
        DMG_MBC_RESET, DMG_READ_CS_PULSE, DMG_READ_METHOD, DMG_ROM_BANK, DMG_WRITE_CS_PULSE,
        FLASH_BUFFER_SIZE, FLASH_PROGRAM_MODE, QUERY_FW_INFO, SET_ADDR_AS_INPUTS, SET_MODE_DMG,
        SET_VOLTAGE_3_3V, SET_VOLTAGE_5V, TRANSFER_SIZE,
    },
    header::ROM_BANK_SIZE,
    mapper::{BusWrite, ROM_BANK_ADDRESS},
};

/// The number of bytes programmed by each [`flash_program`].
pub const PROGRAM_SIZE: usize = 256;

/// Where the reply to a command goes once it is received.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    None,
    Firmware,
    Ack,
    Cfi,
    Erase(u16),
    Program,
    Verify,
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

pub struct Flasher {
    rom: Option<Vec<u8>>,
    commands: VecDeque<Command>,
    pending: Option<(Target, usize)>,
    firmware: Option<Firmware>,
    cfi: Vec<u8>,
    chip: Option<Chip>,
    detected: bool,
    erase_polls: u32,
    programmed: usize,
    verified: usize,
}

impl Flasher {
    /// Builds the flasher of the provided ROM, which must be made of
    /// whole ROM banks.
    pub fn new(rom: Vec<u8>) -> Result<Self, Error> {
        if rom.is_empty() || rom.len() % ROM_BANK_SIZE != 0 {
            return Err(Error::RomSize);
        }
        Ok(Self::build(Some(rom)))
    }

    /// Builds a flasher that only probes the cartridge for its flash
    /// chip, never erasing nor programming it.
    pub fn probe() -> Self {
        Self::build(None)
    }

    fn build(rom: Option<Vec<u8>>) -> Self {
        let mut flasher = Self {
            rom,
            commands: VecDeque::new(),
            pending: None,
            firmware: None,
            cfi: vec![],
            chip: None,
            detected: false,
            erase_polls: 0,
            programmed: 0,
            verified: 0,
        };
        flasher.commands.push_back(Command::with_reply(
            vec![QUERY_FW_INFO],
            Firmware::REPLY_SIZE,
            Target::Firmware,
        ));
        flasher
    }

    /// The next exchange with the device, made of the commands up to
    /// the next one with a reply, `None` while the reply to the last
    /// exchange is pending or once the flashing is over.
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
        Some(Exchange::new(send, expect.1))
    }

    /// The progress of the flashing, from `0.0` to `1.0`, programming
    /// and verifying making half of it each.
    pub fn progress(&self) -> f32 {
        let size = self.rom_size();
        if size == 0 {
            return 0.0;
        }
        (self.programmed + self.verified) as f32 / (size * 2) as f32
    }

    /// If the flashing is complete, with the flash chip detected and the
    /// whole ROM (if any) programmed and verified.
    pub fn is_finished(&self) -> bool {
        self.detected
            && self.commands.is_empty()
            && self.pending.is_none()
            && self.programmed == self.rom_size()
            && self.verified == self.rom_size()
    }

    /// The description of the device, once its firmware is known.
    pub fn device(&self) -> Option<String> {
        self.firmware.as_ref().map(|firmware| format!("{firmware}"))
    }

    /// The flash chip of the cartridge, once its CFI table is read and
    /// as long as it can be flashed.
    pub fn chip(&self) -> Option<&Chip> {
        self.chip.as_ref()
    }

    /// Feeds the reply to the last exchange, which may plan the next
    /// commands (eg: the flashing of the ROM once the chip is known).
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
            Target::Cfi => self.feed_cfi(data)?,
            Target::Erase(address) => self.feed_erase(address, data)?,
            Target::Program => {
                if data != [ACK] {
                    return Err(Error::DataError(format!(
                        "Failed to program the flash chip at 0x{:06x}",
                        self.programmed
                    )));
                }
                self.programmed += PROGRAM_SIZE;
            }
            Target::Verify => self.feed_verify(data)?,
        }
        Ok(())
    }

    /// Finishes the flashing, failing in case it is not complete.
    pub fn finish(&mut self) -> Result<(), Error> {
        if !self.is_finished() {
            return Err(Error::CustomError(String::from(
                "The flashing of the cartridge is not complete",
            )));
        }
        Ok(())
    }

    /// Runs the complete flashing over a blocking transport to the
    /// device (eg: a serial port), calling back with the progress after
    /// each exchange.
    pub fn run<T: Read + Write, F: FnMut(f32)>(
        &mut self,
        transport: &mut T,
        mut callback: F,
    ) -> Result<(), Error> {
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

    fn rom_size(&self) -> usize {
        self.rom.as_ref().map_or(0, Vec::len)
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

        // powers the flash chip with 3.3V in Game Boy mode, resets the
        // MBC and reads the CFI table of the chip (if any)
        self.push(vec![SET_VOLTAGE_3_3V, SET_MODE_DMG]);
        self.push(set_variable(DMG_READ_METHOD, 0x01));
        self.push(set_variable(CART_MODE, 0x01));
        self.push(set_variable(DMG_READ_CS_PULSE, 0x00));
        self.push(set_variable(DMG_WRITE_CS_PULSE, 0x00));
        self.commands
            .push_back(Command::with_reply(vec![DMG_MBC_RESET], 1, Target::Ack));
        self.push(set_variable(FLASH_PROGRAM_MODE, 0x00));

        // reads the start of the cartridge before writing to it, as the
        // writes don't reach the cartridge until the first read once its
        // address lines were released (eg: at the end of a previous run)
        self.push_read(0x0000, CHUNK_SIZE, ACCESS_MODE_ROM_READ);
        self.push_reads(1, CHUNK_SIZE, Target::None);
        self.push_flash_writes(&cfi_writes());
        self.push_read(0x0000, CHUNK_SIZE, ACCESS_MODE_ROM_READ);
        self.push_reads(CFI_SIZE / CHUNK_SIZE, CHUNK_SIZE, Target::Cfi);
        self.push_flash_writes(&reset_writes());
        Ok(())
    }

    fn feed_cfi(&mut self, data: &[u8]) -> Result<(), Error> {
        self.cfi.extend_from_slice(data);
        if self.cfi.len() < CFI_SIZE {
            return Ok(());
        }
        self.detected = true;

        // a probe tells that there's no flash chip that can be flashed
        // (no chip at all) instead of failing, as nothing is written
        let chip = match (Chip::from_cfi(&self.cfi), &self.rom) {
            (Ok(chip), _) => chip,
            (Err(_), None) => {
                self.push(vec![SET_VOLTAGE_5V, SET_ADDR_AS_INPUTS]);
                return Ok(());
            }
            (Err(err), Some(_)) => return Err(err),
        };
        if self.rom_size() > chip.size() {
            return Err(Error::CustomError(format!(
                "The ROM ({} KiB) does not fit in the flash chip ({} KiB)",
                self.rom_size() / 1024,
                chip.size() / 1024
            )));
        }
        if let Some(rom) = self.rom.take() {
            self.push_flashing(&rom, &chip);
            self.rom = Some(rom);
        }
        self.chip = Some(chip);
        self.push(vec![SET_VOLTAGE_5V, SET_ADDR_AS_INPUTS]);
        Ok(())
    }

    fn feed_erase(&mut self, address: u16, data: &[u8]) -> Result<(), Error> {
        if data == ERASED {
            self.erase_polls = 0;
            return Ok(());
        }

        // the chip toggles the value read while it erases the sector, so
        // the read is repeated until the sector is erased or times out,
        // each read taking at least a millisecond
        self.erase_polls += 1;
        let timeout = self.chip.as_ref().map_or(0, Chip::erase_timeout);
        if self.erase_polls > timeout {
            return Err(Error::CustomError(format!(
                "Timeout erasing the flash sector at 0x{address:04x}"
            )));
        }
        for command in Self::read_commands(address, 2, Target::Erase(address))
            .into_iter()
            .rev()
        {
            self.commands.push_front(command);
        }
        Ok(())
    }

    fn feed_verify(&mut self, data: &[u8]) -> Result<(), Error> {
        let rom = self.rom.as_ref().unwrap();
        let expected = &rom[self.verified..self.verified + data.len()];
        if let Some(index) = (0..data.len()).find(|&index| data[index] != expected[index]) {
            return Err(Error::DataError(format!(
                "Verification failed at 0x{:06x}, the cartridge was not flashed correctly",
                self.verified + index
            )));
        }
        self.verified += data.len();
        Ok(())
    }

    /// Plans the flashing of the ROM, loading the buffered programming
    /// commands of the chip, then erasing and programming every bank of
    /// the ROM in order (a sector larger than a bank is only erased from
    /// the first bank it covers) and reading every bank back.
    fn push_flashing(&mut self, rom: &[u8], chip: &Chip) {
        self.push(set_flash_commands(&program_commands()));
        let banks = rom.len() / ROM_BANK_SIZE;
        for bank in 0..banks {
            let start = bank * ROM_BANK_SIZE;
            self.push_bank(bank);
            for sector in chip.sectors_in(start, start + ROM_BANK_SIZE) {
                let address = ROM_BANK_ADDRESS + (sector - start) as u16;
                self.push_flash_writes(&reset_writes());
                self.push_flash_writes(&sector_erase_writes(address));
                self.commands
                    .extend(Self::read_commands(address, 2, Target::Erase(address)));
                self.push_flash_writes(&reset_writes());
            }
            self.push(set_variable(TRANSFER_SIZE, PROGRAM_SIZE as u32));
            self.push(set_variable(FLASH_BUFFER_SIZE, chip.buffer_size() as u32));
            self.push(set_variable(ADDRESS, ROM_BANK_ADDRESS as u32));
            for data in rom[start..start + ROM_BANK_SIZE].chunks(PROGRAM_SIZE) {
                self.commands.push_back(Command::with_reply(
                    flash_program(data),
                    1,
                    Target::Program,
                ));
            }
        }
        for bank in 0..banks {
            self.push_bank(bank);
            self.push_read(ROM_BANK_ADDRESS, CHUNK_SIZE, ACCESS_MODE_ROM_READ);
            self.push_reads(ROM_BANK_SIZE / CHUNK_SIZE, CHUNK_SIZE, Target::Verify);
        }
    }

    fn push(&mut self, data: Vec<u8>) {
        self.commands.push_back(Command::new(data));
    }

    fn push_writes(&mut self, writes: &[BusWrite]) {
        for (address, value) in writes {
            self.push(cart_write(*address, *value));
        }
    }

    fn push_flash_writes(&mut self, writes: &[BusWrite]) {
        for (address, value) in writes {
            self.push(flash_write(*address, *value));
        }
    }

    fn push_bank(&mut self, bank: usize) {
        self.push_writes(&bank_writes(bank));
        self.push(set_variable(DMG_ROM_BANK, bank as u32));
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

    /// The commands of a read of the provided size from the address,
    /// with its reply going to the target.
    fn read_commands(address: u16, size: usize, target: Target) -> Vec<Command> {
        vec![
            Command::new(set_variable(TRANSFER_SIZE, size as u32)),
            Command::new(set_variable(ADDRESS, address as u32)),
            Command::new(set_variable(DMG_ACCESS_MODE, ACCESS_MODE_ROM_READ)),
            Command::with_reply(vec![DMG_CART_READ], size, target),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        io::{self, Read, Write},
    };

    use boytacean_common::error::Error;

    use super::{Flasher, PROGRAM_SIZE};
    use crate::{
        dumper::Exchange,
        flash::CFI_SIZE,
        gbxcart::{
            DMG_CART_READ, DMG_CART_WRITE, DMG_FLASH_PROGRAM, DMG_FLASH_WRITE, DMG_MBC_RESET,
            QUERY_FW_INFO, SET_ADDR_AS_INPUTS, SET_FLASH_COMMANDS, SET_MODE_DMG, SET_VARIABLE,
            SET_VOLTAGE_3_3V, SET_VOLTAGE_5V,
        },
        header::ROM_BANK_SIZE,
    };

    /// The reply to the firmware query of a GBxCart RW v1.3 running
    /// the L1 firmware, as captured from the device.
    const FIRMWARE_L1: [u8; 9] = [0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea];

    /// The CFI values from offset `0x10` of the 4 MiB flash chip of a
    /// flash cartridge (CFI ID `01 7E`), as captured from the device.
    const CFI_4MB: [u8; 37] = [
        0x51, 0x52, 0x59, 0x02, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, 0x36, 0x00, 0x00,
        0x03, 0x04, 0x08, 0x0e, 0x03, 0x05, 0x03, 0x03, 0x16, 0x02, 0x00, 0x05, 0x00, 0x02, 0x07,
        0x00, 0x20, 0x00, 0x3e, 0x00, 0x00, 0x01,
    ];

    /// The commands of the buffered programming, as captured from the
    /// device when flashing a cartridge.
    const FLASH_COMMANDS: [u8; 39] = [
        0x01, 0x02, 0x01, 0x00, 0x00, 0x0a, 0xaa, 0x00, 0xaa, 0x00, 0x00, 0x05, 0x55, 0x00, 0x55,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29,
    ];

    /// A fake GBxCart RW running the L firmware with a flash cartridge
    /// (an MBC5 in front of a flash chip with the AMD command set), the
    /// chip being simulated independently of [`Flasher`], following the
    /// rule of flash chips that programming only clears bits, so that a
    /// missing erase gives a wrong flash.
    struct FakeDevice {
        firmware: Vec<u8>,
        flash: Vec<u8>,
        flash_chip: bool,
        sectors: Vec<usize>,
        bank: usize,
        cfi_mode: bool,
        sequence: Vec<(u16, u8)>,
        erasing: usize,
        erase_polls: usize,
        transfer_size: usize,
        buffer_size: usize,
        address: u32,
        program_mode: u32,
        flash_commands: Vec<u8>,
        voltages: Vec<u8>,
        broken_byte: Option<usize>,
        failed_program: bool,
        driven: bool,
        input: Vec<u8>,
        output: VecDeque<u8>,
        commands: Vec<u8>,
        erases: Vec<usize>,
        banks: Vec<u32>,
    }

    impl FakeDevice {
        fn new(flash: Vec<u8>) -> Self {
            let mut sectors = vec![0x2000; 8];
            sectors.extend(vec![0x10000; 63]);
            Self {
                firmware: FIRMWARE_L1.to_vec(),
                flash,
                flash_chip: true,
                sectors,
                bank: 1,
                cfi_mode: false,
                sequence: vec![],
                erasing: 0,
                erase_polls: 3,
                transfer_size: 1,
                buffer_size: 0,
                address: 0,
                program_mode: 0xff,
                flash_commands: vec![],
                voltages: vec![],
                broken_byte: None,
                failed_program: false,
                driven: false,
                input: vec![],
                output: VecDeque::new(),
                commands: vec![],
                erases: vec![],
                banks: vec![],
            }
        }

        fn offset(&self, address: u32) -> usize {
            if address < 0x4000 {
                address as usize
            } else {
                self.bank * ROM_BANK_SIZE + (address as usize - 0x4000)
            }
        }

        fn read(&mut self, address: u32) -> u8 {
            if self.cfi_mode {
                let cfi = build_cfi(&CFI_4MB);
                return cfi[address as usize % CFI_SIZE];
            }
            if self.erasing > 0 {
                return if address % 2 == 0 { 0x4d } else { 0x09 };
            }
            self.flash[self.offset(address) % self.flash.len()]
        }

        fn write_flash(&mut self, address: u16, value: u8) {
            if !self.flash_chip {
                return;
            }
            match (address, value) {
                (_, 0xf0) => {
                    self.cfi_mode = false;
                    self.sequence.clear();
                }
                (0x00aa, 0x98) => self.cfi_mode = true,
                _ => {
                    self.sequence.push((address, value));
                    let erase = [
                        (0x0aaa, 0xaa),
                        (0x0555, 0x55),
                        (0x0aaa, 0x80),
                        (0x0aaa, 0xaa),
                        (0x0555, 0x55),
                    ];
                    if self.sequence.len() == 6 && self.sequence[..5] == erase && value == 0x30 {
                        let offset = self.offset(address as u32);
                        let mut start = 0;
                        for sector_size in self.sectors.clone() {
                            if offset < start + sector_size {
                                self.flash[start..start + sector_size].fill(0xff);
                                self.erases.push(start);
                                break;
                            }
                            start += sector_size;
                        }
                        self.erasing = self.erase_polls;
                        self.sequence.clear();
                    }
                    if self.sequence.len() >= 6 {
                        self.sequence.remove(0);
                    }
                }
            }
        }

        fn program(&mut self, data: &[u8]) {
            let valid = self.flash_chip
                && self.program_mode == 0
                && self.buffer_size == 32
                && self.flash_commands == FLASH_COMMANDS
                && !self.failed_program;
            if !valid {
                self.output.push_back(0x00);
                return;
            }
            for (index, value) in data.iter().enumerate() {
                let offset = self.offset(self.address + index as u32);
                if self.broken_byte != Some(offset) {
                    self.flash[offset] &= value;
                }
            }
            self.address += data.len() as u32;
            self.output.push_back(0x01);
        }

        fn process(&mut self) {
            loop {
                let size = match self.input.first() {
                    None => return,
                    Some(&SET_VARIABLE) => 10,
                    Some(&SET_FLASH_COMMANDS) => 40,
                    Some(&DMG_CART_WRITE) | Some(&DMG_FLASH_WRITE) => 6,
                    Some(&DMG_FLASH_PROGRAM) => 1 + self.transfer_size,
                    Some(_) => 1,
                };
                if self.input.len() < size {
                    return;
                }
                let command: Vec<u8> = self.input.drain(..size).collect();
                self.commands.push(command[0]);
                match command[0] {
                    QUERY_FW_INFO => self.output.extend(self.firmware.clone()),
                    SET_VOLTAGE_3_3V | SET_VOLTAGE_5V => self.voltages.push(command[0]),
                    SET_MODE_DMG => {}
                    SET_ADDR_AS_INPUTS => self.driven = false,
                    SET_VARIABLE => {
                        let key =
                            u32::from_be_bytes([command[2], command[3], command[4], command[5]]);
                        let value =
                            u32::from_be_bytes([command[6], command[7], command[8], command[9]]);
                        match (command[1], key) {
                            (2, 0x00) => self.transfer_size = value as usize,
                            (2, 0x01) => self.buffer_size = value as usize,
                            (2, 0x02) => self.banks.push(value),
                            (4, 0x00) => self.address = value,
                            (1, 0x05) => self.program_mode = value,
                            _ => {}
                        }
                    }
                    SET_FLASH_COMMANDS => self.flash_commands = command[1..].to_vec(),
                    DMG_CART_WRITE | DMG_FLASH_WRITE if !self.driven => {}
                    DMG_CART_WRITE => {
                        let address =
                            u32::from_be_bytes([command[1], command[2], command[3], command[4]]);
                        match address {
                            0x2000..=0x2fff => {
                                self.bank = (self.bank & 0x100) | command[5] as usize
                            }
                            0x3000..=0x3fff => {
                                self.bank =
                                    (self.bank & 0x0ff) | ((command[5] as usize & 0x01) << 8)
                            }
                            _ => {}
                        }
                    }
                    DMG_FLASH_WRITE => {
                        let address =
                            u32::from_be_bytes([command[1], command[2], command[3], command[4]]);
                        self.write_flash(address as u16, command[5]);
                    }
                    DMG_MBC_RESET => {
                        self.bank = 1;
                        self.output.push_back(0x01);
                    }
                    DMG_CART_READ => {
                        self.driven = true;
                        for index in 0..self.transfer_size as u32 {
                            let byte = self.read(self.address + index);
                            self.output.push_back(byte);
                        }
                        self.erasing = self.erasing.saturating_sub(1);
                        self.address += self.transfer_size as u32;
                    }
                    DMG_FLASH_PROGRAM => self.program(&command[1..]),
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

    /// Builds the CFI table as read from the device, with every value
    /// showing up twice, from the provided values from offset `0x10`.
    fn build_cfi(values: &[u8]) -> Vec<u8> {
        let mut data = vec![0x00; CFI_SIZE];
        for (offset, value) in values.iter().enumerate() {
            data[(0x10 + offset) * 2] = *value;
            data[(0x10 + offset) * 2 + 1] = *value;
        }
        data
    }

    /// Builds a ROM of the provided number of banks, each byte holding
    /// its bank number mixed with its offset so that programming or
    /// reading a wrong bank is caught.
    fn build_rom(banks: usize) -> Vec<u8> {
        (0..banks * ROM_BANK_SIZE)
            .map(|index| ((index / ROM_BANK_SIZE) as u8) ^ ((index >> 4) as u8) ^ index as u8)
            .collect()
    }

    /// Builds the contents of the 4 MiB flash chip, holding a previous
    /// game, so that programming it without erasing it is caught.
    fn build_flash() -> Vec<u8> {
        vec![0x5a; 4 * 1024 * 1024]
    }

    fn flash(device: &mut FakeDevice, rom: Vec<u8>) -> Result<(), Error> {
        Flasher::new(rom).unwrap().run(device, |_| {})
    }

    #[test]
    fn test_new() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        assert_eq!(flasher.next_exchange(), Some(Exchange::new(vec![0xa1], 9)));
        assert_eq!(flasher.progress(), 0.0);
        assert!(!flasher.is_finished());
        assert_eq!(flasher.device(), None);
        assert_eq!(flasher.chip(), None);
    }

    #[test]
    fn test_new_invalid_size() {
        for rom in [vec![], vec![0x00; ROM_BANK_SIZE + 1], vec![0x00; 0x100]] {
            assert_eq!(Flasher::new(rom).err(), Some(Error::RomSize));
        }
    }

    #[test]
    fn test_probe() {
        let mut device = FakeDevice::new(build_flash());
        let mut flasher = Flasher::probe();
        flasher.run(&mut device, |_| {}).unwrap();
        assert_eq!(
            flasher.chip().map(|chip| chip.size()),
            Some(4 * 1024 * 1024)
        );
        assert_eq!(
            flasher.device(),
            Some(String::from("GBxCart RW v1.3 (firmware L1)"))
        );
        assert_eq!(flasher.progress(), 0.0);
        assert!(flasher.is_finished());

        // a probe never erases nor programs, and ends at 5V
        assert_eq!(device.flash, build_flash());
        assert!(device.erases.is_empty());
        assert!(!device.commands.contains(&DMG_FLASH_PROGRAM));
        assert!(!device.commands.contains(&SET_FLASH_COMMANDS));
        assert_eq!(device.voltages, vec![SET_VOLTAGE_3_3V, SET_VOLTAGE_5V]);
        assert_eq!(device.commands.last(), Some(&SET_ADDR_AS_INPUTS));
    }

    #[test]
    fn test_probe_no_chip() {
        let mut device = FakeDevice::new(build_rom(2));
        device.flash_chip = false;
        let mut flasher = Flasher::probe();
        flasher.run(&mut device, |_| {}).unwrap();
        assert_eq!(flasher.chip(), None);
        assert!(flasher.is_finished());
        assert_eq!(device.flash, build_rom(2));
        assert_eq!(device.voltages, vec![SET_VOLTAGE_3_3V, SET_VOLTAGE_5V]);
    }

    #[test]
    fn test_next_exchange_pending() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        assert!(flasher.next_exchange().is_some());
        assert_eq!(flasher.next_exchange(), None);
        flasher.feed(&FIRMWARE_L1).unwrap();
        assert!(flasher.next_exchange().is_some());
    }

    #[test]
    fn test_progress() {
        let mut device = FakeDevice::new(build_flash());
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        let mut progress = vec![];
        flasher
            .run(&mut device, |value| progress.push(value))
            .unwrap();
        assert!(progress.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(progress.contains(&0.5));
        assert_eq!(progress.last(), Some(&1.0));
        assert_eq!(flasher.progress(), 1.0);
    }

    #[test]
    fn test_is_finished() {
        let mut device = FakeDevice::new(build_flash());
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        while let Some(exchange) = flasher.next_exchange() {
            assert!(!flasher.is_finished());
            device.write_all(exchange.send()).unwrap();
            let mut data = vec![0x00; exchange.expect()];
            device.read_exact(&mut data).unwrap();
            flasher.feed(&data).unwrap();
        }
        assert!(flasher.is_finished());
    }

    #[test]
    fn test_feed_no_exchange() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        assert_eq!(
            flasher.feed(&[0x01]).err(),
            Some(Error::CustomError(String::from(
                "No exchange with the device in progress"
            )))
        );
    }

    #[test]
    fn test_feed_short_reply() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        flasher.next_exchange().unwrap();
        assert_eq!(
            flasher.feed(&FIRMWARE_L1[..8]).err(),
            Some(Error::DataError(String::from(
                "Expected 9 bytes from the device, got 8"
            )))
        );
    }

    #[test]
    fn test_feed_unsupported_firmware() {
        let mut device = FakeDevice::new(build_flash());
        device.firmware = vec![0x08, 0x52, 0x00, 0x1f, 0x04, 0x65, 0xe2, 0x1d, 0xea];
        assert_eq!(
            flash(&mut device, build_rom(2)).err(),
            Some(Error::CustomError(String::from(
                "Unsupported firmware R31, only the L firmware is supported"
            )))
        );
        assert!(!device.commands.contains(&SET_VOLTAGE_3_3V));
    }

    #[test]
    fn test_feed_invalid_ack() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        flasher.next_exchange().unwrap();
        flasher.feed(&FIRMWARE_L1).unwrap();
        flasher.next_exchange().unwrap();
        assert_eq!(
            flasher.feed(&[0x00]).err(),
            Some(Error::DataError(String::from(
                "Invalid acknowledgement from the device"
            )))
        );
    }

    #[test]
    fn test_finish_incomplete() {
        let mut flasher = Flasher::new(build_rom(2)).unwrap();
        assert_eq!(
            flasher.finish().err(),
            Some(Error::CustomError(String::from(
                "The flashing of the cartridge is not complete"
            )))
        );
    }

    #[test]
    fn test_run() {
        let rom = build_rom(2);
        let mut device = FakeDevice::new(build_flash());
        flash(&mut device, rom.clone()).unwrap();
        assert_eq!(&device.flash[..rom.len()], &rom[..]);
        assert!(device.flash[rom.len()..].iter().all(|&byte| byte == 0x5a));

        // the four sectors of 8 KiB covering the ROM are erased, and the
        // flashing runs at 3.3V, getting back to 5V at the end
        assert_eq!(device.erases, vec![0x0000, 0x2000, 0x4000, 0x6000]);
        assert_eq!(device.voltages, vec![SET_VOLTAGE_3_3V, SET_VOLTAGE_5V]);
        assert_eq!(device.commands.last(), Some(&SET_ADDR_AS_INPUTS));
        assert_eq!(
            device
                .commands
                .iter()
                .filter(|&&command| command == DMG_FLASH_PROGRAM)
                .count(),
            rom.len() / PROGRAM_SIZE
        );
    }

    #[test]
    fn test_run_sectors() {
        // a 64 KiB sector covers four banks, and is only erased once
        let rom = build_rom(12);
        let mut device = FakeDevice::new(build_flash());
        flash(&mut device, rom.clone()).unwrap();
        assert_eq!(&device.flash[..rom.len()], &rom[..]);
        assert_eq!(
            device.erases,
            vec![0x0000, 0x2000, 0x4000, 0x6000, 0x8000, 0xa000, 0xc000, 0xe000, 0x10000, 0x20000]
        );
    }

    #[test]
    fn test_run_banks() {
        // the bank is set after selecting it, for programming and then
        // for verifying every bank
        let mut device = FakeDevice::new(build_flash());
        flash(&mut device, build_rom(3)).unwrap();
        assert_eq!(device.banks, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn test_run_full() {
        let rom = build_rom(256);
        let mut device = FakeDevice::new(build_flash());
        flash(&mut device, rom.clone()).unwrap();
        assert_eq!(device.flash, rom);
        assert_eq!(device.erases.len(), 71);
    }

    #[test]
    fn test_run_no_chip() {
        let mut device = FakeDevice::new(build_rom(2));
        device.flash_chip = false;
        assert_eq!(
            flash(&mut device, build_rom(2)).err(),
            Some(Error::CustomError(String::from(
                "No flash chip found, the cartridge can't be flashed"
            )))
        );
        assert!(device.erases.is_empty());
        assert!(!device.commands.contains(&DMG_FLASH_PROGRAM));
    }

    #[test]
    fn test_run_too_large() {
        let mut device = FakeDevice::new(build_flash());
        assert_eq!(
            flash(&mut device, build_rom(512)).err(),
            Some(Error::CustomError(String::from(
                "The ROM (8192 KiB) does not fit in the flash chip (4096 KiB)"
            )))
        );
        assert!(device.erases.is_empty());
        assert_eq!(device.flash, build_flash());
    }

    #[test]
    fn test_run_erase_timeout() {
        let mut device = FakeDevice::new(build_flash());
        device.erase_polls = usize::MAX;
        assert_eq!(
            flash(&mut device, build_rom(2)).err(),
            Some(Error::CustomError(String::from(
                "Timeout erasing the flash sector at 0x4000"
            )))
        );
        assert_eq!(device.erases, vec![0x0000]);
    }

    #[test]
    fn test_run_program_failure() {
        let mut device = FakeDevice::new(build_flash());
        device.failed_program = true;
        assert_eq!(
            flash(&mut device, build_rom(2)).err(),
            Some(Error::DataError(String::from(
                "Failed to program the flash chip at 0x000000"
            )))
        );
    }

    #[test]
    fn test_run_verify_failure() {
        let mut device = FakeDevice::new(build_flash());
        device.broken_byte = Some(0x5123);
        assert_eq!(
            flash(&mut device, build_rom(2)).err(),
            Some(Error::DataError(String::from(
                "Verification failed at 0x005123, the cartridge was not flashed correctly"
            )))
        );
    }

    #[test]
    fn test_run_disconnected() {
        let mut device = FakeDevice::new(build_flash());
        device.firmware = vec![];
        assert!(matches!(
            flash(&mut device, build_rom(2)),
            Err(Error::IoError(_))
        ));
    }
}
