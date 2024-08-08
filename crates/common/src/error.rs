#![allow(clippy::uninlined_format_args)]

//! Error related data structures to be shared and used.
//!
//! This module contains the [`Error`] enum, which is used to represent
//! errors that can occur within Boytacean domain.

use std::{
    fmt::{self, Display, Formatter},
    io,
    string::FromUtf8Error,
};

/// Top level enum for error handling within Boytacean.
///
/// Most of the time, you will want to use the `CustomError` variant
/// to provide a more detailed error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidData,
    InvalidKey,
    RomSize,
    IncompatibleBootRom,
    MissingOption(String),
    IoError(String),
    InvalidParameter(String),
    CustomError(String),
}

impl Error {
    pub fn description(&self) -> String {
        match self {
            Error::InvalidData => String::from("Invalid data format"),
            Error::InvalidKey => String::from("Invalid key"),
            Error::RomSize => String::from("Invalid ROM size"),
            Error::IncompatibleBootRom => String::from("Incompatible Boot ROM"),
            Error::MissingOption(option) => format!("Missing option: {}", option),
            Error::IoError(message) => format!("IO error: {}", message),
            Error::InvalidParameter(message) => format!("Invalid parameter: {}", message),
            Error::CustomError(message) => String::from(message),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_error: FromUtf8Error) -> Self {
        Error::CustomError(String::from("From UTF8 error"))
    }
}

impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.description()
    }
}
