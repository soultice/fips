#![feature(async_fn_in_trait)]
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

use std::sync::{Arc, Mutex};
use configuration::nconfiguration::NConfiguration;
use tokio::runtime::Runtime;
use clap::Parser;
use std::fs::File;

mod client;
mod fips;
mod backend;
mod configuration;
mod utility;
mod plugin_registry;
mod terminal_ui;

use crate::configuration::nconfiguration::RuleSet;
use crate::utility::log::Loggable;
use crate::utility::options::CliOptions;

use crate::plugin_registry::ExternalFunctions;


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
        let schema = schemars::schema_for!(Vec<RuleSet>);
        serde_json::to_writer(&File::create("fips-schema.json")?, &schema)?;
        //exit early
        return Ok(());
    };

    let plugins = Arc::new(Mutex::new(ExternalFunctions::new(&cli_options.plugins)));

    let configuration = Arc::new(Mutex::new(
        NConfiguration::load(&cli_options.nconfig).unwrap_or_default()
    ));
    log::info!("new_configuration: {:?}", configuration);

    let (_state, _app, logging) = {
        #[cfg(feature = "ui")]
        let (state, app, logging) =
            { frontend::setup(configuration.clone(), cli_options.clone()) };
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
