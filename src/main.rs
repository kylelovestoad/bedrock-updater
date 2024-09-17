use crate::args::Args;

use clap::Parser;
use error::Result;
use std::path::Path;
use tracing::{error, Level};
use updater::BedrockUpdater;

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

    let update_dir = server_dir.join(&args.update_dir);
    // The version file should be inside the server directory
    let version_path = server_dir.join(&args.version_file);

    let updater = BedrockUpdater::new(
        &client,
        server_dir,
        &update_dir,
        &version_path,
        args.set_first_version.as_deref(),
    );

    loop {
        updater.run_updater()
            .await
            .unwrap_or_else(|err| error!("{err}"));
    }
}
