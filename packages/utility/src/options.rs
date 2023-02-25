use clap::Parser;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Clone)]
#[clap(version = VERSION, author = "Florian Pfingstag")]
pub struct CliOptions {
    /// The directory from where to load config files
    #[clap(short, long, default_value = ".")]
    pub config: PathBuf,
    /// The directory from where to load plugins
    #[clap(long, default_value = ".")]
    pub plugins: PathBuf,
    #[clap(short, long, default_value = "8888")]
    pub port: u16,
    #[clap(long)]
    pub write_schema: bool,
}
