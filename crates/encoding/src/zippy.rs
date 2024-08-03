use boytacean_common::error::Error;
use boytacean_hashing::crc32;

use crate::{
    huffman::{decode_huffman, encode_huffman},
    rle::{decode_rle, encode_rle},
};

#[derive(Default)]
pub struct Zippy {
    data: Vec<u8>,
    crc32: u32,
}

impl Zippy {
    pub fn encode(data: &[u8], description: String) -> Result<Zippy, Error> {
        let crc32 = crc32(&data);
        encode_huffman(&encode_rle(data))
    }

    pub fn decode(data: &[u8]) -> Result<Vec<u8>, Error> {
        decode_zippy(data)
    }
}

pub fn encode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {

    let crc32 = crc32(&self.data);
    encode_huffman(&encode_rle(data))
}

pub fn decode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(decode_rle(&decode_huffman(data)?))
}
