use std::path::Path;

use regex::Regex;
use reqwest::{
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION},
    Url,
};
use scraper::{Html, Selector};
use tracing::{debug, debug_span, info, warn, warn_span};
use version_compare::Version;

use crate::error::BedrockUpdaterError;

use crate::args::Args;

use clap::Parser;

mod error;

mod args;

const BEDROCK_SERVER_PAGE: &str = "https://www.minecraft.net/en-us/download/server/bedrock";

macro_rules! data_platform {
    () => {
        "serverBedrockLinux"
    };
}

macro_rules! selector {
    () => {
        concat!("a.downloadlink[data-platform=", data_platform!(), "]")
    };
}

#[tracing::instrument]
async fn get_latest_download_link<'a>(document: Html) -> Result<Url, BedrockUpdaterError> {
    let unparsed_selector = selector!();

    let download_selector = Selector::parse(&unparsed_selector)?;

    let mut select = document.select(&download_selector);

    let download_element = select
        .next()
        .ok_or(BedrockUpdaterError::NoDownloadElement)?;

    /*
    This is to safeguard incorrect element fetching if the page changes for any reason
    As of now the minecraft bedrock server download page should only have one download link for each "data platform"
    */
    if select.next().is_some() {
        return Err(BedrockUpdaterError::TooManyDownloadElements);
    }

    // No href element means that the element is most likely incorrect or the page has updated
    let link = download_element
        .attr("href")
        .ok_or(BedrockUpdaterError::NoDownloadLinkAttr)?;

    Ok(Url::parse(link)?)
}

#[tracing::instrument(skip_all)]
async fn get_latest_version<'a>(file_path: &'a Path) -> Result<&'a str, BedrockUpdaterError> {
    info!("getting latest version");
    let file_name = file_path
        .file_name()
        .ok_or(BedrockUpdaterError::NoFileName)?
        .to_str()
        .ok_or(BedrockUpdaterError::NoFileName)?;

    let pattern = Regex::new(r"\d+(\.\d+){3}")?;

    let version_str = pattern
        .find(file_name)
        .ok_or(BedrockUpdaterError::NoVersionString)?
        .as_str();

    Ok(version_str)
}

#[tracing::instrument(skip_all)]
async fn get_current_version<'a, T>(
    file_path: T,
    contents: Option<&'a str>,
    version_to_set: Option<&'a str>,
) -> Result<&'a str, BedrockUpdaterError> 
where
    T: AsRef<Path> + std::fmt::Debug
{
    info!("getting current version");
    let version_res = match (version_to_set, contents) {
        (None, None) => Err(BedrockUpdaterError::NoCurrentVersion),
        (None, Some(contents)) => Ok(contents),
        (Some(version), None) | (Some(version), Some(_)) => {
            tokio::fs::write(&file_path, &version).await?;

            Ok(version)
        }
    };

    version_res
}

#[tracing::instrument(skip_all)]
async fn get_versions<'a, T>(
    file_path: &'a Path,
    version_path: T,
    contents: Option<&'a str>,
    set_first_version: Option<&'a str>,
) -> Result<(Version<'a>, Version<'a>), BedrockUpdaterError> 
where
    T: AsRef<Path> + std::fmt::Debug + 'a
{
    info!("getting versions");
    let latest_version_string = get_latest_version(file_path);

    let current_version_string = get_current_version(
        version_path, 
        contents,
        set_first_version,
    );

    let latest_version =
        Version::from(latest_version_string.await?).ok_or(BedrockUpdaterError::UnparseableVersion)?;
    let current_version =
        Version::from(current_version_string.await?).ok_or(BedrockUpdaterError::UnparseableVersion)?;

    Ok((current_version, latest_version))
}

#[tokio::main]
async fn main() -> Result<(), BedrockUpdaterError> {

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;


    let args = Args::parse();

    let client = reqwest::ClientBuilder::new()
        .build()?
        .get(BEDROCK_SERVER_PAGE)
        .header(ACCEPT, "text/html")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header(ACCEPT_ENCODING, "gzip")
        .header(CONNECTION, "keep-alive");

    let html = client.send().await?.text().await?;

    let document = Html::parse_document(&html);

    let download_link = get_latest_download_link(document).await?;

    let file_path = Path::new(download_link.path());

    let server_path = Path::new(&args.server_path);

    let server_path_symlink = tokio::fs::try_exists(server_path).await?;

    if !server_path_symlink {
        return Err(BedrockUpdaterError::BrokenServerPathSymlink)
    }

    let version_path = server_path_symlink
        .then_some(server_path.join(args.version_file))
        .ok_or(BedrockUpdaterError::PathJoinError)?;

    let contents = server_path_symlink
        .then_some(tokio::fs::read(&version_path).await?)
        .and_then(|contents| Some(String::from_utf8(contents)))
        .transpose()?;

    let (current, latest) = get_versions(
        file_path,
        version_path,
        contents.as_deref(),
        args.set_first_version.as_deref(),
    )
    .await?;

    let version_span = warn_span!("version_check");
    let _guard = version_span.enter();
    info!("found server version: {current}");
    info!("found latest version: {latest}");

    if current == latest {
        info!("server is up to date");
    } else if current > latest {
        info!("server is most likely a preview version");
    } else {
        warn!("server is not up to date");
    }

    drop(_guard);

    Ok(())
}
