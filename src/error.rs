
use std::{
    error::Error, 
    fmt::{Debug, Display}, 
    io::Error as IOError
};

use reqwest::header::InvalidHeaderValue;
use url::ParseError;

use fantoccini::error::{CmdError, NewSessionError};

#[derive(Debug)] 
#[allow(dead_code)]
pub enum BedrockUpdaterError {
    ConnectionError(NewSessionError),
    RustlsError(IOError),
    CommandFalureError(CmdError),
    RequestError(reqwest::Error),
    UrlConversionError(ParseError),
    InvalidCookieError(InvalidHeaderValue),
    DownloadError,
}

impl Display for BedrockUpdaterError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for BedrockUpdaterError {
    fn description(&self) -> &str {
        match self {
            Self::ConnectionError(_) => "failed to connect to webdriver",
            Self::RustlsError(_) => "rustls client failed to be created",
            Self::CommandFalureError(_) => "invalid webdriver command",
            Self::RequestError(_) => "request failed",
            Self::UrlConversionError(_) => "failed to parse url",
            Self::InvalidCookieError(_) => "invalid cookie header",
            Self::DownloadError => "failed to download minecraft bedrock server",
        }
    }
}

impl From<NewSessionError> for BedrockUpdaterError {
    fn from(err: NewSessionError) -> BedrockUpdaterError {
        Self::ConnectionError(err)
    }
}

impl From<std::io::Error> for BedrockUpdaterError {
    fn from(err: std::io::Error) -> BedrockUpdaterError {
        Self::RustlsError(err)
    }
}

impl From<CmdError> for BedrockUpdaterError {
    fn from(err: CmdError) -> BedrockUpdaterError {
        Self::CommandFalureError(err)
    }
}

impl From<reqwest::Error> for BedrockUpdaterError {
    fn from(err: reqwest::Error) -> BedrockUpdaterError {
        Self::RequestError(err)
    }
}

impl From<ParseError> for BedrockUpdaterError {
    fn from(err: ParseError) -> BedrockUpdaterError {
        Self::UrlConversionError(err)
    }
}

impl From<InvalidHeaderValue> for BedrockUpdaterError {
    fn from(err: InvalidHeaderValue) -> BedrockUpdaterError {
        Self::InvalidCookieError(err)
    }
}