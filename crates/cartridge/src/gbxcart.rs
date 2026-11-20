//! [GBxCart RW](https://www.gbxcart.com/) device protocol, as spoken by
//! the L firmware of the device.
//!
//! The device is a USB serial converter (CH340) that runs at 1 Mbaud,
//! the host sends one byte commands, some of them followed by their
//! arguments, and only a few of them get a reply. Multi-byte values
//! are sent in big endian.
//!
//! This module only encodes the commands and decodes the replies, the
//! order in which they are sent is up to the [`crate::dumper`].

use std::fmt::{self, Display, Formatter};

use boytacean_common::error::Error;

/// The baud rate of the serial connection to the device.
pub const BAUD_RATE: u32 = 1_000_000;

/// The USB vendor ID of the serial converter of the device.
pub const USB_VENDOR_ID: u16 = 0x1a86;

/// The USB product ID of the serial converter of the device.
pub const USB_PRODUCT_ID: u16 = 0x7523;

/// Replies with the [`Firmware`] information of the device.
pub const QUERY_FW_INFO: u8 = 0xa1;

/// Sets the cartridge slot in Game Boy (DMG) mode.
pub const SET_MODE_DMG: u8 = 0xa3;

/// Powers the cartridge with 5V, the voltage of Game Boy cartridges.
pub const SET_VOLTAGE_5V: u8 = 0xa5;

/// Sets one of the [`Variable`]s of the firmware.
pub const SET_VARIABLE: u8 = 0xa6;

/// Releases the address lines of the cartridge, done at the end.
pub const SET_ADDR_AS_INPUTS: u8 = 0xa8;

/// Replies with as many bytes as the [`TRANSFER_SIZE`] from the
/// [`ADDRESS`], which is then moved past the bytes read.
pub const DMG_CART_READ: u8 = 0xb1;

/// Writes a value at an address of the cartridge bus.
pub const DMG_CART_WRITE: u8 = 0xb2;

/// Resets the MBC of the cartridge, replies with [`ACK`].
pub const DMG_MBC_RESET: u8 = 0xb4;

/// The reply of the commands that acknowledge their execution.
pub const ACK: u8 = 0x01;

/// The [`DMG_ACCESS_MODE`] value for reading the ROM.
pub const ACCESS_MODE_ROM_READ: u32 = 0x01;

/// The [`DMG_ACCESS_MODE`] value for reading the RAM.
pub const ACCESS_MODE_RAM_READ: u32 = 0x03;

/// A variable of the firmware, made of its size in bytes and of its
/// key, keys are only unique among the variables of the same size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Variable {
    size: u8,
    key: u32,
}

/// The slot mode, `0x01` for Game Boy cartridges.
pub const CART_MODE: Variable = Variable { size: 1, key: 0x00 };

/// The kind of access done by [`DMG_CART_READ`], either
/// [`ACCESS_MODE_ROM_READ`] or [`ACCESS_MODE_RAM_READ`].
pub const DMG_ACCESS_MODE: Variable = Variable { size: 1, key: 0x01 };

/// If the chip select line is pulsed in reads, needed for the RAM.
pub const DMG_READ_CS_PULSE: Variable = Variable { size: 1, key: 0x08 };

/// If the chip select line is pulsed in writes.
pub const DMG_WRITE_CS_PULSE: Variable = Variable { size: 1, key: 0x09 };

/// The method used for reads, `0x01` being the default one.
pub const DMG_READ_METHOD: Variable = Variable { size: 1, key: 0x0b };

/// The number of bytes replied by each [`DMG_CART_READ`].
pub const TRANSFER_SIZE: Variable = Variable { size: 2, key: 0x00 };

/// The address of the next [`DMG_CART_READ`].
pub const ADDRESS: Variable = Variable { size: 4, key: 0x00 };

/// Encodes the command that sets a variable of the firmware.
pub fn set_variable(variable: Variable, value: u32) -> Vec<u8> {
    let mut data = vec![SET_VARIABLE, variable.size];
    data.extend_from_slice(&variable.key.to_be_bytes());
    data.extend_from_slice(&value.to_be_bytes());
    data
}

/// Encodes the command that writes a value in the cartridge bus.
pub fn cart_write(address: u16, value: u8) -> Vec<u8> {
    let mut data = vec![DMG_CART_WRITE];
    data.extend_from_slice(&(address as u32).to_be_bytes());
    data.push(value);
    data
}

pub struct Firmware {
    kind: u8,
    version: u16,
    pcb: u8,
    timestamp: u32,
}

impl Firmware {
    /// The size in bytes of the reply to [`QUERY_FW_INFO`].
    pub const REPLY_SIZE: usize = 9;

