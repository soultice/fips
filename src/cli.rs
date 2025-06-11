use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliOptions {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.yaml")]
    pub config: PathBuf,

    /// Port to listen on
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Write schema to file
    #[arg(short, long)]
    pub write_schema: bool,

    /// Plugin directories
    #[arg(short, long)]
    pub plugins: Vec<PathBuf>,
}

pub fn parse_args() -> CliOptions {
    CliOptions::parse()
} 