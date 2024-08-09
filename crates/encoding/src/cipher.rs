pub trait Cipher {
    type EncryptOptions;
    type DecryptOptions;

    fn encrypt(data: &mut [u8], key: &[u8], options: &Self::EncryptOptions);
    fn decrypt(data: &mut [u8], key: &[u8], options: &Self::DecryptOptions);
}
