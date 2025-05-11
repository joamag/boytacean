use boytacean_common::error::Error;

use crate::codec::Codec;

pub struct Rle;

impl Codec for Rle {
    type EncodeOptions = ();
    type DecodeOptions = ();

    fn encode(data: &[u8], _options: &Self::EncodeOptions) -> Result<Vec<u8>, Error> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = Vec::new();
        let mut prev_byte = data[0];
        let mut count = 1;

        for &byte in data.iter().skip(1) {
            if count != 255 && byte == prev_byte {
                count += 1;
            } else {
                encoded.push(prev_byte);
                encoded.push(count);
                prev_byte = byte;
                count = 1;
            }
        }
        encoded.push(prev_byte);
        encoded.push(count);

        Ok(encoded)
    }

    fn decode(data: &[u8], _options: &Self::DecodeOptions) -> Result<Vec<u8>, Error> {
        let mut decoded = Vec::new();

        let mut iter = data.iter();
        while let Some(&byte) = iter.next() {
            if let Some(&count) = iter.next() {
                decoded.extend(std::iter::repeat_n(byte, count as usize));
            }
        }

        Ok(decoded)
    }
}

pub fn encode_rle(data: &[u8]) -> Result<Vec<u8>, Error> {
    Rle::encode(data, &())
}

pub fn decode_rle(data: &[u8]) -> Result<Vec<u8>, Error> {
    Rle::decode(data, &())
}
