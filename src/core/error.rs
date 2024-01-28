use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub status_code: u16,
    pub cause: Option<Box<dyn Debug>>,
}

impl Error {
    pub fn new(message: String, status_code: u16) -> Self {
        Error { message, status_code, cause: None }
    }

    pub fn wrap<T>(message: String, status_code: u16, cause: T) -> Self
    where
        T: Debug + 'static,
    {
        Error {
            message,
            status_code,
            cause: Some(Box::new(cause)),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {:?}", self.message, cause)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
