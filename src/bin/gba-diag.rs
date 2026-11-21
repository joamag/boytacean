//! Diagnostic utility for GBA emulation
//!
//! Runs a GBA ROM for a given number of frames and dumps detailed
//! emulator state including CPU registers, PPU configuration, IRQ
//! status, timer state, and per-frame PPM snapshots, or traces its
//! DirectSound pipeline into a WAV file. The files are saved into the
//! output directory, the system temporary directory when omitted.
//!
//! # Usage
//! gba-diag <rom.gba> \[num_frames\] \[--bios <bios.bin>\] \[--audio\] \[--output <directory>\]

use std::{
    env::{args, temp_dir},
    error::Error,
    fs::create_dir_all,
    path::PathBuf,
};

use boytacean::gba::{
    diag::{run_audio_diagnostics, run_diagnostics},
    GameBoyAdvance,
};
use boytacean_common::util::read_file;

/// The default number of frames run.
const DEFAULT_FRAMES: u32 = 30;

/// The options of a run of the utility, as provided in its arguments.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Options {
    rom: String,
    frames: u32,
    bios: Option<String>,
    audio: bool,
    output: Option<String>,
}

fn print_usage() {
    println!("Usage: gba-diag <rom.gba> [num_frames] [options]");
    println!("If num_frames is not specified, it runs {DEFAULT_FRAMES} frames");
    println!("Options:");
    println!("  --bios <bios.bin>      Boots the ROM through the BIOS file");
    println!("  --audio                Traces the DirectSound pipeline, saving its output as WAV");
    println!("  --output <directory>   Directory for the saved files (system temporary directory)");
}

/// Parses the arguments of the utility, `None` when they are not valid.
fn parse_args(args: &[String]) -> Option<Options> {
    let mut options = Options {
        rom: args.first().filter(|arg| !arg.starts_with("--"))?.clone(),
        frames: DEFAULT_FRAMES,
        bios: None,
        audio: false,
        output: None,
    };
    let mut frames = None;
    let mut args = args[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bios" => options.bios = Some(args.next()?.clone()),
            "--audio" => options.audio = true,
            "--output" => options.output = Some(args.next()?.clone()),
            arg if arg.starts_with("--") || frames.is_some() => return None,
            arg => frames = Some(arg.parse::<u32>().ok().filter(|&value| value > 0)?),
        }
    }
    options.frames = frames.unwrap_or(DEFAULT_FRAMES);
    Some(options)
}

/// Loads the ROM (and the BIOS) of the options and runs the requested
/// diagnostics, saving their files into the output directory.
fn diagnose(options: &Options) -> Result<(), Box<dyn Error>> {
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&read_file(&options.rom)?)?;
    println!("Loaded: {} ({})", info.title(), info.game_code());

    if let Some(path) = &options.bios {
        gba.load_bios(&read_file(path)?);
        println!("BIOS loaded from {path}");
    }

    let directory = options
        .output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(temp_dir);
    create_dir_all(&directory)?;

    if options.audio {
        run_audio_diagnostics(&mut gba, options.frames, &directory)?;
    } else {
        run_diagnostics(&mut gba, options.frames, &directory)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        print_usage();
        return Ok(());
    };
    diagnose(&options)
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, read, remove_dir_all, write},
        path::{Path, PathBuf},
        process,
    };

    use super::{diagnose, parse_args, Options, DEFAULT_FRAMES};

    /// The test ROM run by the tests, relative to the package root.
    const ROM_PATH: &str = "res/roms.gba/test/jsmolka_gba-tests/arm.gba";

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

    fn build_options(frames: u32, audio: bool, output: &Path) -> Options {
        Options {
            rom: String::from(ROM_PATH),
            frames,
            bios: None,
            audio,
            output: Some(output.to_string_lossy().into_owned()),
        }
    }

    #[test]
    fn test_parse_args() {
        assert_eq!(
            parse_args(&build_args(&["game.gba"])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: DEFAULT_FRAMES,
                bios: None,
                audio: false,
                output: None,
            })
        );
        assert_eq!(
            parse_args(&build_args(&[
                "game.gba", "--audio", "100", "--bios", "bios.bin", "--output", "out"
            ])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: 100,
                bios: Some(String::from("bios.bin")),
                audio: true,
                output: Some(String::from("out")),
            })
        );
    }

    #[test]
    fn test_parse_args_invalid() {
        for args in [
            vec![],
            vec!["--audio"],
            vec!["game.gba", "100", "--bois", "x"],
            vec!["game.gba", "100", "200"],
            vec!["game.gba", "x"],
            vec!["game.gba", "0"],
            vec!["game.gba", "-1"],
            vec!["game.gba", "--bios"],
            vec!["game.gba", "--output"],
        ] {
            assert_eq!(parse_args(&build_args(&args)), None);
        }
    }

    #[test]
    fn test_diagnose() {
        // the output directory is created when missing
        let directory = build_directory("gba-diag");
        let output = directory.join("frames");
        diagnose(&build_options(2, false, &output)).unwrap();
        assert!(output.join("gba_frame_0.ppm").exists());
        let data = read(output.join("gba_frame_1.ppm")).unwrap();
        assert_eq!(&data[..15], b"P6\n240 160\n255\n");
        assert_eq!(data.len(), 15 + 240 * 160 * 3);
        assert!(!output.join("gba_frame_2.ppm").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_diagnose_audio() {
        let directory = build_directory("gba-diag-audio");
        diagnose(&build_options(2, true, &directory)).unwrap();
        let data = read(directory.join("gba_audio_diag.wav")).unwrap();
        assert_eq!(&data[..4], b"RIFF");
        assert!(!directory.join("gba_frame_0.ppm").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_diagnose_bios() {
        let directory = build_directory("gba-diag-bios");
        let bios = directory.join("bios.bin");
        write(&bios, vec![0x00; 0x4000]).unwrap();
        let mut options = build_options(1, false, &directory);
        options.bios = Some(bios.to_string_lossy().into_owned());
        diagnose(&options).unwrap();
        assert!(directory.join("gba_frame_0.ppm").exists());

        options.bios = Some(String::from("/dev/boytacean-none.bin"));
        assert_eq!(
            diagnose(&options).unwrap_err().to_string(),
            "Failed to open file: /dev/boytacean-none.bin"
        );
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_diagnose_missing_rom() {
        let directory = build_directory("gba-diag-missing");
        let mut options = build_options(1, false, &directory);
        options.rom = String::from("/dev/boytacean-none.gba");
        assert_eq!(
            diagnose(&options).unwrap_err().to_string(),
            "Failed to open file: /dev/boytacean-none.gba"
        );
        assert!(!directory.join("gba_frame_0.ppm").exists());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_diagnose_invalid_rom() {
        // a file without the fixed value of a GBA header is refused
        let directory = build_directory("gba-diag-invalid");
        let rom = directory.join("game.gba");
        write(&rom, vec![0x00; 0x200]).unwrap();
        let mut options = build_options(1, false, &directory);
        options.rom = rom.to_string_lossy().into_owned();
        assert!(diagnose(&options).is_err());
        assert!(!directory.join("gba_frame_0.ppm").exists());
        remove_dir_all(directory).unwrap();
    }
}
