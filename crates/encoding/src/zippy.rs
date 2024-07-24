use boytacean_common::error::Error;

use crate::{
    huffman::{decode_huffman, encode_huffman},
    rle::{decode_rle, encode_rle},
};

pub fn encode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {
    encode_huffman(&encode_rle(data))
}

pub fn decode_zippy(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(decode_rle(&decode_huffman(data)?))
}
