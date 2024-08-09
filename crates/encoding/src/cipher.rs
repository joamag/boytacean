use boytacean_common::error::Error;

pub trait Cipher {
    type EncryptOptions;
    type DecryptOptions;

    fn encrypt(data: &mut [u8], key: &[u8], options: &Self::EncryptOptions) -> Result<(), Error>;
    fn decrypt(data: &mut [u8], key: &[u8], options: &Self::DecryptOptions) -> Result<(), Error>;
}
