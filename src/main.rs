use std::{io::Cursor, path::Path};

use regex::Regex;
use reqwest::{
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION},
    Client, RequestBuilder, Url,
};
use scraper::{Html, Selector};
use tracing::info;
use tracing::{warn, warn_span};
use version_compare::Version;

use std::fmt::Debug;

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

/// Defines some common headers used for the requests to the bedrock server download page
trait CommonHeaders {
    fn add_common_headers(self) -> RequestBuilder;
}

impl CommonHeaders for RequestBuilder {
    fn add_common_headers(self) -> RequestBuilder {
        self.header(ACCEPT, "text/html")
            .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
            .header(ACCEPT_ENCODING, "gzip")
            .header(CONNECTION, "keep-alive")
    }
}

/// An idiomatic way to throw an error
trait ElseErr {
    fn else_err<E>(self, err: E) -> Result<(), E>;
}

/// False values will return an error
impl ElseErr for bool {
    fn else_err<E>(self, err: E) -> Result<(), E> {
        match self {
            true => Ok(()),
            false => Err(err),
        }
    }
}

/// Gets the download link from the minecraft bedrock server download page
/// This function's selector should be updated as the document changes
#[tracing::instrument]
async fn get_latest_download_link<'a>(document: &Html) -> Result<Url, BedrockUpdaterError> {
    let unparsed_selector = selector!();

    let download_selector = Selector::parse(&unparsed_selector)?;

    let mut select = document.select(&download_selector);

    let download_element = select
        .next()
        .ok_or(BedrockUpdaterError::NoDownloadElement)?;

    // This is to safeguard incorrect element fetching if the page changes for any reason
    // As of now the minecraft bedrock server download page should only have one download link for each "data platform"
    if select.next().is_some() {
        return Err(BedrockUpdaterError::TooManyDownloadElements);
    }

    // No href element means that the element is most likely incorrect or the page has updated
    let link = download_element
        .attr("href")
        .ok_or(BedrockUpdaterError::NoDownloadLinkAttr)?;

    Ok(Url::parse(link)?)
}

/// Gets the latest version of the server
/// This is fetched from the download link of the file, which contains the version string
#[tracing::instrument(skip_all)]
async fn get_latest_version<'a>(file_path: &'a Path) -> Result<&'a str, BedrockUpdaterError> {
    info!("getting latest version");
    let file_name = file_path
        .file_name()
        .ok_or(BedrockUpdaterError::NoFileName)?
        .to_str()
        .ok_or(BedrockUpdaterError::NoFileName)?;

    // Regex for a version string with exactly 4 parts
    // It seems unlikely that the minecraft bedrock versioning scheme will change
    // In the event that it does, this should be changed
    let pattern = Regex::new(r"\d+(\.\d+){3}")?;

    let version_str = pattern
        .find(file_name)
        .ok_or(BedrockUpdaterError::NoVersionString)?
        .as_str();

    Ok(version_str)
}

/// Gets the current version of the server
/// For now, it does not seem like there is an easy way to check this, so it will check a version file
/// For setup, the user must set the version once manually
/// As new versions are downloaded, the version file will be updated
#[tracing::instrument(skip_all)]
async fn get_current_version<'a, T>(
    file_path: T,
    contents: Option<&'a str>,
    version_to_set: Option<&'a str>,
) -> Result<&'a str, BedrockUpdaterError>
where
    T: AsRef<Path> + Debug,
{
    info!("getting current version");
    let version_res = match (version_to_set, contents) {
        (None, None) => Err(BedrockUpdaterError::NoCurrentVersion),
        (None, Some(contents)) => Ok(contents),
        (Some(version), None) | (Some(version), Some(_)) => {
            std::fs::write(&file_path, &version)?;

            Ok(version)
        }
    };

    version_res
}

/// Gets the current and latest versions in a tuple respectively
#[tracing::instrument(skip_all)]
async fn get_versions<'a, T>(
    file_path: &'a Path,
    version_path: T,
    contents: Option<&'a str>,
    set_first_version: Option<&'a str>,
) -> Result<(Version<'a>, Version<'a>), BedrockUpdaterError>
where
    T: AsRef<Path> + Debug + 'a,
{
    info!("getting versions");
    let latest_version_string = get_latest_version(file_path);

    let current_version_string = get_current_version(version_path, contents, set_first_version);

    let current_version = Version::from(current_version_string.await?)
        .ok_or(BedrockUpdaterError::UnparseableVersion)?;
    let latest_version = Version::from(latest_version_string.await?)
        .ok_or(BedrockUpdaterError::UnparseableVersion)?;

    Ok((current_version, latest_version))
}


#[tracing::instrument(skip_all)]
async fn install_server<T>(
    client: &Client,
    download_link: Url,
    update_dir: T,
) -> Result<(), BedrockUpdaterError>
where
    T: AsRef<Path>,
{
    let download_request = client.get(download_link);

    let bedrock_server_zip = download_request.send().await?.bytes().await?;

    std::fs::create_dir_all(&update_dir)?;

    zip_extract::extract(Cursor::new(bedrock_server_zip), update_dir.as_ref(), true)?;

    Ok(())
}

/// Attempt to get the html of the bedrock server page from an http request
#[tracing::instrument(skip_all)]
async fn fetch_document(client: &Client) -> Result<Html, BedrockUpdaterError> {
    let page_request = client.get(BEDROCK_SERVER_PAGE).add_common_headers();

    let html = page_request.send().await?.text().await?;

    let document = Html::parse_document(&html);

    Ok(document)
}

#[tokio::main]
async fn main() -> Result<(), BedrockUpdaterError> {
    // Start by enabling tracing
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    // Arguments passed to the program
    let args = Args::parse();

    let client = reqwest::ClientBuilder::new().build()?;

    // Need to get the html document from the bedrock download page to get the server download link
    let document = fetch_document(&client).await?;

    let download_link = get_latest_download_link(&document).await?;

    // The path part of the Url is necessary to get the filename
    // This is so to prevent version strings from being parsed in the url if they are ever added
    let file_path = Path::new(download_link.path());

    let server_path = Path::new(&args.server_path);

    server_path
        .exists()
        .else_err(BedrockUpdaterError::NoServerPath)?;

    // The version file should be inside the server directory
    let version_path = server_path.join(args.version_file);

    let contents = String::from_utf8(std::fs::read(&version_path)?).ok();

    let (current, latest) = get_versions(
        file_path,
        version_path,
        contents.as_deref(),
        args.set_first_version.as_deref(),
    )
    .await?;

    let version_span = warn_span!("version_check");
    let _guard: tracing::span::Entered = version_span.enter();
    info!("found server version: {current}");
    info!("found latest version: {latest}");

    // The program will only try to install the server if it is not up to date
    if current == latest {
        info!("server is up to date");
    } else if current > latest {
        info!("server is most likely a preview version");
    } else {
        warn!("server is not up to date");
        let update_path = server_path.join("update");
        install_server(&client, download_link, update_path).await?;
    }

    drop(_guard);

    Ok(())
}
