//! Serial port access to a GBxCart RW, finding the device by the USB
//! IDs of its serial converter and opening its port, so that any native
//! front-end can run the exchanges with the device over it.

use std::time::Duration;

use boytacean_common::error::Error;
use serialport::{available_ports, SerialPort, SerialPortType};

use crate::gbxcart::{BAUD_RATE, USB_PRODUCT_ID, USB_VENDOR_ID};

/// The time to wait for a reply of the device before giving up,
/// which is the case when it runs another firmware.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// Opens the serial port of the GBxCart RW at the provided port
/// (`auto` finds it), returning the open port and its name.
pub fn open_port(port: &str) -> Result<(Box<dyn SerialPort>, String), Error> {
    let port = if port == "auto" {
        find_port()?
    } else {
        String::from(port)
    };
    let serial = serialport::new(&port, BAUD_RATE)
        .timeout(TIMEOUT)
        .open()
        .map_err(|err| Error::IoError(format!("Failed to open {port}: {err}")))?;
    Ok((serial, port))
}

/// Finds the serial port of the first GBxCart RW connected, by the
/// USB IDs of its serial converter.
pub fn find_port() -> Result<String, Error> {
    let ports = available_ports().map_err(|err| Error::IoError(err.to_string()))?;
    ports
        .into_iter()
        .find(|port| is_gbxcart(&port.port_type))
        .map(|port| port.port_name)
        .ok_or(Error::CustomError(String::from(
            "No GBxCart RW found, connect it or provide its serial port",
        )))
}

pub fn is_gbxcart(port_type: &SerialPortType) -> bool {
    matches!(
        port_type,
        SerialPortType::UsbPort(info) if info.vid == USB_VENDOR_ID && info.pid == USB_PRODUCT_ID
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use boytacean_common::error::Error;
    use serialport::{SerialPortType, UsbPortInfo};

    use super::{is_gbxcart, open_port};

    fn build_usb_port(vid: u16, pid: u16) -> SerialPortType {
        SerialPortType::UsbPort(UsbPortInfo {
            vid,
            pid,
            serial_number: None,
            manufacturer: None,
            product: None,
        })
    }

    #[test]
    fn test_open_port_invalid() {
        let result = open_port("/dev/boytacean-none");
        assert!(matches!(
            result,
            Err(Error::IoError(message)) if message.starts_with("Failed to open /dev/boytacean-none: ")
        ));
    }

    #[test]
    fn test_open_port_auto() {
        // the device is either found and opened (when connected), the
        // ports fail to be listed or opened (eg: open in another program)
        // or the lack of it is reported, never another error
        match open_port("auto") {
            Ok((_, port)) => assert!(Path::new(&port).exists()),
            Err(Error::IoError(_)) => {}
            Err(err) => assert_eq!(
                err,
                Error::CustomError(String::from(
                    "No GBxCart RW found, connect it or provide its serial port"
                ))
            ),
        }
    }

    #[test]
    fn test_is_gbxcart() {
        assert!(is_gbxcart(&build_usb_port(0x1a86, 0x7523)));
    }

    #[test]
    fn test_is_gbxcart_other() {
        assert!(!is_gbxcart(&build_usb_port(0x1a86, 0x5523)));
        assert!(!is_gbxcart(&build_usb_port(0x0403, 0x7523)));
        assert!(!is_gbxcart(&SerialPortType::PciPort));
        assert!(!is_gbxcart(&SerialPortType::Unknown));
    }
}
