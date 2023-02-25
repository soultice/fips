use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

use fips_plugin_registry::ExternalFunctions;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use clap::Parser;
use std::fs::File;

use fips_configuration::rule_collection::RuleCollection;
use fips_configuration::configuration::Configuration;
use fips_utility::{log::Loggable, options::CliOptions};

mod client;
mod fips;
mod backend;

#[cfg(feature = "logging")]
mod logging;

#[cfg(feature = "ui")]
mod frontend;

type LogFunction = Box<dyn Fn(&Loggable) + Send + Sync>;

pub struct PaintLogsCallbacks(LogFunction);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "logging")]
    {
        logging::init()?;
        log::info!("Starting FIPS");
        std::panic::set_hook({
            Box::new(|e| {
                log::error!("Panic: {}", e);
            })
        });
    }

    let cli_options = CliOptions::parse();

    if cli_options.write_schema {
        let schema = schemars::schema_for!(Vec<RuleCollection>);
        serde_json::to_writer(&File::create("fips-schema.json")?, &schema)?;
        //exit early
        return Ok(());
    };

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
