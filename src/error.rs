use std::fmt::{self, Display, Formatter};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use wasm_bindgen::convert::*;

/// Top level enum for error handling.
/// Most of the time, you will want to use the `CustomError` variant
/// to provide a more detailed error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotFound,
    BadRequest,
    InternalServerError,
    CustomError(String),
}

impl Error {
    pub fn description(&self) -> &str {
        match self {
            Error::NotFound => "Not Found",
            Error::BadRequest => "Bad Request",
            Error::InternalServerError => "Internal Server Error",
            Error::CustomError(message) => message,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(feature = "wasm")]
impl IntoWasmAbi for Error {
    type Abi = <Vec<u8> as IntoWasmAbi>::Abi;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self.description().into_abi();
    }
}
