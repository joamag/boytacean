use std::io::{Cursor, Read, Write};

use boytacean_common::error::Error;
use boytacean_hashing::crc32::crc32;

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

impl Zippy {
    pub fn build(data: &[u8], name: String, description: String) -> Result<Self, Error> {
        Ok(Self {
            name,
            description,
            crc32: crc32(&data),
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

    pub fn decode(data: &[u8]) -> Result<Zippy, Error> {
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
        self.crc32 == crc32(&self.data)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

pub fn encode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(Zippy::build(&data, String::from(""), String::from(""))?.encode()?)
}

pub fn decode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(Zippy::decode(&data)?.data().to_vec())
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{decode_zippy, Zippy};

    #[test]
    fn test_build_and_encode() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone()).unwrap();
        let encoded = zippy.encode().unwrap();

        let decoded = Zippy::decode(&encoded).unwrap();
        assert_eq!(decoded.name, name);
        assert_eq!(decoded.description, description);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_decode_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone()).unwrap();
        let encoded = zippy.encode().unwrap();

        let decoded_data = decode_zippy(&encoded).unwrap();
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_crc32_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone()).unwrap();
        let encoded = zippy.encode().unwrap();

        let zippy = Zippy::decode(&encoded).unwrap();
        assert!(zippy.check_crc32());
    }

    #[test]
    fn test_decode_invalid() {
        let decoded_data = decode_zippy(b"invalid");
        assert!(decoded_data.is_err());
        assert_eq!(decoded_data.unwrap_err(), Error::InvalidData);
    }
}
