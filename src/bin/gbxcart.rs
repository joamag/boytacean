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
    io::{stdout, Read, Write},
    path::Path,
};

use boytacean_cartridge::{
    dumper::Dumper, flasher::Flasher, header::Header, mapper::Mapper, serial::open_port,
};
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

/// Checks that there's no file at the path, never to replace one.
fn check_output(path: &str) -> Result<(), Box<dyn Error>> {
    if Path::new(path).exists() {
        return Err(format!("The file {path} already exists, move it away first").into());
    }
    Ok(())
}

/// Writes the data into a new file, refusing to replace an existing one.
fn write_output(path: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    check_output(path)?;
    write_new_file(path, data)?;
    println!("Wrote {path}");
    Ok(())
}

fn info<T: Read + Write>(transport: &mut T, port: &str) -> Result<(), Box<dyn Error>> {
    let mut flasher = Flasher::probe();
    flasher.run(transport, |_| {})?;
    if let Some(device) = flasher.device() {
        println!("{device} on {port}");
    }
    match flasher.chip() {
        Some(chip) => println!("Flash cartridge with a {chip}"),
        None => println!("No flash chip found, the cartridge can't be flashed"),
    }
    Ok(())
}

fn dump<T: Read + Write>(
    transport: &mut T,
    port: &str,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    let path = options.file.as_deref().unwrap_or_default();
    let mut dumper = Dumper::new();
    let dump = dumper.run(transport, print_progress("Reading cartridge"))?;
    println!();
    if let Some(device) = dumper.device() {
        println!("Read \"{}\" with {device} on {port}", dump.title());
    }
    if !dump.checksum_valid() {
        println!("The ROM checksum does not match its header, usual for reproduction cartridges");
    }

    // checks every file before writing any of them, so that an existing
    // save never leaves the ROM written without its save
    check_output(path)?;
    if dump.ram_data().is_some() {
        check_output(&save_path(path))?;
    }
    write_output(path, dump.rom_data())?;
    if let Some(ram_data) = dump.ram_data() {
        write_output(&save_path(path), ram_data)?;
    }
    Ok(())
}

