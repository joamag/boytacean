//! Assorted utility functions and structures.
//!
//! This module contains various utility functions and structures
//! that are used throughout the Boytacean codebase.

use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Read, Write},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::error::Error;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Shared mutable type able to be passed between types
/// allowing for circular referencing and interior mutability.
pub type SharedMut<T> = Rc<RefCell<T>>;

/// Shared thread type able to be passed between threads.
///
/// Significant performance overhead compared to `SharedMut`.
pub type SharedThread<T> = Arc<Mutex<T>>;

/// The size of a BMP file header in bytes.
const BMP_HEADER_SIZE: u32 = 54;

/// Reads the contents of the file at the given path into
/// a vector of bytes.
pub fn read_file(path: &str) -> Result<Vec<u8>, Error> {
    let mut file =
        File::open(path).map_err(|_| Error::CustomError(format!("Failed to open file: {path}")))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| Error::CustomError(format!("Failed to read from file: {path}")))?;
    Ok(data)
}

/// Writes the given data to the file at the given path.
pub fn write_file(path: &str, data: &[u8], flush: Option<bool>) -> Result<(), Error> {
    let mut file = File::create(path)
        .map_err(|_| Error::CustomError(format!("Failed to create file: {path}")))?;
    file.write_all(data)
        .map_err(|_| Error::CustomError(format!("Failed to write to file: {path}")))?;
    if flush.unwrap_or(true) {
        file.flush()
            .map_err(|_| Error::CustomError(format!("Failed to flush file: {path}")))?;
    }
    Ok(())
}

/// Replaces the extension in the given path with the provided extension.
///
/// This function allows for simple associated file discovery.
pub fn replace_ext(path: &str, new_extension: &str) -> Option<String> {
    let file_path = Path::new(path);
    let parent_dir = file_path.parent()?;
    let file_stem = file_path.file_stem()?;
    let file_extension = file_path.extension()?;
    if file_stem == file_extension {
        return None;
    }
    let new_file_name = format!("{}.{}", file_stem.to_str()?, new_extension);
    let new_file_path = parent_dir.join(new_file_name);
    Some(String::from(new_file_path.to_str()?))
}

