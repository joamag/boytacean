//! Loading of games straight from their cartridge, through a
//! GBxCart RW connected to a serial port.

use std::{
    fs::remove_file,
    io::{stdout, Write},
    path::Path,
    time::Duration,
};

use boytacean_common::{
    error::Error,
    util::{read_file, write_file},
};
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

    write_files(
        Path::new("."),
        &dump.title(),
        dump.rom_data(),
        dump.ram_data().map(Vec::as_slice),
    )
}

/// Writes the ROM and the save (if any) of a cartridge into files in
/// the provided directory, returning the path of the ROM file.
///
/// The save in the cartridge is only written when there's no save for
/// it yet, so that the progress made in the emulator is never lost.
fn write_files(
    directory: &Path,
    title: &str,
    rom_data: &[u8],
    ram_data: Option<&[u8]>,
) -> Result<String, Error> {
    let (rom_name, ram_name) = file_paths(title);
    let rom_path = directory.join(rom_name).to_string_lossy().into_owned();
    let ram_path = directory.join(ram_name).to_string_lossy().into_owned();

    // another game with a similar title may have been loaded from its
    // cartridge before, whose save must not be used for this one, while
    // the same ROM is left untouched
    if !Path::new(&rom_path).exists() {
        write_new_file(&rom_path, rom_data)?;
    } else if read_file(&rom_path)? != rom_data {
        return Err(Error::CustomError(format!(
            "A different ROM was loaded from a cartridge as {rom_path}, move it and its save away to load this one"
        )));
    }
    if let Some(ram_data) = ram_data {
        if Path::new(&ram_path).exists() {
            println!(
                "Using the existing save {ram_path}, delete it to use the save in the cartridge"
            );
        } else {
            write_new_file(&ram_path, ram_data)?;
        }
    }
    Ok(rom_path)
}

/// Writes the data into a new file, removing the file in case the write
/// fails, so that an incomplete file is never taken as a complete one.
fn write_new_file(path: &str, data: &[u8]) -> Result<(), Error> {
    if let Err(err) = write_file(path, data, None) {
        remove_file(path).ok();
        return Err(err);
    }
    Ok(())
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
    use std::{
        env::temp_dir,
        fs::{create_dir_all, read, remove_dir_all, write},
        path::{Path, PathBuf},
        process,
    };

    use boytacean_common::error::Error;
    use serialport::{SerialPortType, UsbPortInfo};

    use super::{dump_cartridge, file_paths, is_gbxcart, write_files, write_new_file};

    /// Builds an empty directory for the files of a test, unique to the
    /// test and to the process running it, so that runs never collide.
    fn build_directory(name: &str) -> PathBuf {
        let directory = temp_dir().join(format!("boytacean-{name}-{}", process::id()));
        if directory.exists() {
            remove_dir_all(&directory).unwrap();
        }
        create_dir_all(&directory).unwrap();
        directory
    }

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
    fn test_dump_cartridge_invalid_port() {
        let result = dump_cartridge("/dev/boytacean-none");
        assert!(matches!(
            result,
            Err(Error::IoError(message)) if message.starts_with("Failed to open /dev/boytacean-none: ")
        ));
    }

    #[test]
    fn test_write_files() {
        let directory = build_directory("cartridge-write");
        let rom_path = write_files(
            &directory,
            "BOYTACEAN WRITE TEST",
            &[0x01, 0x02, 0x03],
            Some(&[0x04, 0x05]),
        )
        .unwrap();
        let ram_path = directory.join("boytacean-write-test.cartridge.sav");
        assert_eq!(
            Path::new(&rom_path),
            directory.join("boytacean-write-test.cartridge.gb")
        );
        assert_eq!(read(&rom_path).unwrap(), vec![0x01, 0x02, 0x03]);
        assert_eq!(read(&ram_path).unwrap(), vec![0x04, 0x05]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_files_existing_save() {
        // the save already in place holds the progress made in the
        // emulator, which must never be replaced by the cartridge one
        let directory = build_directory("cartridge-existing");
        let ram_path = directory.join("boytacean-existing-test.cartridge.sav");
        write(&ram_path, [0x0a, 0x0b, 0x0c]).unwrap();
        let rom_path = write_files(
            &directory,
            "BOYTACEAN EXISTING TEST",
            &[0x01, 0x02, 0x03],
            Some(&[0x04, 0x05]),
        )
        .unwrap();
        assert_eq!(read(&rom_path).unwrap(), vec![0x01, 0x02, 0x03]);
        assert_eq!(read(&ram_path).unwrap(), vec![0x0a, 0x0b, 0x0c]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_files_other_rom() {
        // the titles of both games share the same file names, but the
        // save of the first must never be used for the second
        let directory = build_directory("cartridge-other");
        let rom_path = write_files(
            &directory,
            "BOYTACEAN OTHER TEST",
            &[0x01, 0x02, 0x03],
            Some(&[0x04, 0x05]),
        )
        .unwrap();
        let ram_path = directory.join("boytacean-other-test.cartridge.sav");
        let result = write_files(
            &directory,
            "BOYTACEAN-OTHER-TEST",
            &[0x09, 0x09, 0x09],
            Some(&[0x07]),
        );
        assert_eq!(
            result,
            Err(Error::CustomError(format!(
                "A different ROM was loaded from a cartridge as {rom_path}, move it and its save away to load this one"
            )))
        );
        assert_eq!(read(&rom_path).unwrap(), vec![0x01, 0x02, 0x03]);
        assert_eq!(read(&ram_path).unwrap(), vec![0x04, 0x05]);

        // the same game keeps on being loaded with its existing save
        let result = write_files(
            &directory,
            "BOYTACEAN OTHER TEST",
            &[0x01, 0x02, 0x03],
            Some(&[0x06]),
        );
        assert_eq!(result, Ok(rom_path.clone()));
        assert_eq!(read(&ram_path).unwrap(), vec![0x04, 0x05]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_files_no_save() {
        let directory = build_directory("cartridge-no-save");
        let rom_path = write_files(&directory, "BOYTACEAN NO SAVE TEST", &[0x01], None).unwrap();
        assert_eq!(read(&rom_path).unwrap(), vec![0x01]);
        assert!(!directory
            .join("boytacean-no-save-test.cartridge.sav")
            .exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_files_invalid_directory() {
        let directory = Path::new("/dev/boytacean-none");
        let result = write_files(directory, "BOYTACEAN INVALID TEST", &[0x01], None);
        let rom_path = directory.join("boytacean-invalid-test.cartridge.gb");
        assert_eq!(
            result,
            Err(Error::CustomError(format!(
                "Failed to create file: {}",
                rom_path.to_string_lossy()
            )))
        );
    }

    #[test]
    fn test_write_new_file() {
        let directory = build_directory("cartridge-new");
        let path = directory
            .join("boytacean.gb")
            .to_string_lossy()
            .into_owned();
        write_new_file(&path, &[0x01, 0x02]).unwrap();
        assert_eq!(read(&path).unwrap(), vec![0x01, 0x02]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_new_file_invalid() {
        let path = Path::new("/dev/boytacean-none").join("boytacean.gb");
        let path = path.to_string_lossy();
        assert_eq!(
            write_new_file(&path, &[0x01]),
            Err(Error::CustomError(format!("Failed to create file: {path}")))
        );
        assert!(!Path::new(path.as_ref()).exists());
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