fn flash<T: Read + Write>(
    transport: &mut T,
    port: &str,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
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

    if let Some(backup) = &options.backup {
        if options.yes && Path::new(backup).exists() {
            return Err(format!(
                "The backup file {backup} already exists, move it away or use --backup"
            )
            .into());
        }
    }

    // the cartridge is probed first, so that a cartridge that can't be
    // flashed is neither backed up nor touched, and without an explicit
    // confirmation the probe only describes what flashing would do
    let mut probe = Flasher::probe();
    probe.run(transport, |_| {})?;
    if let Some(device) = probe.device() {
        println!("{device} on {port}");
    }
    let Some(chip) = probe.chip() else {
        return Err("No flash chip found, the cartridge can't be flashed".into());
    };
    if !options.yes {
        println!(
            "Flash cartridge with a {chip}, run again with --yes to erase it and flash {path} ({} KiB) into it",
            rom.len() / 1024
        );
        return Ok(());
    }

    // the backup is read through the MBC5 of the flash cartridge, as the
    // MBC declared by the game it holds may select other banks on it
    if let Some(backup) = &options.backup {
        let mut dumper = Dumper::with_mapper(Mapper::Mbc5);
        let dump = dumper
            .run(transport, print_progress("Backing up cartridge"))
            .map_err(|err| {
                format!("Failed to back up the cartridge ({err}), use --no-backup for a blank one")
            })?;
        println!();
        write_output(backup, dump.rom_data())?;
    }

    // an interrupted or failed flashing never damages the cartridge, as
    // flashing it again always recovers it, which is told to the user
    println!("Flashing can be run again at any point, if it's interrupted");
    flasher
        .run(transport, print_progress("Flashing cartridge"))
        .map_err(|err| {
            format!("Failed to flash the cartridge ({err}), flashing it again recovers it")
        })?;
    println!();
    println!("Flashed {path} into the {chip}");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        print_usage();
        return Ok(());
    };
    let (mut serial, port) = open_port(&options.port)?;
    match options.command.as_str() {
        "info" => info(&mut serial, &port),
        "dump" => dump(&mut serial, &port, &options),
        _ => flash(&mut serial, &port, &options),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        env::temp_dir,
        fs::{create_dir_all, read, remove_dir_all, write},
        io::{self, Read, Write},
        path::{Path, PathBuf},
        process,
    };

    use boytacean_cartridge::header::NINTENDO_LOGO;

    use super::{
        check_output, dump, flash, info, parse_args, save_path, write_output, Options, BACKUP_FILE,
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

    /// A transport that replays the replies of a device in order,
    /// ignoring what is sent to it, as the exchanges themselves are
    /// covered by the tests of the cartridge crate.
    struct Replay {
        replies: VecDeque<u8>,
    }

    impl Replay {
        fn new(replies: Vec<u8>) -> Self {
            Self {
                replies: replies.into(),
            }
        }
    }

    impl Read for Replay {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let count = buf.len().min(self.replies.len());
            for byte in buf.iter_mut().take(count) {
                *byte = self.replies.pop_front().unwrap();
            }
            Ok(count)
        }
    }

    impl Write for Replay {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

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

    fn build_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| String::from(*arg)).collect()
    }

    fn build_options(command: &str, file: &Path, backup: Option<&Path>, yes: bool) -> Options {
        Options {
            command: String::from(command),
            file: Some(file.to_string_lossy().into_owned()),
            port: String::from("auto"),
            backup: backup.map(|backup| backup.to_string_lossy().into_owned()),
            yes,
        }
    }

    /// Builds a 32 KiB ROM with a valid header for the provided cartridge
    /// type and RAM size, its bytes mixing their offsets.
    fn build_rom(cartridge_type: u8, ram_size: u8) -> Vec<u8> {
        let mut rom: Vec<u8> = (0..0x8000).map(|index| (index >> 3) as u8).collect();
        rom[0x0100..0x0150].fill(0x00);
        rom[0x0104..0x0134].copy_from_slice(&NINTENDO_LOGO);
        rom[0x0134..0x0138].copy_from_slice(b"GAME");
        rom[0x0147] = cartridge_type;
        rom[0x0149] = ram_size;
        rom[0x014d] = rom[0x0134..0x014d]
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1));
        rom
    }

    /// The replies of the dump of a cartridge with the provided ROM and
    /// RAM: the firmware, the MBC reset, the header twice, the ROM and
    /// the RAM.
    fn dump_replies(rom: &[u8], ram: &[u8]) -> Vec<u8> {
        let mut replies = FIRMWARE_L1.to_vec();
        replies.push(0x01);
        for _ in 0..2 {
            replies.extend_from_slice(&rom[0x0100..0x0150]);
        }
        replies.extend_from_slice(rom);
        replies.extend_from_slice(ram);
        replies
    }

    /// The replies of the probe of a cartridge, with or without a flash
    /// chip: the firmware, the MBC reset, the start of the cartridge and
    /// the CFI table, with every value showing up twice.
    fn probe_replies(chip: bool) -> Vec<u8> {
        let mut replies = FIRMWARE_L1.to_vec();
        replies.push(0x01);
        replies.extend(vec![0x00; 64]);
        let mut cfi = vec![0x00; 256];
        if chip {
            for (offset, value) in CFI_4MB.iter().enumerate() {
                cfi[(0x10 + offset) * 2] = *value;
                cfi[(0x10 + offset) * 2 + 1] = *value;
            }
        }
        replies.extend(cfi);
        replies
    }

    /// The replies of the flashing of a 32 KiB ROM: the probe, then for
    /// both banks the erase of their two sectors and the acknowledgement
    /// of every program, and the ROM read back.
    fn flash_replies(rom: &[u8]) -> Vec<u8> {
        let mut replies = probe_replies(true);
        for _ in 0..2 {
            replies.extend([0xff; 4]);
            replies.extend(vec![0x01; 64]);
        }
        replies.extend_from_slice(rom);
        replies
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
    fn test_check_output() {
        let directory = build_directory("gbxcart-check");
        let path = directory.join("game.gb").to_string_lossy().into_owned();
        check_output(&path).unwrap();
        write(&path, [0x0a]).unwrap();
        assert_eq!(
            check_output(&path).unwrap_err().to_string(),
            format!("The file {path} already exists, move it away first")
        );
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_write_output() {
        let directory = build_directory("gbxcart-output");
        let path = directory.join("game.gb").to_string_lossy().into_owned();
        write_output(&path, &[0x01, 0x02]).unwrap();
        assert_eq!(
            write_output(&path, &[0x03]).unwrap_err().to_string(),
            format!("The file {path} already exists, move it away first")
        );
        assert_eq!(read(&path).unwrap(), vec![0x01, 0x02]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_info() {
        for chip in [true, false] {
            let mut transport = Replay::new(probe_replies(chip));
            info(&mut transport, "/dev/ttyUSB0").unwrap();
            assert!(transport.replies.is_empty());
        }
    }

    #[test]
    fn test_dump() {
        let directory = build_directory("gbxcart-dump");
        let rom = build_rom(0x00, 0x00);
        let mut transport = Replay::new(dump_replies(&rom, &[]));
        let options = build_options("dump", &directory.join("game.gb"), None, false);
        dump(&mut transport, "/dev/ttyUSB0", &options).unwrap();
        assert_eq!(read(directory.join("game.gb")).unwrap(), rom);
        assert!(!directory.join("game.sav").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_dump_save() {
        let directory = build_directory("gbxcart-save");
        let rom = build_rom(0x03, 0x02);
        let ram: Vec<u8> = (0..0x2000).map(|index| index as u8).collect();
        let mut transport = Replay::new(dump_replies(&rom, &ram));
        let options = build_options("dump", &directory.join("game.gb"), None, false);
        dump(&mut transport, "/dev/ttyUSB0", &options).unwrap();
        assert_eq!(read(directory.join("game.gb")).unwrap(), rom);
        assert_eq!(read(directory.join("game.sav")).unwrap(), ram);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_dump_existing() {
        let directory = build_directory("gbxcart-existing");
        write(directory.join("game.gb"), [0x0a]).unwrap();
        let rom = build_rom(0x00, 0x00);
        let mut transport = Replay::new(dump_replies(&rom, &[]));
        let options = build_options("dump", &directory.join("game.gb"), None, false);
        let result = dump(&mut transport, "/dev/ttyUSB0", &options);
        assert!(result
            .unwrap_err()
            .to_string()
            .ends_with("already exists, move it away first"));
        assert_eq!(read(directory.join("game.gb")).unwrap(), vec![0x0a]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_dump_existing_save() {
        // an existing save stops the dump before the ROM is written, so
        // that a retry once it's moved away writes both
        let directory = build_directory("gbxcart-existing-save");
        write(directory.join("game.sav"), [0x0a]).unwrap();
        let rom = build_rom(0x03, 0x02);
        let ram = vec![0x55; 0x2000];
        let mut transport = Replay::new(dump_replies(&rom, &ram));
        let options = build_options("dump", &directory.join("game.gb"), None, false);
        let result = dump(&mut transport, "/dev/ttyUSB0", &options);
        assert!(result
            .unwrap_err()
            .to_string()
            .ends_with("game.sav already exists, move it away first"));
        assert!(!directory.join("game.gb").exists());
        assert_eq!(read(directory.join("game.sav")).unwrap(), vec![0x0a]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash() {
        // the cartridge is backed up before it's flashed, the ROM being
        // for an MBC1 (only a warning for flash cartridges)
        let directory = build_directory("gbxcart-flash");
        let old_rom = build_rom(0x00, 0x00);
        let rom = build_rom(0x03, 0x02);
        write(directory.join("game.gb"), &rom).unwrap();
        let mut replies = probe_replies(true);
        replies.extend(dump_replies(&old_rom, &[]));
        replies.extend(flash_replies(&rom));
        let mut transport = Replay::new(replies);
        let options = build_options(
            "flash",
            &directory.join("game.gb"),
            Some(&directory.join("backup.gb")),
            true,
        );
        flash(&mut transport, "/dev/ttyUSB0", &options).unwrap();
        assert_eq!(read(directory.join("backup.gb")).unwrap(), old_rom);
        assert!(transport.replies.is_empty());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_no_backup() {
        let directory = build_directory("gbxcart-no-backup");
        let rom = build_rom(0x19, 0x00);
        write(directory.join("game.gb"), &rom).unwrap();
        let mut replies = probe_replies(true);
        replies.extend(flash_replies(&rom));
        let mut transport = Replay::new(replies);
        let options = build_options("flash", &directory.join("game.gb"), None, true);
        flash(&mut transport, "/dev/ttyUSB0", &options).unwrap();
        assert!(transport.replies.is_empty());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_failure() {
        // a failed flashing tells that flashing it again recovers it
        let directory = build_directory("gbxcart-failure");
        let rom = build_rom(0x19, 0x00);
        write(directory.join("game.gb"), &rom).unwrap();
        let mut replies = probe_replies(true);
        replies.extend(flash_replies(&rom));

        // past the probe of the tool, the probe of the flashing and the
        // erase of the first two sectors is the first acknowledgement
        let program = probe_replies(true).len() * 2 + 4;
        replies[program] = 0x00;
        let mut transport = Replay::new(replies);
        let options = build_options("flash", &directory.join("game.gb"), None, true);
        let result = flash(&mut transport, "/dev/ttyUSB0", &options);
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to flash the cartridge (Data error: Failed to program the flash chip at 0x000000), flashing it again recovers it"
        );
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_confirmation() {
        // without --yes the cartridge is only probed, never backed up
        // nor flashed
        let directory = build_directory("gbxcart-confirmation");
        write(directory.join("game.gb"), build_rom(0x19, 0x00)).unwrap();
        let mut transport = Replay::new(probe_replies(true));
        let options = build_options(
            "flash",
            &directory.join("game.gb"),
            Some(&directory.join("backup.gb")),
            false,
        );
        flash(&mut transport, "/dev/ttyUSB0", &options).unwrap();
        assert!(transport.replies.is_empty());
        assert!(!directory.join("backup.gb").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_no_chip() {
        // a cartridge without a flash chip is refused before it's backed
        // up, with or without --yes
        let directory = build_directory("gbxcart-no-chip");
        write(directory.join("game.gb"), build_rom(0x19, 0x00)).unwrap();
        for yes in [false, true] {
            let mut transport = Replay::new(probe_replies(false));
            let options = build_options(
                "flash",
                &directory.join("game.gb"),
                Some(&directory.join("backup.gb")),
                yes,
            );
            let result = flash(&mut transport, "/dev/ttyUSB0", &options);
            assert_eq!(
                result.unwrap_err().to_string(),
                "No flash chip found, the cartridge can't be flashed"
            );
            assert!(transport.replies.is_empty());
            assert!(!directory.join("backup.gb").exists());
        }
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_existing_backup() {
        let directory = build_directory("gbxcart-existing-backup");
        write(directory.join("game.gb"), build_rom(0x19, 0x00)).unwrap();
        write(directory.join("backup.gb"), [0x0a]).unwrap();
        let mut transport = Replay::new(vec![]);
        let options = build_options(
            "flash",
            &directory.join("game.gb"),
            Some(&directory.join("backup.gb")),
            true,
        );
        let result = flash(&mut transport, "/dev/ttyUSB0", &options);
        assert!(result
            .unwrap_err()
            .to_string()
            .ends_with("already exists, move it away or use --backup"));
        assert_eq!(read(directory.join("backup.gb")).unwrap(), vec![0x0a]);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_backup_failure() {
        // a blank cartridge (no valid header) can't be backed up, which
        // stops the flashing before anything is written
        let directory = build_directory("gbxcart-backup-failure");
        write(directory.join("game.gb"), build_rom(0x19, 0x00)).unwrap();
        let mut replies = probe_replies(true);
        replies.extend(dump_replies(&[0xff; 0x8000], &[]));
        let mut transport = Replay::new(replies);
        let options = build_options(
            "flash",
            &directory.join("game.gb"),
            Some(&directory.join("backup.gb")),
            true,
        );
        let result = flash(&mut transport, "/dev/ttyUSB0", &options);
        assert!(result
            .unwrap_err()
            .to_string()
            .ends_with("use --no-backup for a blank one"));
        assert!(!directory.join("backup.gb").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_flash_missing_rom() {
        let mut transport = Replay::new(vec![]);
        let options = build_options("flash", Path::new("/dev/boytacean-none.gb"), None, true);
        let result = flash(&mut transport, "/dev/ttyUSB0", &options);
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to open file: /dev/boytacean-none.gb"
        );
    }

    #[test]
    fn test_flash_invalid_rom() {
        // a ROM file must be made of whole ROM banks
        let directory = build_directory("gbxcart-invalid");
        write(directory.join("game.gb"), [0x00; 0x100]).unwrap();
        let mut transport = Replay::new(vec![]);
        let options = build_options("flash", &directory.join("game.gb"), None, true);
        let result = flash(&mut transport, "/dev/ttyUSB0", &options);
        assert_eq!(result.unwrap_err().to_string(), "Invalid ROM size");
        remove_dir_all(directory).unwrap();
    }
}
