use clap::{command, Parser};

/// Updates a bedrock server continuously
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    
    /// Use this server directory
    #[arg(short, long, value_name = "DIR")]
    pub server_dir: String,

    /// Update directory relative to the server directory
    #[arg(short, long, value_name = "DIR", default_value = "update")]
    pub update_dir: String,

    /// Version path relative to the server directory
    #[arg(short, long, value_name = "FILE", default_value = "version.txt")]
    pub version_file: String,

    /// Set the version of the server, generally used for setting the initial version
    #[arg(long, value_name = "VERSION")]
    pub set_first_version: Option<String>

}