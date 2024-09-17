use std::{collections::HashSet, fs, io::Cursor, path::Path};

use bytes::Bytes;
use fs_extra::dir::CopyOptions;
use regex::Regex;
use reqwest::{
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION},
    Client, RequestBuilder, Url,
};
use scraper::{Html, Selector};
use tracing::{debug, info, info_span};
use version_compare::Version;

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
trait ElseErr {
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

pub struct BedrockUpdater<'a> {
    client: &'a Client,
    server_dir: &'a Path,
    update_dir: &'a Path,
    version_path: &'a Path,
    set_first_version: Option<&'a str>,
}

impl<'a> BedrockUpdater<'a> {
    pub fn new(
        client: &'a Client,
        server_dir: &'a Path,
        update_dir: &'a Path,
        version_path: &'a Path,
        set_first_version: Option<&'a str>,
    ) -> Self {
        Self {
            client,
            server_dir,
            update_dir,
            version_path,
            set_first_version,
        }
    }

    /// Gets the download link from the minecraft bedrock server download page
    /// This function's selector should be updated as the document changes
    #[tracing::instrument]
    async fn get_latest_download_link(document: &Html) -> Result<Url> {
        let unparsed_selector = selector!();

        let download_selector = Selector::parse(&unparsed_selector)?;

        let mut select = document.select(&download_selector);

        info!("Looking for download element");
        let download_element = select
            .next()
            .ok_or(BedrockUpdaterError::NoDownloadElement)?;

        // This is to safeguard incorrect element fetching if the page changes for any reason
        // As of now the minecraft bedrock server download page should only have one download link for each "data platform"
        info!("Checking for extra download elements");
        if select.next().is_some() {
            return Err(BedrockUpdaterError::TooManyDownloadElements);
        }

        info!("No other matching download buttons found, attempting to get link from button");
        // No href element means that the element is most likely incorrect or the page has updated
        let link = download_element
            .attr("href")
            .ok_or(BedrockUpdaterError::NoDownloadLinkAttr)?;

        info!("Successfully got link from element");
        Ok(Url::parse(link)?)
    }

