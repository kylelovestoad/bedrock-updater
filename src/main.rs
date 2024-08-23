use fantoccini::{
    elements::{self, Element}, Client, ClientBuilder, Locator
};
use reqwest::{cookie::{self, Cookie, CookieStore, Jar}, header::{HeaderValue, InvalidHeaderValue}};
use url::Url;

use crate::error::BedrockUpdaterError;

// use log::{error, info};

mod error;



async fn create_webdriver_client(
    running_url: &str
) -> Result<Client, BedrockUpdaterError> {
    Ok(ClientBuilder::rustls()?
        .connect(running_url)
        .await?)
}

#[tokio::main]
async fn main() -> Result<(), BedrockUpdaterError> {

    env_logger::init();

    let data_platform = "serverBedrockLinux";

    let css_selector = format!("a.downloadlink[data-platform={data_platform}]");
    
    let webdriver_client = create_webdriver_client("http://localhost:4444")
        .await?;
    
    let bedrock_server_page_str = "https://www.minecraft.net/en-us/download/server/bedrock";

    let bedrock_server_page_url = &bedrock_server_page_str.parse::<Url>()?;

    webdriver_client.goto(bedrock_server_page_str).await?;

    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

    let website_cookies = webdriver_client.get_all_cookies()
        .await?;

    let cookie_headers = website_cookies.iter()
        .map(|cookie| HeaderValue::from_str(&cookie.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let cookie_store = Jar::default();

    cookie_store.set_cookies(&mut cookie_headers.iter(), bedrock_server_page_url);
    
    let element: Element = webdriver_client.find(Locator::Css(&css_selector)).await?;
    
    let possible_download_url = element.attr("href")
    .await?
    .ok_or(BedrockUpdaterError::DownloadError)?;

    webdriver_client.close().await?;

    let reqwest_client = reqwest::ClientBuilder::new()
        .user_agent(user_agent)
        .cookie_provider(cookie_store.into())
        .build()?;

    let _html = reqwest_client.get(bedrock_server_page_url.to_owned())
        .send()
        .await?
        .text()
        .await?;

    print!("{_html:?}");

    Ok(())
}