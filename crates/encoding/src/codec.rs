use boytacean_common::error::Error;

pub trait Codec {
    type EncodeOptions;
    type DecodeOptions;

    fn encode(data: &[u8], options: &Self::EncodeOptions) -> Result<Vec<u8>, Error>;
    fn decode(data: &[u8], options: &Self::DecodeOptions) -> Result<Vec<u8>, Error>;
}
