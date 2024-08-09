use std::{
    collections::HashSet,
    convert::TryInto,
    default,
    hash::Hash,
    io::{Cursor, Read, Write},
    iter::FromIterator,
    mem::size_of,
};

use boytacean_common::error::Error;
use boytacean_hashing::crc32c::crc32c;

use crate::{
    codec::Codec,
    huffman::{decode_huffman, encode_huffman},
    rc4::{decrypt_rc4, encrypt_rc4},
    rle::{decode_rle, encode_rle},
};

pub const ZIPPY_MAGIC: &str = "ZIPY";

pub const ZIPPY_MAGIC_UINT: u32 = 0x5a495059;

pub const ZIPPY_CIPHER_TEST: &[u8; 22] = b"ZIPPY_CIPHER_SIGNATURE";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ZippyFeatures {
    Crc32,
    EncryptedRc4,
    Other,
}

impl From<ZippyFeatures> for &str {
    fn from(value: ZippyFeatures) -> Self {
        match value {
            ZippyFeatures::Crc32 => "crc32",
            ZippyFeatures::EncryptedRc4 => "encrypted_rc4",
            ZippyFeatures::Other => "other",
        }
    }
}

impl From<&ZippyFeatures> for &str {
    fn from(value: &ZippyFeatures) -> Self {
        match value {
            ZippyFeatures::Crc32 => "crc32",
            ZippyFeatures::EncryptedRc4 => "encrypted_rc4",
            ZippyFeatures::Other => "other",
        }
    }
}

impl From<u32> for ZippyFeatures {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Crc32,
            1 => Self::EncryptedRc4,
            _ => Self::Other,
        }
    }
}

impl From<&str> for ZippyFeatures {
    fn from(value: &str) -> Self {
        match value {
            "crc32" => Self::Crc32,
            "encrypted_rc4" => Self::EncryptedRc4,
            _ => Self::Other,
        }
    }
}

#[derive(Default)]
pub struct Zippy {
    name: String,
    description: String,
    features: HashSet<ZippyFeatures>,
    options: ZippyOptions,
    crc32: u32,
    data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ZippyOptions {
    crc32: bool,
    key: Option<String>,
}

impl ZippyOptions {
    pub fn new(crc32: bool, key: Option<String>) -> Self {
        Self { crc32, key }
    }
}

impl default::Default for ZippyOptions {
    fn default() -> Self {
        Self {
            crc32: true,
            key: None,
        }
    }
}

pub struct ZippyEncodeOptions {
    name: Option<String>,
    description: Option<String>,
    features: Option<Vec<ZippyFeatures>>,
    options: Option<ZippyOptions>,
}

pub struct ZippyDecodeOptions {
    options: Option<ZippyOptions>,
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
        let is_crc32 = options.crc32;
        Ok(Self {
            name,
            description,
            features: HashSet::from_iter(features.iter().cloned()),
            options,
            crc32: if is_crc32 { crc32c(data) } else { 0xffffffff },
            data: data.to_vec(),
        })
    }

    pub fn encode_data(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Cursor::new(vec![]);
        let mut encoded = encode_huffman(&encode_rle(&self.data)?)?;

        if self.has_feature(ZippyFeatures::EncryptedRc4) {
            encrypt_rc4(&mut encoded, self.key()?)?;
        }

        Self::write_u32(&mut buffer, ZIPPY_MAGIC_UINT)?;

        Self::write_string(&mut buffer, &self.name)?;
        Self::write_string(&mut buffer, &self.description)?;

        self.write_features(&mut buffer)?;

        Self::write_buffer(&mut buffer, &encoded)?;

        Ok(buffer.into_inner())
    }

    pub fn decode_data(data: &[u8], options: Option<ZippyOptions>) -> Result<Zippy, Error> {
        let options = options.unwrap_or_default();

        let mut data = Cursor::new(data);

        let magic = Self::read_u32(&mut data)?;
        if magic != ZIPPY_MAGIC_UINT {
            return Err(Error::InvalidData);
        }

        let name = Self::read_string(&mut data)?;
        let description = Self::read_string(&mut data)?;

        let mut instance = Self {
            name,
            description,
            features: HashSet::new(),
            options,
            crc32: 0xffffffff,
            data: vec![],
        };

        instance.read_features(&mut data)?;

        let mut buffer = Self::read_buffer(&mut data)?;
        if instance.has_feature(ZippyFeatures::EncryptedRc4) {
            decrypt_rc4(&mut buffer, instance.key()?)?;
        }

        let decoded = decode_rle(&decode_huffman(&buffer)?)?;
        instance.data = decoded;

        Ok(instance)
    }

