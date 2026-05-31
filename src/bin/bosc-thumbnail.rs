//! Thumbnail extractor for BOSC files
//!
//! This utility reads a compressed BOSC file and extracts the thumbnail image,
//! saving it as a BMP file. If no output file is specified, it will use the input file's
//! name with a `.bmp` extension.
//!
//! # Usage
//! bosc-thumbnail <bosc_file> \[thumbnail_file\]

use std::{env::args, error::Error, fs::File, io::Read, path::Path};

use boytacean::state::{SaveStateFormat, StateManager};

fn print_usage() {
    println!("Usage: bosc-thumbnail <bosc_file> [thumbnail_file]");
    println!("If thumbnail_file is not specified, it will use bosc_file with .bmp extension");
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let input_path = Path::new(&args[1]);
    let output_path = if args.len() > 2 {
        Path::new(&args[2]).to_path_buf()
    } else {
        let mut output = input_path.to_path_buf();
        output.set_extension("bmp");
        output
    };

    println!("Extracting BOSC thumbnail {}", output_path.display());

    // read input file with the compressed values
    let mut input_file = File::open(input_path)?;
    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // validates that input is BOSC format
    let format = StateManager::format(&input_data)?;
    if format != SaveStateFormat::Bosc {
        return Err("Input file is not in BOSC format".into());
    }

    // reads as BOSC and saves the thumbnail as BMP
    let mut bosc_state = StateManager::read_bosc(&input_data)?;
    bosc_state
        .bos()
        .save_image_bmp(output_path.to_str().ok_or("Invalid output path")?)?;

    println!("Successfully extracted thumbnail from BOSC file");

    Ok(())
}
