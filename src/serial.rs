//! Serial transfer (Link Cable) functions and structures.

use std::{any::Any, io::Cursor};

use boytacean_common::{
    data::{read_u16, read_u8, write_u16, write_u8},
    error::Error,
};

use crate::{
    consts::{SB_ADDR, SC_ADDR},
    infoln,
    mmu::BusComponent,
    state::{StateComponent, StateFormat},
    warnln,
};

pub trait SerialDevice {
    /// Sends a byte (u8) through the serial connection, returning
    /// the byte received from the other end.
    ///
    /// This operation semantics is seen from the device perspective
    /// meaning that a byte is moved from the device to the Game Boy.
    fn send(&mut self) -> u8;

    /// Receives a byte (u8) from the serial connection,
    /// can be either another device or the host.
    ///
    /// This operation semantics is seen from the device perspective
    /// meaning that a byte is moved from the Game Boy to the device.
    fn receive(&mut self, byte: u8);

    /// Whether the device has data ready to be read.
    ///
    /// For network devices, this indicates if a byte has been
    /// received from the remote end.
    fn is_ready(&self) -> bool {
        false
    }

    /// Whether this device should force the serial controller into
    /// a specific clock mode
    ///
    /// Returns Some(true) to force master mode, Some(false) to force
    /// slave mode, or None to use the game's setting.
    fn force_clock(&self) -> Option<bool> {
        None
    }

    /// Returns a short description of the serial device.
    ///
    /// Should be a short string describing the device, useful
    /// for debugging purposes.
    fn description(&self) -> String;

    /// Returns a string describing the current state of the
    /// serial device, useful for debugging purposes.
    fn state(&self) -> String;

    /// Returns the device as `Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns the device as mutable `Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct Serial {
    data: u8,
    control: u8,
    shift_clock: bool,
    clock_speed: bool,
    transfer_enabled: bool,
    transferring: bool,
    timer: i16,
    length: u16,
    bit_count: u8,
    byte_send: u8,
    byte_receive: u8,
    int_serial: bool,
    device: Box<dyn SerialDevice>,
}

impl Serial {
    pub fn new() -> Self {
        Self {
            data: 0x0,
            control: 0x0,
            shift_clock: false,
            clock_speed: false,
            transfer_enabled: false,
            transferring: false,
            timer: 0,
            length: 512,
            bit_count: 0,
            byte_send: 0x0,
            byte_receive: 0x0,
            int_serial: false,
            device: Box::<NullDevice>::default(),
        }
    }

    pub fn reset(&mut self) {
        self.data = 0x0;
        self.control = 0x0;
        self.shift_clock = false;
        self.clock_speed = false;
        self.transferring = false;
        self.timer = 0;
        self.length = 512;
        self.bit_count = 0;
        self.byte_send = 0x0;
        self.byte_receive = 0x0;
        self.int_serial = false;
    }

