//! Loading of games straight from their cartridge, through a
//! GBxCart RW connected to a serial port.

use std::{
    io::{stdout, Write},
    path::Path,
    time::Duration,
};

use boytacean_common::{error::Error, util::write_file};
use boytacean_dumper::{
    dumper::Dumper,
    gbxcart::{BAUD_RATE, USB_PRODUCT_ID, USB_VENDOR_ID},
};
use serialport::{available_ports, SerialPortType};

/// The time to wait for a reply of the device before giving up,
/// which is the case when it runs another firmware.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Dumps the cartridge in the GBxCart RW at the provided serial port
/// (`auto` finds it) into a ROM file in the current directory, so that
/// it can be loaded as any other ROM file, returning its path.
///
/// The save in the cartridge is only written when there's no save for
/// it yet, so that the progress made in the emulator is never lost.
pub fn dump_cartridge(port: &str) -> Result<String, Error> {
    let port = if port == "auto" {
        find_port()?
    } else {
        String::from(port)
    };
    let mut serial = serialport::new(&port, BAUD_RATE)
        .timeout(TIMEOUT)
        .open()
        .map_err(|err| Error::IoError(format!("Failed to open {port}: {err}")))?;

    // prints the progress only when its percentage changes, as there
    // are many thousands of exchanges with the device in a dump
    let mut percentage = None;
    let mut dumper = Dumper::new();
    let dump = dumper.run(&mut serial, |progress| {
        let value = (progress * 100.0) as u8;
        if percentage != Some(value) {
            percentage = Some(value);
            print!("\rReading cartridge... {value}%");
            stdout().flush().ok();
        }
    })?;
    println!();
    if let Some(device) = dumper.device() {
        println!("Read \"{}\" with {device} on {port}", dump.title());
    }
    if !dump.checksum_valid() {
        println!("The ROM checksum does not match its header, usual for reproduction cartridges");
    }

    let (rom_path, ram_path) = file_paths(&dump.title());
    write_file(&rom_path, dump.rom_data(), None)?;
    if let Some(ram_data) = dump.ram_data() {
        if Path::new(&ram_path).exists() {
            println!(
                "Using the existing save {ram_path}, delete it to use the save in the cartridge"
            );
        } else {
            write_file(&ram_path, ram_data, None)?;
        }
    }
    Ok(rom_path)
}

/// Finds the serial port of the first GBxCart RW connected, by the
/// USB IDs of its serial converter.
fn find_port() -> Result<String, Error> {
    let ports = available_ports().map_err(|err| Error::IoError(err.to_string()))?;
    ports
        .into_iter()
        .find(|port| is_gbxcart(&port.port_type))
        .map(|port| port.port_name)
        .ok_or(Error::CustomError(String::from(
            "No GBxCart RW found, connect it or provide its serial port",
        )))
}

fn is_gbxcart(port_type: &SerialPortType) -> bool {
    matches!(
        port_type,
        SerialPortType::UsbPort(info) if info.vid == USB_VENDOR_ID && info.pid == USB_PRODUCT_ID
    )
}

/// The paths of the ROM and save files for the cartridge with the
/// provided title, set apart from any other ROM file by their name.
fn file_paths(title: &str) -> (String, String) {
    let mut name = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_lowercase());
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
    }
    let name = name.trim_end_matches('-');
    let name = if name.is_empty() { "cartridge" } else { name };
    (
        format!("{name}.cartridge.gb"),
        format!("{name}.cartridge.sav"),
    )
}

#[cfg(test)]
mod tests {
    use serialport::{SerialPortType, UsbPortInfo};

    use super::{file_paths, is_gbxcart};

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

    #[test]
    fn test_file_paths() {
        assert_eq!(
            file_paths("PETAL LANES"),
            (
                String::from("petal-lanes.cartridge.gb"),
                String::from("petal-lanes.cartridge.sav")
            )
        );
    }

    #[test]
    fn test_file_paths_symbols() {
        assert_eq!(
            file_paths("  POKEMON: RED/BLUE!  ").0,
            String::from("pokemon-red-blue.cartridge.gb")
        );
        assert_eq!(file_paths("../../ETC").0, String::from("etc.cartridge.gb"));
    }

    #[test]
    fn test_file_paths_empty() {
        assert_eq!(file_paths("").0, String::from("cartridge.cartridge.gb"));
        assert_eq!(file_paths("?!?").0, String::from("cartridge.cartridge.gb"));
    }
}
