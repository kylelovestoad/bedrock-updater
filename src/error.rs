use std::{str::Utf8Error, string::FromUtf8Error};

use tracing::subscriber::SetGlobalDefaultError;
use url::ParseError;

use scraper::error::SelectorErrorKind;

#[derive(thiserror::Error, Debug)] 
#[allow(dead_code)]
pub enum BedrockUpdaterError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error(transparent)]
    SelectorParseError(#[from] SelectorErrorKind<'static>),
    #[error("no download element found")]
    NoDownloadElement,
    #[error("too many download elements found, this probably means the page changed")]
    TooManyDownloadElements,
    #[error("no downloadlink attribute")]
    NoDownloadLinkAttr,
    #[error("no href attribute found, or invalid url")]
    NotFileUrl(()),
    #[error(transparent)]
    CannotParseUrl(#[from] ParseError),
    #[error("file name terminates in ..")]
    NoFileName,
    #[error(transparent)]
    PatternError(#[from] regex::Error),
    #[error("could not find version string in filename")]
    NoVersionString,
    #[error("could not parse version number")]
    UnparseableVersion,
    #[error("file not found")]
    FileNotFound(#[from] std::io::Error),
    #[error("could not join path")]
    PathJoinError,
    #[error("string is not valid utf-8")]
    Utf8Error(#[from] FromUtf8Error),
    #[/* TODO */error("unable to find version in file, use")]
    NoCurrentVersion,
    #[error("broken symlink to server path")]
    BrokenServerPathSymlink,
    #[error("setting global default tracing subscriber failed")]
    GlobalSubscriberFailed(#[from] SetGlobalDefaultError)
    
}