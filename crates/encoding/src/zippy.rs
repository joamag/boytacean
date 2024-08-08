use std::{
    collections::HashSet,
    default,
    io::{Cursor, Read, Write},
    iter::FromIterator,
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ZippyFeatures {
    Crc32,
    Encrypted,
    Other,
}

impl From<u32> for ZippyFeatures {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Crc32,
            1 => Self::Encrypted,
            _ => Self::Other,
        }
    }
}

impl From<&str> for ZippyFeatures {
    fn from(value: &str) -> Self {
        match value {
            "crc32" => Self::Crc32,
            "encrypted" => Self::Encrypted,
            _ => Self::Other,
        }
    }
}

#[derive(Default)]
pub struct Zippy {
    name: String,
    description: String,
    features: HashSet<ZippyFeatures>,
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
        features: Option<Vec<ZippyFeatures>>,
        options: Option<ZippyOptions>,
    ) -> Result<Self, Error> {
        let features = features.unwrap_or(vec![ZippyFeatures::Crc32]);
        let options = options.unwrap_or_default();
        Ok(Self {
            name,
            description,
            features: HashSet::from_iter(features.iter().cloned()),
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

        let magic = Self::read_u32(&mut data)?;
        if magic != ZIPPY_MAGIC_UINT {
            return Err(Error::InvalidData);
        }

        let name = Self::read_string(&mut data)?;
        let description = Self::read_string(&mut data)?;
        let features = Self::read_features(&mut data)?;

        let crc32 = if features.contains(&ZippyFeatures::Crc32) {
            Self::read_u32(&mut data)?
        } else {
            0xffffffff
        };

        let data_length = Self::read_u32(&mut data)?;
        let mut buffer = vec![0; data_length as usize];
        data.read_exact(&mut buffer)?;

        let decoded = decode_rle(&decode_huffman(&buffer)?);

        Ok(Zippy {
            name,
            description,
            features,
            crc32,
            data: decoded,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Cursor::new(vec![]);
        let encoded = encode_huffman(&encode_rle(&self.data))?;

        Self::write_u32(&mut buffer, ZIPPY_MAGIC_UINT)?;

        Self::write_string(&mut buffer, &self.name)?;
        Self::write_string(&mut buffer, &self.description)?;
        Self::write_features(&mut buffer, &self.features)?;

        if self.has_feature(ZippyFeatures::Crc32) {
            Self::write_u32(&mut buffer, self.crc32)?;
        }

        Self::write_u32(&mut buffer, encoded.len() as u32)?;
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

    pub fn has_feature(&self, feature: ZippyFeatures) -> bool {
        self.features.contains(&feature)
    }

    #[inline(always)]
    fn read_u32(data: &mut Cursor<&[u8]>) -> Result<u32, Error> {
        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    #[inline(always)]
    fn read_string(data: &mut Cursor<&[u8]>) -> Result<String, Error> {
        let length = Self::read_u32(data)?;
        let mut buffer = vec![0; length as usize];
        data.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    #[inline(always)]
    fn read_features(data: &mut Cursor<&[u8]>) -> Result<HashSet<ZippyFeatures>, Error> {
        let mut features = HashSet::new();
        let num_features = Self::read_u32(data)?;
        for _ in 0..num_features {
            let feature = Self::read_string(data)?;
            features.insert(ZippyFeatures::from(feature.as_str()));
        }
        Ok(features)
    }

    #[inline(always)]
    fn write_u32(data: &mut Cursor<Vec<u8>>, value: u32) -> Result<(), Error> {
        data.write_all(&value.to_le_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn write_string(data: &mut Cursor<Vec<u8>>, value: &str) -> Result<(), Error> {
        Self::write_u32(data, value.len() as u32)?;
        data.write_all(value.as_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn write_features(
        data: &mut Cursor<Vec<u8>>,
        features: &HashSet<ZippyFeatures>,
    ) -> Result<(), Error> {
        Self::write_u32(data, features.len() as u32)?;
        for feature in features {
            Self::write_string(
                data,
                match feature {
                    ZippyFeatures::Crc32 => "crc32",
                    ZippyFeatures::Encrypted => "encrypted",
                    _ => "other",
                },
            )?;
        }
        Ok(())
    }
}

pub fn encode_zippy(data: &[u8], options: Option<ZippyOptions>) -> Result<Vec<u8>, Error> {
    Zippy::build(data, String::from(""), String::from(""), None, options)?.encode()
}

pub fn decode_zippy(data: &[u8], options: Option<ZippyOptions>) -> Result<Vec<u8>, Error> {
    Ok(Zippy::decode(data, options)?.data().to_vec())
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{decode_zippy, Zippy, ZippyFeatures, ZippyOptions};

    #[test]
    fn test_build_and_encode() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
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

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
        let encoded = zippy.encode().unwrap();

        let decoded_data = decode_zippy(&encoded, None).unwrap();
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_crc32_zippy() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
        let encoded = zippy.encode().unwrap();

        let zippy = Zippy::decode(&encoded, None).unwrap();
        assert!(zippy.has_feature(ZippyFeatures::Crc32));
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
            None,
            Some(ZippyOptions::new(false)),
        )
        .unwrap();
        let encoded = zippy.encode().unwrap();

        let zippy = Zippy::decode(&encoded, None).unwrap();
        assert!(zippy.has_feature(ZippyFeatures::Crc32));
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
