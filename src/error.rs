use std::fmt::{self, Display, Formatter};

/// Top level enum for error handling.
/// Most of the time, you will want to use the `CustomError` variant
/// to provide a more detailed error message.
#[derive(Debug)]
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
