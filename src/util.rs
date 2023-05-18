use std::{cell::RefCell, fs::File, io::Read, rc::Rc};

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
