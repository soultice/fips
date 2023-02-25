use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

mod client;
mod fips;
use clap::Parser;
use configuration::configuration::Configuration;

use plugin_registry::ExternalFunctions;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use utility::{log::Loggable, options::Opts};

mod backend;

#[cfg(feature = "ui")]
mod frontend;

#[cfg(feature = "logging")]
use log::LevelFilter;
#[cfg(feature = "logging")]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

type LogFunction = Box<dyn Fn(&Loggable) + Send + Sync>;

pub struct PaintLogsCallbacks(LogFunction);

#[cfg(feature = "logging")]
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
        .build("log/fips.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "logging")]
    {
        init_logging()?;
        log::info!("Starting FIPS");
        panic::set_hook({
            Box::new(|e| {
                log::error!("Panic: {}", e);
            })
        });
    }

    let cli_options = Opts::parse();

    let plugins = Arc::new(Mutex::new(ExternalFunctions::new(&cli_options.plugins)));
    let configuration = Arc::new(Mutex::new(
        Configuration::new(&cli_options.config).unwrap_or(Configuration::default()),
    ));

    let (_state, _app, logging) = {
        #[cfg(feature = "ui")]
        let (state, app, logging) =
            { frontend::setup(plugins.clone(), configuration.clone(), cli_options.clone()) };
        #[cfg(not(feature = "ui"))]
        let (state, app, logging) = {
            backend::setup()
        };
        (state, app, logging)
    };

    let addr = ([127, 0, 0, 1], cli_options.port).into();
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();

    let _rt_handle = backend::spawn_backend(&configuration, &plugins, &addr, &logging);

    #[cfg(feature = "ui")]
    {
        frontend::spawn_frontend(_app, runtime)?;
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("server is running");
        _rt_handle.await?.unwrap();
    }

    Ok(())
}
