use std::{fmt::Display, result};

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Unspecified(String),
    Unimplemented(String),
    Unhandled(Box<dyn std::error::Error>),
    /// Device returned a response that we could not properly handle
    BadResponse(String),
    /// Device or driver does not support configuration/functionality
    NotSupported(String),
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unspecified(desc) => write!(f, "Unspecified error: {}", desc),
            Error::Unimplemented(desc) => write!(f, "Unimplemented: {}", desc),
            Error::Unhandled(e) => write!(f, "Unhandled error: {}", e),
            Error::BadResponse(e) => write!(f, "Bad response: {}", e),
            Error::NotSupported(e) => write!(f, "Not supported: {}", e),
        }
    }
}