    /// Decodes the reply to [`QUERY_FW_INFO`], made of its length,
    /// the firmware kind, version, PCB version and build timestamp.
    pub fn from_data(data: &[u8]) -> Result<Self, Error> {
        if data.len() != Self::REPLY_SIZE || data[0] as usize != Self::REPLY_SIZE - 1 {
            return Err(Error::DataError(String::from(
                "Invalid firmware information from the device",
            )));
        }
        Ok(Self {
            kind: data[1],
            version: u16::from_be_bytes([data[2], data[3]]),
            pcb: data[4],
            timestamp: u32::from_be_bytes([data[5], data[6], data[7], data[8]]),
        })
    }

    pub fn kind(&self) -> char {
        self.kind as char
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn pcb(&self) -> u8 {
        self.pcb
    }

    /// The build date of the firmware, as a UNIX timestamp.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn pcb_name(&self) -> &'static str {
        match self.pcb {
            4 => "v1.3",
            5 => "v1.4",
            6 => "v1.4a/b/c",
            101 => "Mini v1.0d",
            _ => "unknown",
        }
    }

    /// If the firmware speaks the protocol of this module, which is
    /// the case for the L firmware.
    pub fn is_supported(&self) -> bool {
        self.kind == b'L'
    }
}

impl Display for Firmware {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GBxCart RW {} (firmware {}{})",
            self.pcb_name(),
            self.kind(),
            self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{
        cart_write, set_variable, Firmware, ADDRESS, DMG_ACCESS_MODE, DMG_READ_CS_PULSE,
        TRANSFER_SIZE,
    };

    /// The reply to the firmware query of a GBxCart RW v1.3 running
    /// the L1 firmware, as captured from the device.
    const FIRMWARE_L1: [u8; 9] = [0x08, 0x4c, 0x00, 0x01, 0x04, 0x65, 0xe2, 0x1d, 0xea];

    #[test]
    fn test_set_variable() {
        assert_eq!(
            set_variable(TRANSFER_SIZE, 0x40),
            vec![0xa6, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40]
        );
        assert_eq!(
            set_variable(ADDRESS, 0xa000),
            vec![0xa6, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa0, 0x00]
        );
        assert_eq!(
            set_variable(DMG_ACCESS_MODE, 0x03),
            vec![0xa6, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03]
        );
        assert_eq!(
            set_variable(DMG_READ_CS_PULSE, 0x01),
            vec![0xa6, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn test_cart_write() {
        assert_eq!(
            cart_write(0x6000, 0x01),
            vec![0xb2, 0x00, 0x00, 0x60, 0x00, 0x01]
        );
        assert_eq!(
            cart_write(0x0000, 0x0a),
            vec![0xb2, 0x00, 0x00, 0x00, 0x00, 0x0a]
        );
        assert_eq!(
            cart_write(0xffff, 0xff),
            vec![0xb2, 0x00, 0x00, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn test_firmware_from_data() {
        let firmware = Firmware::from_data(&FIRMWARE_L1).unwrap();
        assert_eq!(firmware.kind(), 'L');
        assert_eq!(firmware.version(), 1);
        assert_eq!(firmware.pcb(), 4);
        assert_eq!(firmware.timestamp(), 1709317610);
    }

    #[test]
    fn test_firmware_from_data_short() {
        assert_eq!(
            Firmware::from_data(&FIRMWARE_L1[..8]).err(),
            Some(Error::DataError(String::from(
                "Invalid firmware information from the device"
            )))
        );
    }

    #[test]
    fn test_firmware_from_data_empty() {
        assert!(Firmware::from_data(&[]).is_err());
    }

    #[test]
    fn test_firmware_from_data_invalid_length() {
        let mut data = FIRMWARE_L1;
        data[0] = 0x09;
        assert!(Firmware::from_data(&data).is_err());
    }

    #[test]
    fn test_firmware_pcb_name() {
        for (pcb, name) in [
            (4, "v1.3"),
            (5, "v1.4"),
            (6, "v1.4a/b/c"),
            (101, "Mini v1.0d"),
            (2, "unknown"),
        ] {
            let mut data = FIRMWARE_L1;
            data[4] = pcb;
            assert_eq!(Firmware::from_data(&data).unwrap().pcb_name(), name);
        }
    }

    #[test]
    fn test_firmware_is_supported() {
        assert!(Firmware::from_data(&FIRMWARE_L1).unwrap().is_supported());
    }

    #[test]
    fn test_firmware_is_supported_other() {
        let mut data = FIRMWARE_L1;
        data[1] = b'R';
        assert!(!Firmware::from_data(&data).unwrap().is_supported());
    }

    #[test]
    fn test_firmware_display() {
        let firmware = Firmware::from_data(&FIRMWARE_L1).unwrap();
        assert_eq!(format!("{firmware}"), "GBxCart RW v1.3 (firmware L1)");
    }
}
