//! Assorted utility functions and structures.

use boytacean_common::error::Error;
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Read, Write},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Shared mutable type able to be passed between types
/// allowing for circular referencing and interior mutability.
pub type SharedMut<T> = Rc<RefCell<T>>;

/// Shared thread type able to be passed between threads.
/// Significant performance overhead compared to `SharedMut`.
pub type SharedThread<T> = Arc<Mutex<T>>;

/// Reads the contents of the file at the given path into
/// a vector of bytes.
pub fn read_file(path: &str) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path)
        .map_err(|_| Error::CustomError(format!("Failed to open file: {}", path)))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| Error::CustomError(format!("Failed to read from file: {}", path)))?;
    Ok(data)
}

/// Writes the given data to the file at the given path.
pub fn write_file(path: &str, data: &[u8], flush: Option<bool>) -> Result<(), Error> {
    let mut file = File::create(path)
        .map_err(|_| Error::CustomError(format!("Failed to create file: {}", path)))?;
    file.write_all(data)
        .map_err(|_| Error::CustomError(format!("Failed to write to file: {}", path)))?;
    if flush.unwrap_or(true) {
        file.flush()
            .map_err(|_| Error::CustomError(format!("Failed to flush file: {}", path)))?;
    }
    Ok(())
}

/// Replaces the extension in the given path with the provided extension.
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

pub fn save_bmp(path: &str, pixels: &[u8], width: u32, height: u32) -> Result<(), Error> {
    let file = File::create(path)
        .map_err(|_| Error::CustomError(format!("Failed to create file: {}", path)))?;
    let mut writer = BufWriter::new(file);

    // writes the BMP file header
    let file_size = 54 + (width * height * 3);
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
    writer
        .write_all(&[(width * height * 3) as u8, 0x00, 0x00, 0x00])
        .unwrap(); // image size
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

#[cfg(not(feature = "wasm"))]
pub fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn get_timestamp() -> u64 {
    use js_sys::Date;

    (Date::now() / 1000.0) as u64
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{capitalize, replace_ext};

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
}
