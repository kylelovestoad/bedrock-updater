use std::{str::Utf8Error, string::FromUtf8Error};

use tracing::subscriber::SetGlobalDefaultError;
use url::ParseError;

use scraper::error::SelectorErrorKind;
use zip_extract::ZipExtractError;

pub(crate) type Result<T> = ::std::result::Result<T, BedrockUpdaterError>;

#[derive(thiserror::Error, Debug)]
pub enum BedrockUpdaterError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error(transparent)]
    SelectorParseError(#[from] SelectorErrorKind<'static>),
    #[error("no download element found")]
    NoDownloadElement,
    #[error("too many download elements found, this probably means the page changed")]
    TooManyDownloadElements,
    #[error("no href attribute found")]
    NoDownloadLinkAttr,
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
    #[error("string is not valid utf-8")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("string slice is not valid utf-8")]
    Utf8Error(#[from] Utf8Error),
    #[error("unable to find version in file, use --set-first-version")]
    NoCurrentVersion,
    #[error("server path does not exist")]
    NoServerPath,
    #[error("setting global default tracing subscriber failed")]
    GlobalSubscriberFailed(#[from] SetGlobalDefaultError),
    #[error("server zip extraction failed. did the download link download the correct file?")]
    ServerZipExtractFailed(#[from] ZipExtractError),
    #[error("could not copy contents of update files")]
    UpdateCopyError(#[from] fs_extra::error::Error)
}