    pub fn is_zippy(data: &[u8]) -> Result<bool, Error> {
        let mut data = Cursor::new(data);

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer)?;
        let magic = u32::from_le_bytes(buffer);

        Ok(magic == ZIPPY_MAGIC_UINT)
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
    fn read_buffer(data: &mut Cursor<&[u8]>) -> Result<Vec<u8>, Error> {
        let size = Self::read_u32(data)?;
        let mut payload = vec![0; size as usize];
        data.read_exact(&mut payload)?;
        Ok(payload)
    }

    #[inline(always)]
    fn read_features(&mut self, data: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let num_features = Self::read_u32(data)?;
        for _ in 0..num_features {
            let feature_str = Self::read_string(data)?;
            let feature = ZippyFeatures::from(feature_str.as_str());
            match feature {
                ZippyFeatures::Crc32 => self.read_crc32_feature(data)?,
                ZippyFeatures::EncryptedRc4 => self.read_rc4_feature(data)?,
                _ => self.read_empty_feature(data)?,
            };
            self.features.insert(feature);
        }
        Ok(())
    }

    #[inline(always)]
    fn read_crc32_feature(&mut self, data: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let payload = Self::read_buffer(data)?;
        if payload.len() != size_of::<u32>() {
            return Err(Error::InvalidData);
        }
        let payload: [u8; 4] = payload.try_into().unwrap();
        self.crc32 = u32::from_le_bytes(payload);
        Ok(())
    }

    #[inline(always)]
    fn read_rc4_feature(&mut self, data: &mut Cursor<&[u8]>) -> Result<(), Error> {
        let mut test_data = Self::read_buffer(data)?;
        decrypt_rc4(&mut test_data, self.key()?)?;
        if test_data != ZIPPY_CIPHER_TEST {
            return Err(Error::InvalidKey);
        }
        Ok(())
    }