/// Capitalizes the first character in the provided string.
pub fn capitalize(string: &str) -> String {
    let mut chars = string.chars();
    match chars.next() {
        None => String::new(),
        Some(chr) => chr.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Saves the pixel data as a BMP file at the specified path.
/// The pixel data should be in RGB format, with each pixel
/// represented by three bytes (red, green, blue).
///
/// This is a raw implementation of BMP file saving, not using any
/// external libraries. It writes the BMP file header and pixel data
/// directly to the file in the correct format.
pub fn save_bmp(path: &str, pixels: &[u8], width: u32, height: u32) -> Result<(), Error> {
    let file = File::create(path)
        .map_err(|_| Error::CustomError(format!("Failed to create file: {path}")))?;
    let mut writer = BufWriter::new(file);

    // calculates the size of the BMP file header and the pixel data
    // according to the BMP file format specification
    let row_bytes = (width * 3 + 3) & !3;
    let image_size = row_bytes * height;
    let file_size = BMP_HEADER_SIZE + image_size;

    // writes the BMP file header into the writer
    writer.write_all(&[0x42, 0x4d]).unwrap(); // "BM" magic number
    writer.write_all(&file_size.to_le_bytes()).unwrap(); // file size
    writer.write_all(&[0x00, 0x00]).unwrap(); // reserved
    writer.write_all(&[0x00, 0x00]).unwrap(); // reserved
    writer.write_all(&[0x36, 0x00, 0x00, 0x00]).unwrap(); // offset to pixel data
    writer.write_all(&[0x28, 0x00, 0x00, 0x00]).unwrap(); // DIB header size
    writer.write_all(&(width as i32).to_le_bytes()).unwrap(); // image width
    writer.write_all(&(height as i32).to_le_bytes()).unwrap(); // image height
    writer.write_all(&[0x01, 0x00]).unwrap(); // color planes
    writer.write_all(&[0x18, 0x00]).unwrap(); // bits per pixel
    writer.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap(); // compression method
    writer.write_all(&image_size.to_le_bytes()).unwrap(); // image size
    writer.write_all(&[0x13, 0x0b, 0x00, 0x00]).unwrap(); // horizontal resolution (72 DPI)
    writer.write_all(&[0x13, 0x0b, 0x00, 0x00]).unwrap(); // vertical resolution (72 DPI)
    writer.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap(); // color palette
    writer.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap(); // important colors

    // iterates over the complete array of pixels in reverse order
    // to account for the fact that BMP files are stored upside down
    for y in (0..height).rev() {
        for x in 0..width {
            let [r, g, b] = [
                pixels[((y * width + x) * 3) as usize],
                pixels[((y * width + x) * 3 + 1) as usize],
                pixels[((y * width + x) * 3 + 2) as usize],
            ];
            writer.write_all(&[b, g, r]).unwrap();
        }
        let padding = (4 - ((width * 3) % 4)) % 4;
        for _ in 0..padding {
            writer.write_all(&[0x00]).unwrap();
        }
    }

    Ok(())
}

/// Copies the contents of the source slice into the destination slice.
///
/// This function is optimized for performance and uses pointer-based
/// operations to copy the data as fast as possible.
pub fn copy_fast(src: &[u8], dst: &mut [u8], count: usize) {
    assert!(src.len() >= count);
    assert!(dst.len() >= count);

    unsafe {
        let src_ptr = src.as_ptr();
        let dst_ptr = dst.as_mut_ptr();
        std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, count);
    }
}

// Interleaves two arrays of bytes into a single array using
// a pointer-based approach for performance reasons.
pub fn interleave_arrays(a: &[u8], b: &[u8], output: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    assert_eq!(output.len(), a.len() + b.len());

    let len = a.len();

    unsafe {
        let mut out_ptr = output.as_mut_ptr();
        let mut a_ptr = a.as_ptr();
        let mut b_ptr = b.as_ptr();

        for _ in 0..len {
            std::ptr::write(out_ptr, *a_ptr);
            out_ptr = out_ptr.add(1);
            a_ptr = a_ptr.add(1);

            std::ptr::write(out_ptr, *b_ptr);
            out_ptr = out_ptr.add(1);
            b_ptr = b_ptr.add(1);
        }
    }
}

/// Flips a 2D array of pixels vertically, in place.
///
/// This function is optimized for performance and uses pointer-based
/// operations to flip the pixels as fast as possible.
pub fn flip_vertical(pixels: &[u8], width: usize, height: usize, channels: usize) -> Vec<u8> {
    let row_len = width * channels;
    let mut flipped = vec![0u8; pixels.len()];
    for y in 0..height {
        let src = &pixels[y * row_len..(y + 1) * row_len];
        let dst = &mut flipped[(height - 1 - y) * row_len..(height - y) * row_len];
        dst.copy_from_slice(src);
    }
    flipped
}

#[cfg(not(feature = "wasm"))]
pub fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn timestamp() -> u64 {
    use js_sys::Date;

    (Date::now() / 1000.0) as u64
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{read, remove_file},
        path::Path,
    };

    use super::{capitalize, replace_ext, save_bmp};

    #[test]
    fn test_change_extension() {
        let new_path = replace_ext("/path/to/file.txt", "dat").unwrap();
        assert_eq!(
            new_path,
            Path::new("/path/to").join("file.dat").to_str().unwrap()
        );

        let new_path = replace_ext("/path/to/file.with.multiple.dots.txt", "dat").unwrap();
        assert_eq!(
            new_path,
            Path::new("/path/to")
                .join("file.with.multiple.dots.dat")
                .to_str()
                .unwrap()
        );

        let new_path = replace_ext("/path/to/file.without.extension", "dat").unwrap();
        assert_eq!(
            new_path,
            Path::new("/path/to")
                .join("file.without.dat")
                .to_str()
                .unwrap()
        );

        let new_path = replace_ext("/path/to/directory/", "dat");
        assert_eq!(new_path, None);
    }

    #[test]
    fn test_capitalize_empty_string() {
        let result = capitalize("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_capitalize_single_character() {
        let result = capitalize("a");
        assert_eq!(result, "A");
    }

    #[test]
    fn test_capitalize_multiple_characters() {
        let result = capitalize("hello, world!");
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_bmp_le_bytes() {
        // according to the BMP file format specification, both the file size
        // and the image size fields are stored using little-endian encoding.
        let path = temp_dir().join("boytacean_le_test.bmp");
        save_bmp(path.to_str().unwrap(), &[255, 0, 0], 1, 1).expect("Failed to save BMP file");
        let data: Vec<u8> = read(&path).unwrap();
        assert_eq!(&data[2..6], &(58u32).to_le_bytes());
        assert_eq!(&data[34..38], &(4u32).to_le_bytes());
        remove_file(path).unwrap();
    }

    #[test]
    fn test_bmp_file_structure() {
        // Creates a 2x2 image and verifies that the BMP header follows the
        // expected structure as defined in the specification.
        let path = temp_dir().join("boytacean_spec_test.bmp");
        let pixels = [
            255, 0, 0, // red
            0, 255, 0, // green
            0, 0, 255, // blue
            255, 255, 0, // yellow
        ];
        save_bmp(path.to_str().unwrap(), &pixels, 2, 2).expect("Failed to save BMP file");
        let data = read(&path).unwrap();

        // header checks
        assert_eq!(&data[0..2], b"BM");
        assert_eq!(&data[2..6], &(70u32).to_le_bytes());
        assert_eq!(&data[10..14], &(54u32).to_le_bytes());
        assert_eq!(&data[14..18], &(40u32).to_le_bytes());
        assert_eq!(&data[18..22], &(2i32).to_le_bytes());
        assert_eq!(&data[22..26], &(2i32).to_le_bytes());
        assert_eq!(&data[26..28], &(1u16).to_le_bytes());
        assert_eq!(&data[28..30], &(24u16).to_le_bytes());
        assert_eq!(&data[34..38], &(16u32).to_le_bytes());

        remove_file(path).unwrap();
    }
}
