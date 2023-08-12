use std::{
    cell::RefCell,
    fs::File,
    io::{Read, Write},
    path::Path,
    rc::Rc,
};

pub type SharedMut<T> = Rc<RefCell<T>>;

/// Reads the contents of the file at the given path into
/// a vector of bytes.
pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Err(format!("Failed to open file: {}", path)),
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    Ok(data)
}

/// Writes the given data to the file at the given path.
pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    let mut file = match File::create(path) {
        Ok(file) => file,
        Err(_) => return Err(format!("Failed to open file: {}", path)),
    };
    file.write_all(data).unwrap();
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::replace_ext;

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
}
