//! Loading of games straight from their cartridge, through a
//! GBxCart RW connected to a serial port.

use std::{
    io::{stdout, Write},
    path::Path,
};

use boytacean_cartridge::{dumper::Dumper, serial::open_port};
use boytacean_common::{
    error::Error,
    util::{read_file, write_new_file},
};

/// Dumps the cartridge in the GBxCart RW at the provided serial port
/// (`auto` finds it) into a ROM file in the current directory, so that
/// it can be loaded as any other ROM file, returning its path.
pub fn dump_cartridge(port: &str) -> Result<String, Error> {
    let (mut serial, port) = open_port(port)?;

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

    use super::{dump_cartridge, file_paths, write_files};

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
                "Failed to create file: {}.{}.part",
                rom_path.to_string_lossy(),
                process::id()
            )))
        );
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
