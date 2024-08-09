use boytacean_common::error::Error;

pub trait Hash {
    type Options;

    fn hash(data: &[u8], options: &Self::Options) -> Result<Vec<u8>, Error>;
}
