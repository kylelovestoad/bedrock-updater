use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION,
};

use crate::error::BedrockUpdaterError;

mod error;

#[tokio::main]
async fn main() -> Result<(), BedrockUpdaterError> {
    env_logger::init();

    let bedrock_server_page = "https://www.minecraft.net/en-us/download/server/bedrock";

    let reqwest_client = reqwest::ClientBuilder::new()
        .build()?;

    let _html = reqwest_client
        .get(bedrock_server_page)
        .header(ACCEPT, "text/html")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header(ACCEPT_ENCODING, "gzip, deflate, br, zstd")
        .header(CONNECTION, "keep-alive")
        .send()
        .await?
        .text()
        .await?;

    print!("{_html:?}");

    Ok(())
}
