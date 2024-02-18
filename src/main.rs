use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

use clap::Parser;
use configuration::configuration::Config;
use std::fs::File;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod backend;
mod configuration;
mod fips;
mod plugin_registry;
mod terminal_ui;
mod utility;

use crate::configuration::ruleset::RuleSet;
use crate::utility::log::Loggable;
use crate::utility::options::CliOptions;

use tokio::sync::Mutex as AsyncMutex;

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

    //TODO: get rid of duplication caused by introduction of async mutex
    let async_configuration = Arc::new(AsyncMutex::new(
        Config::load(&cli_options.config).unwrap_or_default(),
    ));

    let (_state, _app, logging) = {
        #[cfg(feature = "ui")]
        let (state, app, logging) = {
            frontend::setup(
                Arc::clone(&async_configuration),
                cli_options.clone(),
            )
            .await
        };
        #[cfg(not(feature = "ui"))]
        let (state, app, logging) = { backend::setup() };
        (state, app, logging)
    };

    let addr = ([127, 0, 0, 1], cli_options.port).into();
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();

    let _rt_handle =
        backend::spawn_backend(&async_configuration, &addr, &logging);

    #[cfg(feature = "ui")]
    {
        frontend::spawn_frontend(_app, runtime).await?;
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("server is running");
        _rt_handle.await?.unwrap();
    }

    Ok(())
}
