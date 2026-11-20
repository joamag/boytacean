//! GBxCart RW utility, to dump and flash Game Boy cartridges
//!
//! This utility talks to a GBxCart RW running the L firmware, through its
//! serial port, to probe the cartridge for a flash chip, to dump the
//! cartridge into a ROM file (and its save) or to flash a ROM file into a
//! flash cartridge, backing up its current contents first.
//!
//! # Usage
//! gbxcart info \[--port port\]
//! gbxcart dump <output_file> \[--port port\]
//! gbxcart flash <rom_file> \[--port port\] \[--backup backup_file\] \[--no-backup\] \[--yes\]

use std::{
    env::args,
    error::Error,
    io::{stdout, Write},
    path::Path,
};

use boytacean_cartridge::{dumper::Dumper, flasher::Flasher, header::Header, serial::open_port};
use boytacean_common::util::{read_file, replace_ext, write_new_file};

/// The default file for the backup of a cartridge before flashing it.
const BACKUP_FILE: &str = "cartridge.backup.gb";

/// The options of a run of the utility, as provided in its arguments.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Options {
    command: String,
    file: Option<String>,
    port: String,
    backup: Option<String>,
    yes: bool,
}

fn print_usage() {
    println!("Usage: gbxcart <command> [options]");
    println!("  info                    Probes the cartridge for a flash chip");
    println!("  dump <output_file>      Dumps the cartridge into a ROM file (and its save)");
    println!("  flash <rom_file>        Flashes the ROM file into a flash cartridge");
    println!("Options:");
    println!("  --port <port>           Serial port of the GBxCart RW (found when omitted)");
    println!("  --backup <backup_file>  File for the backup made before flashing ({BACKUP_FILE})");
    println!("  --no-backup             Flashes without backing up the cartridge first");
    println!("  --yes                   Flashes the cartridge, otherwise only describing what it would do");
}

/// Parses the arguments of the utility, `None` when they are not valid.
fn parse_args(args: &[String]) -> Option<Options> {
    let mut options = Options {
        command: args.first()?.clone(),
        file: None,
        port: String::from("auto"),
        backup: Some(String::from(BACKUP_FILE)),
        yes: false,
    };
    let mut args = args[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => options.port = args.next()?.clone(),
            "--backup" => options.backup = Some(args.next()?.clone()),
            "--no-backup" => options.backup = None,
            "--yes" => options.yes = true,
            arg if arg.starts_with("--") || options.file.is_some() => return None,
            arg => options.file = Some(String::from(arg)),
        }
    }
    match (options.command.as_str(), &options.file) {
        ("info", None) | ("dump", Some(_)) | ("flash", Some(_)) => Some(options),
        _ => None,
    }
}

/// The path of the save next to the provided ROM file.
fn save_path(path: &str) -> String {
    replace_ext(path, "sav").unwrap_or(format!("{path}.sav"))
}

/// Builds the callback that prints the progress, only when its
/// percentage changes, as there are many thousands of exchanges.
fn print_progress(label: &str) -> impl FnMut(f32) + '_ {
    let mut percentage = None;
    move |progress| {
        let value = (progress * 100.0) as u8;
        if percentage != Some(value) {
            percentage = Some(value);
            print!("\r{label}... {value}%");
            stdout().flush().ok();
        }
    }
}

/// Writes the data into a new file, refusing to replace an existing one.
fn write_output(path: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if Path::new(path).exists() {
        return Err(format!("The file {path} already exists, move it away first").into());
    }
    write_new_file(path, data)?;
    println!("Wrote {path}");
    Ok(())
}

fn info(options: &Options) -> Result<(), Box<dyn Error>> {
    let (mut serial, port) = open_port(&options.port)?;
    let mut flasher = Flasher::probe();
    flasher.run(&mut serial, |_| {})?;
    if let Some(device) = flasher.device() {
        println!("{device} on {port}");
    }
    match flasher.chip() {
        Some(chip) => println!("Flash cartridge with a {chip}"),
        None => println!("No flash chip found, the cartridge can't be flashed"),
    }
    Ok(())
}

fn dump(options: &Options) -> Result<(), Box<dyn Error>> {
    let path = options.file.as_deref().unwrap_or_default();
    let (mut serial, port) = open_port(&options.port)?;
    let mut dumper = Dumper::new();
    let dump = dumper.run(&mut serial, print_progress("Reading cartridge"))?;
    println!();
    if let Some(device) = dumper.device() {
        println!("Read \"{}\" with {device} on {port}", dump.title());
    }
    if !dump.checksum_valid() {
        println!("The ROM checksum does not match its header, usual for reproduction cartridges");
    }
    write_output(path, dump.rom_data())?;
    if let Some(ram_data) = dump.ram_data() {
        write_output(&save_path(path), ram_data)?;
    }
    Ok(())
}