    #[inline(always)]
    fn read_empty_feature(&mut self, data: &mut Cursor<&[u8]>) -> Result<(), Error> {
        Self::read_buffer(data)?;
        Ok(())
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
    fn write_buffer(data: &mut Cursor<Vec<u8>>, value: &[u8]) -> Result<(), Error> {
        Self::write_u32(data, value.len() as u32)?;
        data.write_all(value)?;
        Ok(())
    }

    #[inline(always)]
    fn write_features(&self, data: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        Self::write_u32(data, self.features.len() as u32)?;
        for feature in &self.features {
            match feature {
                ZippyFeatures::Crc32 => self.write_crc32_feature(data)?,
                ZippyFeatures::EncryptedRc4 => self.write_rc4_feature(data)?,
                _ => self.write_empty_feature(data, feature.into())?,
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn write_crc32_feature(&self, data: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        Self::write_string(data, ZippyFeatures::Crc32.into())?;
        Self::write_u32(data, size_of::<u32>() as u32)?;
        Self::write_u32(data, self.crc32)?;
        Ok(())
    }

    #[inline(always)]
    fn write_rc4_feature(&self, data: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        let mut test_data = ZIPPY_CIPHER_TEST.to_vec();
        encrypt_rc4(&mut test_data, self.key()?)?;
        Self::write_string(data, ZippyFeatures::EncryptedRc4.into())?;
        Self::write_buffer(data, &test_data)?;
        Ok(())
    }

    #[inline(always)]
    fn write_empty_feature(&self, data: &mut Cursor<Vec<u8>>, name: &str) -> Result<(), Error> {
        Self::write_string(data, name)?;
        Self::write_u32(data, 0)?;
        Ok(())
    }

    fn key(&self) -> Result<&[u8], Error> {
        Ok(self
            .options
            .key
            .as_ref()
            .ok_or(Error::MissingOption(String::from("key")))?
            .as_bytes())
    }
}

impl Codec for Zippy {
    type EncodeOptions = ZippyEncodeOptions;
    type DecodeOptions = ZippyDecodeOptions;

    fn encode(data: &[u8], options: &Self::EncodeOptions) -> Result<Vec<u8>, Error> {
        Self::build(
            data,
            options.name.clone().unwrap_or_default(),
            options.description.clone().unwrap_or_default(),
            options.features.clone(),
            options.options.clone(),
        )?
        .encode_data()
    }

    fn decode(data: &[u8], options: &Self::DecodeOptions) -> Result<Vec<u8>, Error> {
        Ok(Zippy::decode_data(data, options.options.clone())?
            .data()
            .to_vec())
    }
}

pub fn encode_zippy(
    data: &[u8],
    features: Option<Vec<ZippyFeatures>>,
    options: Option<ZippyOptions>,
) -> Result<Vec<u8>, Error> {
    Zippy::encode(
        data,
        &ZippyEncodeOptions {
            name: None,
            description: None,
            features,
            options,
        },
    )
}

pub fn decode_zippy(data: &[u8], options: Option<ZippyOptions>) -> Result<Vec<u8>, Error> {
    Zippy::decode(data, &ZippyDecodeOptions { options })
}

#[cfg(test)]
mod tests {
    use boytacean_common::error::Error;

    use super::{decode_zippy, encode_zippy, Zippy, ZippyFeatures, ZippyOptions};

    #[test]
    fn test_zippy_build_and_encode() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
        let encoded = zippy.encode_data().unwrap();

        let decoded = Zippy::decode_data(&encoded, None).unwrap();
        assert_eq!(decoded.name, name);
        assert_eq!(decoded.description, description);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_zippy_decode() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
        let encoded = zippy.encode_data().unwrap();

        let decoded_data = decode_zippy(&encoded, None).unwrap();
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_zippy_crc32() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(&data, name.clone(), description.clone(), None, None).unwrap();
        let encoded = zippy.encode_data().unwrap();

        let zippy = Zippy::decode_data(&encoded, None).unwrap();
        assert!(zippy.has_feature(ZippyFeatures::Crc32));
        assert!(zippy.check_crc32());
        assert_eq!(zippy.crc32(), 0x53518fab);
    }

    #[test]
    fn test_zippy_no_crc32() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(
            &data,
            name.clone(),
            description.clone(),
            None,
            Some(ZippyOptions::new(false, None)),
        )
        .unwrap();
        let encoded = zippy.encode_data().unwrap();

        let zippy = Zippy::decode_data(&encoded, None).unwrap();
        assert!(zippy.has_feature(ZippyFeatures::Crc32));
        assert!(!zippy.check_crc32());
        assert_eq!(zippy.crc32(), 0xffffffff);
    }

    #[test]
    fn test_zippy_decode_invalid() {
        let decoded_data = decode_zippy(b"invalid", None);
        assert!(decoded_data.is_err());
        assert_eq!(decoded_data.unwrap_err(), Error::InvalidData);
    }

    #[test]
    fn test_zippy_dummy_feature() {
        let data = vec![1, 2, 3, 4, 5];
        let name = String::from("Test");
        let description = String::from("Test description");

        let zippy = Zippy::build(
            &data,
            name.clone(),
            description.clone(),
            Some(vec![ZippyFeatures::Other]),
            Some(ZippyOptions::new(false, None)),
        )
        .unwrap();
        let encoded = zippy.encode_data().unwrap();

        let zippy = Zippy::decode_data(&encoded, None).unwrap();
        assert!(zippy.has_feature(ZippyFeatures::Other));
        assert!(!zippy.has_feature(ZippyFeatures::Crc32));
        assert!(!zippy.check_crc32());
        assert_eq!(zippy.crc32(), 0xffffffff);
    }

    #[test]
    fn test_zippy_encrypted() {
        let encoded = encode_zippy(
            b"test",
            Some(vec![ZippyFeatures::EncryptedRc4]),
            Some(ZippyOptions::new(false, Some(String::from("key")))),
        )
        .unwrap();
        let decoded = decode_zippy(
            &encoded,
            Some(ZippyOptions::new(false, Some(String::from("key")))),
        )
        .unwrap();
        assert_eq!(decoded, b"test");
    }

    #[test]
    fn test_zippy_wrong_key() {
        let encoded = encode_zippy(
            b"test",
            Some(vec![ZippyFeatures::EncryptedRc4]),
            Some(ZippyOptions::new(false, Some(String::from("key")))),
        )
        .unwrap();
        let decoded = decode_zippy(
            &encoded,
            Some(ZippyOptions::new(false, Some(String::from("wrong_key")))),
        );
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err(), Error::InvalidKey);
    }

    #[test]
    fn test_zippy_no_key() {
        let encoded = encode_zippy(
            b"test",
            Some(vec![ZippyFeatures::EncryptedRc4]),
            Some(ZippyOptions::new(false, Some(String::from("key")))),
        )
        .unwrap();
        let decoded = decode_zippy(&encoded, Some(ZippyOptions::new(false, None)));
        assert!(decoded.is_err());
        assert_eq!(
            decoded.unwrap_err(),
            Error::MissingOption(String::from("key"))
        );
    }
}
