use boytacean_common::error::Error;

use crate::cipher::Cipher;

pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    pub fn new(key: &[u8]) -> Self {
        let mut s: [u8; 256] = [0; 256];
        for (i, v) in s.iter_mut().enumerate() {
            *v = i as u8;
        }

        let key_len = key.len();
        if key_len > 0 {
            let mut j = 0;
            for i in 0..256 {
                j = (j + s[i] as usize + key[i % key_len] as usize) % 256;
                s.swap(i, j);
            }
        }

        Rc4 { s, i: 0, j: 0 }
    }

    pub fn process(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k =
                self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            *byte ^= k;
        }
    }
}

impl Cipher for Rc4 {
    type EncryptOptions = ();
    type DecryptOptions = ();

    fn encrypt(data: &mut [u8], key: &[u8], _options: &Self::EncryptOptions) -> Result<(), Error> {
        let mut rc4 = Rc4::new(key);
        rc4.process(data);
        Ok(())
    }

    fn decrypt(data: &mut [u8], key: &[u8], options: &Self::DecryptOptions) -> Result<(), Error> {
        Self::encrypt(data, key, options)
    }
}

pub fn encrypt_rc4(data: &mut [u8], key: &[u8]) -> Result<(), Error> {
    Rc4::encrypt(data, key, &())
}

pub fn decrypt_rc4(data: &mut [u8], key: &[u8]) -> Result<(), Error> {
    encrypt_rc4(data, key)
}

#[cfg(test)]
mod tests {
    use super::Rc4;

    #[test]
    fn test_rc4_initialization() {
        let key = b"key";
        let rc4 = Rc4::new(key);
        assert_eq!(rc4.s.len(), 256);
    }

    #[test]
    fn test_rc4_encryption_decryption() {
        let key = b"supersecretkey";
        let plaintext = b"hello world";
        let mut data = plaintext.to_vec();

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);
        assert_ne!(&data, plaintext);

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);
        assert_eq!(&data, plaintext);
    }

    #[test]
    fn test_rc4_empty_key() {
        let key = b"";
        let mut data = b"hello world".to_vec();

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn test_rc4_empty_data() {
        let key = b"supersecretkey";
        let mut data: Vec<u8> = vec![];

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);

        let mut rc4 = Rc4::new(key);
        rc4.process(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_rc4_different_keys() {
        let key1 = b"key1";
        let key2 = b"key2";
        let plaintext = b"hello world";
        let mut data1 = plaintext.to_vec();
        let mut data2 = plaintext.to_vec();

        let mut rc4 = Rc4::new(key1);
        rc4.process(&mut data1);

        let mut rc4 = Rc4::new(key2);
        rc4.process(&mut data2);
        assert_ne!(data1, data2);

        let mut rc4 = Rc4::new(key1);
        rc4.process(&mut data1);

        let mut rc4 = Rc4::new(key2);
        rc4.process(&mut data2);
        assert_eq!(data1, plaintext);
        assert_eq!(data2, plaintext);
    }
}
