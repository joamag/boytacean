pub trait Codec {
    type EncodeOptions;
    type DecodeOptions;

    fn encode(data: &[u8], options: &Self::EncodeOptions) -> Vec<u8>;
    fn decode(data: &[u8], options: &Self::DecodeOptions) -> Vec<u8>;
}
