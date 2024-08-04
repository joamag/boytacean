use std::{
    default,
    io::{Cursor, Read, Write},
    mem::size_of,
};

use boytacean_common::error::Error;
use boytacean_hashing::crc32c::crc32c;

use crate::{
    huffman::{decode_huffman, encode_huffman},
    rle::{decode_rle, encode_rle},
};

pub const ZIPPY_MAGIC: &str = "ZIPY";

pub const ZIPPY_MAGIC_UINT: u32 = 0x5a495059;

#[derive(Default)]
pub struct Zippy {
    name: String,
    description: String,
    crc32: u32,
    data: Vec<u8>,
}

pub struct ZippyOptions {
    crc32: bool,
}

impl ZippyOptions {
    pub fn new(crc32: bool) -> Self {
        Self { crc32 }
    }
}

impl default::Default for ZippyOptions {
    fn default() -> Self {
        Self { crc32: true }
    }
}

impl Zippy {
    pub fn build(
        data: &[u8],
        name: String,
        description: String,
        options: Option<ZippyOptions>,
    ) -> Result<Self, Error> {
        let options = options.unwrap_or_default();
        Ok(Self {
            name,
            description,
            crc32: if options.crc32 {
                crc32c(data)
            } else {
                0xffffffff
            },
            data: data.to_vec(),
        })
    }

    pub fn is_zippy(data: &[u8]) -> Result<bool, Error> {
        let mut data = Cursor::new(data);

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let magic = u32::from_le_bytes(buffer);

        Ok(magic == ZIPPY_MAGIC_UINT)
    }

    pub fn decode(data: &[u8], _options: Option<ZippyOptions>) -> Result<Zippy, Error> {
        let mut data = Cursor::new(data);

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let magic = u32::from_le_bytes(buffer);
        if magic != ZIPPY_MAGIC_UINT {
            return Err(Error::InvalidData);
        }

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let name_length = u32::from_le_bytes(buffer);
        let mut buffer = vec![0; name_length as usize];
        data.read_exact(&mut buffer)?;
        let name = String::from_utf8(buffer)?;

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let description_length = u32::from_le_bytes(buffer);
        let mut buffer = vec![0; description_length as usize];
        data.read_exact(&mut buffer)?;
        let description = String::from_utf8(buffer)?;

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let crc32 = u32::from_le_bytes(buffer);

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let data_length = u32::from_le_bytes(buffer);
        let mut buffer = vec![0; data_length as usize];
        data.read_exact(&mut buffer)?;

        let decoded = decode_rle(&decode_huffman(&buffer)?);

        Ok(Zippy {
            name,
            description,
            crc32,
            data: decoded,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Cursor::new(vec![]);
        let encoded = encode_huffman(&encode_rle(&self.data))?;

        buffer.write_all(&(ZIPPY_MAGIC_UINT.to_le_bytes()))?;
        buffer.write_all(&(self.name.as_bytes().len() as u32).to_le_bytes())?;
        buffer.write_all(self.name.as_bytes())?;
        buffer.write_all(&(self.description.as_bytes().len() as u32).to_le_bytes())?;
        buffer.write_all(self.description.as_bytes())?;
        buffer.write_all(&self.crc32.to_le_bytes())?;
        buffer.write_all(&(encoded.len() as u32).to_le_bytes())?;
        buffer.write_all(&encoded)?;

        Ok(buffer.into_inner())
    }

    pub fn check_crc32(&self) -> bool {
        self.crc32 == crc32c(&self.data)
    }

    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

pub fn encode_zippy(data: &[u8], options: Option<ZippyOptions>) -> Result<Vec<u8>, Error> {
    Zippy::build(data, String::from(""), String::from(""), options)?.encode()
}

pub fn decode_zippy(data: &[u8], options: Option<ZippyOptions>) -> Result<Vec<u8>, Error> {
    Ok(Zippy::decode(data, options)?.data().to_vec())
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{decode_zippy, Zippy, ZippyOptions};

    #[test]
    fn test_build_and_encode() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None).unwrap();
        let encoded = zippy.encode().unwrap();

        let decoded = Zippy::decode(&encoded, None).unwrap();
        assert_eq!(decoded.name, name);
        assert_eq!(decoded.description, description);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_decode_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None).unwrap();
        let encoded = zippy.encode().unwrap();

        let decoded_data = decode_zippy(&encoded, None).unwrap();
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_crc32_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None).unwrap();
        let encoded = zippy.encode().unwrap();

        let zippy = Zippy::decode(&encoded, None).unwrap();
        assert!(zippy.check_crc32());
        assert_eq!(zippy.crc32(), 0x53518fab);
    }

    #[test]
    fn test_no_crc32_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(
            &data,
            name.clone(),
            description.clone(),
            Some(ZippyOptions::new(false)),
        )
        .unwrap();
        let encoded = zippy.encode().unwrap();

        let zippy = Zippy::decode(&encoded, None).unwrap();
        assert!(!zippy.check_crc32());
        assert_eq!(zippy.crc32(), 0xffffffff);
    }

    #[test]
    fn test_decode_invalid() {
        let decoded_data = decode_zippy(b"invalid", None);
        assert!(decoded_data.is_err());
        assert_eq!(decoded_data.unwrap_err(), Error::InvalidData);
    }
}
