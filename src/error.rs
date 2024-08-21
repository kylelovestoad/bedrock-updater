
use std::{
    error::Error, 
    fmt::{Debug, Display}, 
    io::Error as IOError
};

use fantoccini::error::{CmdError, NewSessionError};

pub enum WebDriverError {
    ConnectionError(NewSessionError),
    RustlsError(IOError),
    InvalidCommandError(CmdError)
}

impl Display for WebDriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Debug for WebDriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionError(arg0) => f.debug_tuple("ConnectionError").field(arg0).finish(),
            Self::RustlsError(arg0) => f.debug_tuple("RustlsError").field(arg0).finish(),
            Self::InvalidCommandError(arg0) => f.debug_tuple("InvalidCommandError").field(arg0).finish(),
        }
    }
}

impl Error for WebDriverError {}

impl From<NewSessionError> for WebDriverError {
    fn from(err: NewSessionError) -> WebDriverError {
        WebDriverError::ConnectionError(err)
    }
}

impl From<std::io::Error> for WebDriverError {
    fn from(err: std::io::Error) -> WebDriverError {
        WebDriverError::RustlsError(err)
    }
}

impl From<CmdError> for WebDriverError {
    fn from(err: CmdError) -> WebDriverError {
        WebDriverError::InvalidCommandError(err)
    }
}