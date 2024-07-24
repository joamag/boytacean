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
    CustomError(String),
}

impl Error {
    pub fn description(&self) -> &str {
        match self {
            Error::InvalidData => "Invalid data format",
            Error::RomSize => "Invalid ROM size",
            Error::IncompatibleBootRom => "Incompatible Boot ROM",
            Error::CustomError(message) => message,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