    pub fn clock(&mut self, cycles: u16) {
        if !self.transfer_enabled {
            return;
        }

        // if there's data ready in the slave device, we need to read it
        // and set the interrupt flag, this will make the Game Boy aware
        // that there's a new byte to be read from the serial device.
        // TODO: this is a hack to get the serial working, we need to find
        // a better way to do this, we cannot run this on every clock cycle,
        if self.is_slave() && self.device.is_ready() {
            self.byte_receive = self.device.send();
            self.data = self.byte_receive;
            self.transfer_enabled = false;
            self.int_serial = true;
            infoln!("[SERIAL] Byte received: 0x{:02x}", self.byte_receive);
        }

        // in case the transferring flag is not set, meaning
        // that no transfer is happening (not master or not
        // transfer enabled), then we can return early
        if !self.transferring {
            return;
        }

        self.timer = self.timer.saturating_sub(cycles as i16);
        if self.timer <= 0 {
            let bit = (self.byte_receive >> (7 - self.bit_count)) & 0x01;
            self.data = (self.data << 1) | bit;

            self.tick_transfer();

            self.timer = self.length as i16;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0xFF01 — SB: Serial transfer data
            SB_ADDR => self.data,
            // 0xFF02 — SC: Serial transfer control
            SC_ADDR =>
            {
                #[allow(clippy::bool_to_int_with_if)]
                (if self.shift_clock { 0x01 } else { 0x00 }
                    | if self.clock_speed { 0x02 } else { 0x00 }
                    | if self.transfer_enabled { 0x80 } else { 0x00 })
            }
            _ => {
                warnln!("Reding from unknown Serial location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF01 — SB: Serial transfer data
            SB_ADDR => self.data = value,
            // 0xFF02 — SC: Serial transfer control
            SC_ADDR => {
                self.shift_clock = value & 0x01 == 0x01;
                self.clock_speed = value & 0x02 == 0x02;
                self.transfer_enabled = value & 0x80 == 0x80;

                infoln!(
                    "[SERIAL] Transfer enabled: {}, Clock speed: {}, Shift clock: {}",
                    self.transfer_enabled,
                    self.clock_speed,
                    self.shift_clock
                );

                // by the default a transfer is considered to be happening
                // in case the transfer enabled flag is set
                self.transferring = true;

                // in case the clock is meant to be set by the attached/other device
                // then the coordination of the transferring is going to be handled
                // by the other device, so we need to disable the transferring flag
                if !self.shift_clock {
                    self.transferring = false;
                }

                // in case a transfer of byte has been requested and
                // this is the then we need to start the transfer setup
                if self.transferring {
                    // @TODO: if the GBC mode exists there should
                    // be special check logic here
                    //self.length = if self.gb.is_cgb() && self.clock_speed { 16 } else { 512 };
                    self.length = 512;
                    self.bit_count = 0;
                    self.timer = self.length as i16;

                    // executes the send and receive operation immediately
                    // this is considered an operational optimization with
                    // no real effect on the emulation (ex: no timing issues)
                    // then stores the byte to be sent to the device so that
                    // it's sent by the end of the send cycle
                    self.byte_receive = self.device.send();
                    self.byte_send = self.data;
                }
            }
            _ => warnln!("Writing to unknown Serial location 0x{:04x}", addr),
        }
    }

    #[inline(always)]
    pub fn int_serial(&self) -> bool {
        self.int_serial
    }

    #[inline(always)]
    pub fn set_int_serial(&mut self, value: bool) {
        self.int_serial = value;
    }

    #[inline(always)]
    pub fn ack_serial(&mut self) {
        self.set_int_serial(false);
    }

    pub fn shift_clock(&self) -> bool {
        self.shift_clock
    }

    pub fn set_shift_clock(&mut self, value: bool) {
        self.shift_clock = value;
    }

    pub fn transferring(&self) -> bool {
        self.transferring
    }

    pub fn set_transferring(&mut self, value: bool) {
        self.transferring = value;
    }

    pub fn device(&self) -> &dyn SerialDevice {
        self.device.as_ref()
    }

    pub fn device_mut(&mut self) -> &mut dyn SerialDevice {
        self.device.as_mut()
    }

    pub fn set_device(&mut self, device: Box<dyn SerialDevice>) {
        self.device = device;
    }

    #[inline(always)]
    pub fn is_master(&self) -> bool {
        self.shift_clock
    }

    #[inline(always)]
    pub fn is_slave(&self) -> bool {
        !self.shift_clock
    }

    /// Ticks the transfer operation, incrementing the bit count
    /// and handling the transfer completion.
    ///
    /// This operation is only valid in the master mode (`shift_clock` is true).
    fn tick_transfer(&mut self) {
        self.bit_count += 1;
        if self.bit_count == 8 {
            // resets the transfer related values effectively
            // disabling the transfer
            self.transfer_enabled = false;
            self.transferring = false;

            self.length = 0;
            self.bit_count = 0;

            // receives the byte on the device since the
            // complete send operation has been performed
            self.device.receive(self.byte_send);

            // signals the interrupt for the serial
            // transfer completion, indicating that
            // a new byte is ready to be read
            self.int_serial = true;
        }
    }
}

impl BusComponent for Serial {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self::new()
    }
}

impl StateComponent for Serial {
    fn state(&self, _format: Option<StateFormat>) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(vec![]);
        write_u8(&mut cursor, self.data)?;
        write_u8(&mut cursor, self.control)?;
        write_u8(&mut cursor, self.shift_clock as u8)?;
        write_u8(&mut cursor, self.clock_speed as u8)?;
        write_u8(&mut cursor, self.transferring as u8)?;
        write_u16(&mut cursor, self.timer as u16)?;
        write_u16(&mut cursor, self.length)?;
        write_u8(&mut cursor, self.bit_count)?;
        write_u8(&mut cursor, self.byte_send)?;
        write_u8(&mut cursor, self.byte_receive)?;
        write_u8(&mut cursor, self.int_serial as u8)?;
        Ok(cursor.into_inner())
    }

    fn set_state(&mut self, data: &[u8], _format: Option<StateFormat>) -> Result<(), Error> {
        let mut cursor = Cursor::new(data);
        self.data = read_u8(&mut cursor)?;
        self.control = read_u8(&mut cursor)?;
        self.shift_clock = read_u8(&mut cursor)? != 0;
        self.clock_speed = read_u8(&mut cursor)? != 0;
        self.transferring = read_u8(&mut cursor)? != 0;
        self.timer = read_u16(&mut cursor)? as i16;
        self.length = read_u16(&mut cursor)?;
        self.bit_count = read_u8(&mut cursor)?;
        self.byte_send = read_u8(&mut cursor)?;
        self.byte_receive = read_u8(&mut cursor)?;
        self.int_serial = read_u8(&mut cursor)? != 0;
        Ok(())
    }
}

unsafe impl Send for Serial {}

pub struct NullDevice {}

impl NullDevice {
    pub fn new() -> Self {
        Self {}
    }
}

impl SerialDevice for NullDevice {
    fn send(&mut self) -> u8 {
        0xff
    }

    fn receive(&mut self, _: u8) {}

    fn description(&self) -> String {
        String::from("Null")
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

impl Default for NullDevice {
    fn default() -> Self {
        Self::new()
    }
}
