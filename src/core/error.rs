use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub struct Error {
    message: String,
    cause: Option<Box<dyn StdError>>,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            return write!(f, "{}: {}", self.message, cause);
        }
        write!(f, "{}", self.message)
    }
}

impl StdError for Error {}
