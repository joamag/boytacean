//! Error related data structures to be shared and used.
//!
//! This module contains the [`Error`] enum, which is used to represent
//! errors that can occur within Boytacean domain.

use std::{
    backtrace::Backtrace,
    error,
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
    NotImplemented,
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
            Error::IncompatibleBootRom => String::from("Incompatible boot ROM"),
            Error::NotImplemented => String::from("Not implemented"),
            Error::MissingOption(option) => format!("Missing option: {option}"),
            Error::IoError(message) => format!("IO error: {message}"),
            Error::InvalidParameter(message) => format!("Invalid parameter: {message}"),
            Error::CustomError(message) => String::from(message),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::InvalidData => "Invalid data format",
            Error::InvalidKey => "Invalid key",
            Error::RomSize => "Invalid ROM size",
            Error::IncompatibleBootRom => "Incompatible boot ROM",
            Error::NotImplemented => "Not implemented",
            Error::MissingOption(_) => "Missing option",
            Error::IoError(_) => "IO error",
            Error::InvalidParameter(_) => "Invalid parameter",
            Error::CustomError(_) => "Custom error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
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

#[derive(Debug)]
pub struct TraceError {
    error: Error,
    backtrace: Backtrace,
}

impl TraceError {
    pub fn new(error: Error) -> Self {
        Self {
            error,
            backtrace: Backtrace::capture(),
        }
    }

    pub fn backtrace(&self) -> Option<&Backtrace> {
        Some(&self.backtrace)
    }
}

impl Display for TraceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error.description())
    }
}
