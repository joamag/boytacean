use std::{cell::RefCell, fs::File, io::Read, path::Path, rc::Rc};

pub type SharedMut<T> = Rc<RefCell<T>>;

pub fn read_file(path: &str) -> Vec<u8> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => panic!("Failed to open file: {}", path),
    };
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    data
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

#[cfg(test)]
mod tests {
    use super::*;

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
