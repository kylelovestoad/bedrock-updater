
use std::{
    error::Error, 
    fmt::{Debug, Display},
};

#[derive(Debug)] 
#[allow(dead_code)]
pub enum BedrockUpdaterError {
    RequestError(reqwest::Error),
}

impl Display for BedrockUpdaterError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for BedrockUpdaterError {
    fn description(&self) -> &str {
        match self {
            Self::RequestError(_) => "request failed"
        }
    }
}

impl From<reqwest::Error> for BedrockUpdaterError {
    fn from(err: reqwest::Error) -> BedrockUpdaterError {
        Self::RequestError(err)
    }
}