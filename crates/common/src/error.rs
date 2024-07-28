#![allow(clippy::uninlined_format_args)]

//! Error related data structures to be shared and used.
//!
//! This module contains the [`Error`] enum, which is used to represent
//! errors that can occur within Boytacean domain.

use std::fmt::{self, Display, Formatter};

/// Top level enum for error handling within Boytacean.
///
/// Most of the time, you will want to use the `CustomError` variant
/// to provide a more detailed error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidData,
    RomSize,
    IncompatibleBootRom,
    InvalidParameter(String),
    CustomError(String),
}

impl Error {
    pub fn description(&self) -> String {
        match self {
            Error::InvalidData => String::from("Invalid data format"),
            Error::RomSize => String::from("Invalid ROM size"),
            Error::IncompatibleBootRom => String::from("Incompatible Boot ROM"),
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
