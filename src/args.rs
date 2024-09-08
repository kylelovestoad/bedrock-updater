use clap::{command, Parser};

/// Updates a bedrock server continuously
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    
    /// server directory
    #[arg(short, long, value_name = "DIR")]
    pub server_path: String,

    /// version path relative to the server directory
    #[arg(short, long, value_name = "FILE", default_value = "version.txt")]
    pub version_file: String,

    /// set the version of the server, generally used for setting the initial version
    #[arg(long, value_name = "VERSION")]
    pub set_first_version: Option<String>

}