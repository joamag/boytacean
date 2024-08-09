use crate::codec::Codec;

pub struct Rle;

impl Codec for Rle {
    type EncodeOptions = ();
    type DecodeOptions = ();

    fn encode(data: &[u8], _options: &Self::EncodeOptions) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
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

        encoded
    }

    fn decode(data: &[u8], _options: &Self::DecodeOptions) -> Vec<u8> {
        let mut decoded = Vec::new();

        let mut iter = data.iter();
        while let Some(&byte) = iter.next() {
            if let Some(&count) = iter.next() {
                decoded.extend(std::iter::repeat(byte).take(count as usize));
            }
        }

        decoded
    }
}

pub fn encode_rle(data: &[u8]) -> Vec<u8> {
    Rle::encode(data, &())
}

pub fn decode_rle(data: &[u8]) -> Vec<u8> {
    Rle::decode(data, &())
}