fn flash(options: &Options) -> Result<(), Box<dyn Error>> {
    let path = options.file.as_deref().unwrap_or_default();
    let rom = read_file(path)?;
    let mut flasher = Flasher::new(rom.clone())?;

    // flash cartridges emulate an MBC5, which ROMs for other MBCs may
    // not run on, so the header of the ROM is checked for it
    if let Ok(header) = Header::from_data(rom.get(0x0100..0x0150).unwrap_or_default()) {
        if !matches!(header.cartridge_type(), 0x00 | 0x19..=0x1e) {
            println!(
                "The ROM is for a cartridge of type 0x{:02x}, it may not run correctly on a flash cartridge, which emulates an MBC5",
                header.cartridge_type()
            );
        }
    }

    // without an explicit confirmation the cartridge is only probed,
    // describing what flashing it would do
    let (mut serial, port) = open_port(&options.port)?;
    if !options.yes {
        let mut probe = Flasher::probe();
        probe.run(&mut serial, |_| {})?;
        if let Some(device) = probe.device() {
            println!("{device} on {port}");
        }
        match probe.chip() {
            Some(chip) => println!(
                "Flash cartridge with a {chip}, run again with --yes to erase it and flash {path} ({} KiB) into it",
                rom.len() / 1024
            ),
            None => println!("No flash chip found, the cartridge can't be flashed"),
        }
        return Ok(());
    }

    if let Some(backup) = &options.backup {
        if Path::new(backup).exists() {
            return Err(format!(
                "The backup file {backup} already exists, move it away or use --backup"
            )
            .into());
        }
        let mut dumper = Dumper::new();
        let dump = dumper
            .run(&mut serial, print_progress("Backing up cartridge"))
            .map_err(|err| {
                format!("Failed to back up the cartridge ({err}), use --no-backup for a blank one")
            })?;
        println!();
        write_output(backup, dump.rom_data())?;
    }

    flasher.run(&mut serial, print_progress("Flashing cartridge"))?;
    println!();
    if let (Some(device), Some(chip)) = (flasher.device(), flasher.chip()) {
        println!("Flashed {path} into the {chip} with {device} on {port}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        print_usage();
        return Ok(());
    };
    match options.command.as_str() {
        "info" => info(&options),
        "dump" => dump(&options),
        _ => flash(&options),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{remove_file, write},
        process,
    };

    use super::{dump, flash, info, parse_args, save_path, write_output, Options, BACKUP_FILE};

    fn build_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| String::from(*arg)).collect()
    }

    fn build_options(command: &str, file: Option<&str>) -> Options {
        Options {
            command: String::from(command),
            file: file.map(String::from),
            port: String::from("/dev/boytacean-none"),
            backup: Some(String::from(BACKUP_FILE)),
            yes: false,
        }
    }

    #[test]
    fn test_parse_args() {
        assert_eq!(
            parse_args(&build_args(&["info"])),
            Some(Options {
                command: String::from("info"),
                file: None,
                port: String::from("auto"),
                backup: Some(String::from(BACKUP_FILE)),
                yes: false,
            })
        );
        assert_eq!(
            parse_args(&build_args(&["dump", "game.gb", "--port", "/dev/ttyUSB0"])),
            Some(Options {
                command: String::from("dump"),
                file: Some(String::from("game.gb")),
                port: String::from("/dev/ttyUSB0"),
                backup: Some(String::from(BACKUP_FILE)),
                yes: false,
            })
        );
        assert_eq!(
            parse_args(&build_args(&[
                "flash", "--yes", "game.gb", "--backup", "old.gb"
            ])),
            Some(Options {
                command: String::from("flash"),
                file: Some(String::from("game.gb")),
                port: String::from("auto"),
                backup: Some(String::from("old.gb")),
                yes: true,
            })
        );
        assert_eq!(
            parse_args(&build_args(&["flash", "game.gb", "--no-backup"]))
                .unwrap()
                .backup,
            None
        );
    }

    #[test]
    fn test_parse_args_invalid() {
        for args in [
            vec![],
            vec!["erase"],
            vec!["info", "game.gb"],
            vec!["dump"],
            vec!["flash"],
            vec!["flash", "game.gb", "other.gb"],
            vec!["flash", "game.gb", "--force"],
            vec!["dump", "game.gb", "--port"],
            vec!["flash", "game.gb", "--backup"],
        ] {
            assert_eq!(parse_args(&build_args(&args)), None);
        }
    }

    #[test]
    fn test_save_path() {
        assert_eq!(save_path("game.gb"), "game.sav");
        assert_eq!(save_path("dumps/game.gbc"), "dumps/game.sav");
        assert_eq!(save_path("game"), "game.sav");
    }

    #[test]
    fn test_write_output() {
        let path = temp_dir()
            .join(format!("boytacean-gbxcart-{}.gb", process::id()))
            .to_string_lossy()
            .into_owned();
        write_output(&path, &[0x01, 0x02]).unwrap();
        assert_eq!(
            write_output(&path, &[0x03]).unwrap_err().to_string(),
            format!("The file {path} already exists, move it away first")
        );
        remove_file(path).unwrap();
    }

    #[test]
    fn test_info_invalid_port() {
        let result = info(&build_options("info", None));
        assert!(result
            .unwrap_err()
            .to_string()
            .starts_with("IO error: Failed to open /dev/boytacean-none: "));
    }

    #[test]
    fn test_dump_invalid_port() {
        let result = dump(&build_options("dump", Some("game.gb")));
        assert!(result
            .unwrap_err()
            .to_string()
            .starts_with("IO error: Failed to open /dev/boytacean-none: "));
    }

    #[test]
    fn test_flash_missing_rom() {
        let result = flash(&build_options("flash", Some("/dev/boytacean-none.gb")));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to open file: /dev/boytacean-none.gb"
        );
    }

    #[test]
    fn test_flash_invalid_rom() {
        // a ROM file must be made of whole ROM banks
        let path = temp_dir()
            .join(format!("boytacean-gbxcart-invalid-{}.gb", process::id()))
            .to_string_lossy()
            .into_owned();
        write(&path, [0x00; 0x100]).unwrap();
        let result = flash(&build_options("flash", Some(&path)));
        assert_eq!(result.unwrap_err().to_string(), "Invalid ROM size");
        remove_file(path).unwrap();
    }
}
