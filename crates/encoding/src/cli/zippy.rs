use boytacean_common::util::{read_file, write_file};
use boytacean_encoding::zippy::encode_zippy;
use std::{env, error::Error, process};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    let data = read_file(input)?;
    let encoded = encode_zippy(&data, None, None)?;
    write_file(output, &encoded, None)?;

    Ok(())
}
