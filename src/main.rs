use crate::updater::ElseErr;
use crate::error::BedrockUpdaterError;
use crate::args::Args;

use clap::Parser;
use error::Result;
use std::path::Path;
use tracing::Level;

mod error;

mod args;

mod updater;

#[tokio::main]
async fn main() -> Result<()> {
    // Start by enabling tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Arguments passed to the program
    let args = Args::parse();

    let client = reqwest::ClientBuilder::new().build()?;

    let server_dir = Path::new(&args.server_dir);

    let update_dir = Path::new(&args.update_dir);

    // The version file should be inside the server directory
    let version_path = &server_dir.join(&args.version_file);

    loop {
        let document = updater::fetch_document(&client).await?;

        // The path part of the Url is necessary to get the filename
        // This is so to prevent version strings from being parsed in the url if they are ever added
        let download_link = updater::get_latest_download_link(&document).await?;

        let cloned_download_link = download_link.clone();

        server_dir
            .exists()
            .else_err(BedrockUpdaterError::NoServerPath)?;


        let contents = String::from_utf8(std::fs::read(version_path)?).ok();

        let (current, latest) = updater::get_versions(
            cloned_download_link.path(),
            version_path,
            contents.as_deref(),
            args.set_first_version.as_deref(),
        )
        .await?;

        updater::try_update(
            &client,
            &current,
            &latest,
            download_link,
            server_dir,
            update_dir,
            &version_path,
        )
        .await?;
    }
}
