use std::{
    collections::HashSet,
    fs,
    hash::Hash,
    io::Cursor,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use fs_extra::dir::CopyOptions;
use regex::Regex;
use reqwest::{
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION},
    Client, RequestBuilder, Url,
};
use scraper::{Html, Selector};
use tracing::{debug, info, info_span, trace};
use version_compare::Version;

use std::fmt::Debug;

use crate::error::BedrockUpdaterError;

use crate::error::Result;

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

macro_rules! hashset {
    ($($val:expr),* ) => {{
        let mut set = HashSet::new();
        $(
            set.insert($val);
        )*
        set
    }};
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
pub trait ElseErr {
    fn else_err<E>(self, err: E) -> std::result::Result<(), E>;
}

/// False values will return an error
impl ElseErr for bool {
    fn else_err<E>(self, err: E) -> std::result::Result<(), E> {
        match self {
            true => Ok(()),
            false => Err(err),
        }
    }
}

/// Gets the download link from the minecraft bedrock server download page
/// This function's selector should be updated as the document changes
#[tracing::instrument]
pub async fn get_latest_download_link<'a>(document: &Html) -> Result<Url> {
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
async fn get_latest_version(file_name: &str) -> Result<&str> {
    info!("Getting latest version");

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
) -> Result<&'a str>
where
    T: AsRef<Path> + Debug,
{
    info!("Getting current version");
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
pub async fn get_versions<'a, T>(
    download_link_file: &'a str,
    version_path: T,
    contents: Option<&'a str>,
    set_first_version: Option<&'a str>,
) -> Result<(Version<'a>, Version<'a>)>
where
    T: AsRef<Path> + Debug + 'a,
{

    info!("Getting versions");
    let latest_version_string = get_latest_version(download_link_file);

    let current_version_string = get_current_version(version_path, contents, set_first_version);

    let current_version = Version::from(current_version_string.await?)
        .ok_or(BedrockUpdaterError::UnparseableVersion)?;
    let latest_version = Version::from(latest_version_string.await?)
        .ok_or(BedrockUpdaterError::UnparseableVersion)?;

    Ok((current_version, latest_version))
}

#[tracing::instrument(skip_all)]
async fn install_server<'a, T, U>(
    bedrock_server_zip: &Bytes,
    server_dir: T,
    update_dir: U,
    version_path: T,
    new_version: &'a Version<'a>,
    blacklist: &HashSet<PathBuf>,
) -> Result<()>
where
    T: AsRef<Path> + Debug,
    U: AsRef<Path> + Hash + Eq,
{
    info!("creating updater directory");
    std::fs::create_dir_all(&update_dir)?;

    info!("extracting updated server zip");
    zip_extract::extract(Cursor::new(bedrock_server_zip), update_dir.as_ref(), true)?;

    let entries = std::fs::read_dir(&update_dir)?;

    info!("copying files");
    for entry in entries {
        let path = entry?.path();
        let file_name = Path::new(path.file_name().ok_or(BedrockUpdaterError::NoFileName)?);
        let abs_server_file_path = server_dir.as_ref().join(&file_name);
        if !blacklist.contains(file_name) || !abs_server_file_path.exists() {
            let abs_file_path = update_dir.as_ref().join(&path);
            debug!("copying {abs_file_path:?} to {abs_server_file_path:?}");
            if abs_file_path.is_file() {
                fs::copy(&abs_file_path, &abs_server_file_path)?;
            } else {
                trace!("dir");
                fs_extra::dir::create_all(&abs_server_file_path, false)?;
                fs_extra::dir::copy(
                    &abs_file_path,
                    &abs_server_file_path,
                    &CopyOptions::new().overwrite(true),
                )?;
            }
        }
    }

    fs::write(version_path, new_version.as_str())?;

    // Cleanup the update directory
    info!("cleaning up");
    std::fs::remove_dir_all(update_dir)?;

    Ok(())
}

/// Attempt to get the html of the bedrock server page from an http request
#[tracing::instrument(skip_all)]
pub async fn fetch_document(client: &Client) -> Result<Html> {
    info!("Attempting to fetch html document");
    let page_request = client.get(BEDROCK_SERVER_PAGE).add_common_headers();

    let html = page_request.send().await?.text().await?;

    let document = Html::parse_document(&html);

    Ok(document)
}

pub async fn try_update<'a, T, U>(
    client: &Client,
    current: &Version<'a>,
    latest: &Version<'a>,
    download_link: Url,
    server_dir: T,
    update_dir: U,
    version_path: T,
) -> Result<()>
where
    T: AsRef<Path> + Debug,
    U: AsRef<Path> + Hash + Eq,
{
    let version_span = info_span!("version_check");
    let version_guard = version_span.enter();
    info!("Found server version: {current}");
    info!("Found latest version: {latest}");

    // The program will only try to install the server if it is not up to date
    if current == latest {
        info!("Server is up to date");
        drop(version_guard);
    } else if current > latest {
        info!("Server is most likely a preview version, make sure you set the correct version");
        drop(version_guard);
    } else {
        info!("Server is not up to date");
        drop(version_guard);
        let install_span = info_span!("install_phase");
        let install_guard = install_span.enter();

        let update_dir = server_dir.as_ref().join(Path::new(update_dir.as_ref()));

        debug!("reading blacklist");
        let overwrite_blacklist =
            hashset!["permissions.json", "allowlist.json", "server.properties"]
                .into_iter()
                .map(|file_name| {
                    debug!("blacklisted from overwriting: {file_name:?}");
                    PathBuf::from(file_name)
                })
                .collect::<HashSet<_>>();

        let download_request = client.get(download_link);

        info!("downloading new server version");
        let bedrock_server_zip: Bytes = download_request.send().await?.bytes().await?;

        install_server(
            &bedrock_server_zip,
            server_dir,
            update_dir,
            version_path,
            &latest,
            &overwrite_blacklist,
        )
        .await?;
        drop(install_guard);
    }

    Ok(())
}