    /// Gets the current version of the server
    /// For now, it does not seem like there is an easy way to check this, so it will check a version file
    /// For setup, the user must set the version once manually
    /// As new versions are downloaded, the version file will be updated
    #[tracing::instrument(skip_all)]
    async fn get_current_version<'b>(&self, contents: Option<&'b str>) -> Result<&'b str>
    where
        'a: 'b,
    {
        info!("Getting current version");
        let version_res = match (self.set_first_version, contents) {
            (None, None) => Err(BedrockUpdaterError::NoCurrentVersion),
            (None, Some(contents)) => Ok(contents),
            (Some(version), None) | (Some(version), Some(_)) => {
                info!("Writing to version file");
                std::fs::write(self.version_path, &version)?;

                Ok(version)
            }
        };

        version_res
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

    /// Gets the current and latest versions in a tuple respectively
    #[tracing::instrument(skip_all)]
    async fn get_versions<'b>(
        &self,
        download_link_file: &'b str,
        contents: Option<&'b str>,
    ) -> Result<(Version<'b>, Version<'b>)>
    where
        'a: 'b,
    {
        info!("Getting versions");
        let latest_version_string = Self::get_latest_version(download_link_file);

        let current_version_string = Self::get_current_version(self, contents);

        let current_version = Version::from(current_version_string.await?)
            .ok_or(BedrockUpdaterError::UnparseableVersion)?;
        let latest_version = Version::from(latest_version_string.await?)
            .ok_or(BedrockUpdaterError::UnparseableVersion)?;

        Ok((current_version, latest_version))
    }

    /// Attempt to get the html of the bedrock server page from an http request
    #[tracing::instrument(skip_all)]
    #[tracing::instrument(skip_all)]
    async fn fetch_document(client: &Client) -> Result<Html> {
        info!("Attempting to fetch html document");
        let page_request = client.get(BEDROCK_SERVER_PAGE).add_common_headers();

        let html = page_request.send().await?.text().await?;

        let document = Html::parse_document(&html);
        info!("Found document!");

        Ok(document)
    }

    #[tracing::instrument(skip_all)]
    async fn install_server<'b>(
        &self,
        bedrock_server_zip: &'b Bytes,
        new_version: &'b Version<'b>,
        blacklist: &'b HashSet<&str>,
    ) -> Result<()> {
        info!("Creating updater directory");
        std::fs::create_dir_all(self.update_dir)?;

        info!("Extracting updated server zip");
        zip_extract::extract(Cursor::new(bedrock_server_zip), self.update_dir, true)?;

        let entries = std::fs::read_dir(self.update_dir)?;

        info!("Copying files");
        // Start by looping through each of the files in the update dir
        for entry in entries {
            let path = entry?.path();

            // file_name is taken from the path to compare to file names from the blacklist
            let file_name = path
                .file_name()
                .ok_or(BedrockUpdaterError::NoFileName)?
                .to_str()
                .ok_or(BedrockUpdaterError::NoFileName)?;

            // The destination is always the server's directory
            let destination = self.server_dir.join(&file_name);

            // Prevent overwrites of the files in the blacklist
            // Don't prevent blacklisted files from being copied from update dir if they don't exist in the server dir
            if !blacklist.contains(file_name) || !destination.exists() {
                // The source is always the update directory
                let source = self.update_dir.join(&path);
                debug!("Copying {source:?} to {destination:?}");
                if source.is_file() {
                    debug!("Copying file");
                    // When it is a file, just do a simple copy
                    fs::copy(&source, &destination)?;
                } else {
                    debug!("Copying dir");
                    // Recursive copy requires that all directories being copied to exist
                    // fs_extra copy copies inside the destination directory instead of overwriting
                    // The server directory makes more sense here
                    fs_extra::dir::copy(
                        &source,
                        self.server_dir,
                        &CopyOptions::new().overwrite(true),
                    )?;
                }
            }
        }

        // Finally, write the updated version in the version file
        // At this point, the server is completely updated
        fs::write(self.version_path, new_version.as_str())?;

        // Cleanup the update directory
        info!("Cleaning up");
        std::fs::remove_dir_all(self.update_dir)?;

        Ok(())
    }

    async fn try_update<'b>(
        &self,
        current: &Version<'b>,
        latest: &Version<'b>,
        download_link: Url,
    ) -> Result<()> {
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

            // This will eventually be turned into an option in the struct, but for now it is hardcoded
            debug!("Reading blacklist");
            let overwrite_blacklist =
                hashset!["permissions.json", "allowlist.json", "server.properties"];

            let download_request = self.client.get(download_link);

            info!("Downloading new server version");
            let bedrock_server_zip: Bytes = download_request.send().await?.bytes().await?;

            Self::install_server(self, &bedrock_server_zip, &latest, &overwrite_blacklist).await?;
            drop(install_guard);
        }

        Ok(())
    }

    pub async fn run_updater(&self) -> Result<()> {
        // First get the minecraft download page html
        let document = Self::fetch_document(self.client).await?;

        // The path part of the Url is necessary to get the filename
        // This is so to prevent version strings from being parsed in the url if they are ever added
        let download_link = Self::get_latest_download_link(&document).await?;

        // The clone is necessary
        // The function will not be able to move download_link since it gets borrowed when calling .path()
        let cloned_download_link = download_link.clone();

        self.server_dir
            .exists()
            .else_err(BedrockUpdaterError::NoServerPath)?;

        info!("Attempting to get version file version");
        let contents = std::fs::read(self.version_path)
            .map_or(None,|contents| Some(String::from_utf8(contents)))
            .transpose()?;

        let (current, latest) =
            Self::get_versions(self, cloned_download_link.path(), contents.as_deref()).await?;

        Self::try_update(self, &current, &latest, download_link).await?;

        Ok(())
    }
}
