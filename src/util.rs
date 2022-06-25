use std::{cell::RefCell, fs::File, io::Read, rc::Rc};

pub type SharedMut<T> = Rc<RefCell<T>>;

pub fn read_file(path: &str) -> Vec<u8> {
    let mut file = File::open(path).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    data
}
