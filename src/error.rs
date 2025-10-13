use std::{fmt::Display, result};

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Unspecified(String),
    Unimplemented(String),
    Unhandled(Box<dyn std::error::Error + Send + Sync>),
    IoError(std::io::Error),
    /// Device returned a response that we could not properly handle
    BadResponse(String),
    /// Device or driver does not support configuration/functionality
    NotSupported(String),
    /// Timed out during an operation
    Timeout(String),
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unspecified(desc) => write!(f, "Unspecified error: {desc}"),
            Error::Unimplemented(desc) => write!(f, "Unimplemented: {desc}"),
            Error::Unhandled(e) => write!(f, "Unhandled error: {e}"),
            Error::IoError(e) => write!(f, "IO error: {e}"),
            Error::BadResponse(e) => write!(f, "Bad response: {e}"),
            Error::NotSupported(e) => write!(f, "Not supported: {e}"),
            Error::Timeout(e) => write!(f, "Timed out: {e}"),
        }
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}
