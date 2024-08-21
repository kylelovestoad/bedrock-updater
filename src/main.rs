use std::result;

use fantoccini::{
    ClientBuilder, 
    Client,
};

use crate::error::WebDriverError;

use log::{error, info};

mod error;

async fn create_webdriver_client(running_url: &str) -> Result<Client, WebDriverError> {
    Ok(ClientBuilder::rustls()?
        .connect(running_url)
        .await?)
}

#[tokio::main]
async fn main() -> Result<(), WebDriverError> {

    env_logger::init();
    
    let result: Result<Client, WebDriverError> = create_webdriver_client("http://localhost:4444")
        .await;

    let client = result?;

    client.goto("https://www.minecraft.net/en-us/download/server/bedrock").await?;

    let website_cookies = client.get_all_cookies().await?;

    info!("{website_cookies:?}");

    Ok(())
}