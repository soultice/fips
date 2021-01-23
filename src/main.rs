use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

#[macro_use]
extern crate strum_macros;

mod cli;
mod client;
mod configuration;
mod debug;
mod moxy;
mod plugin;
mod util;

use bytes;
use clap::Clap;
use cli::{ui, App};
use client::ClientError;
use configuration::Configuration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use plugin::ExternalFunctions;
use std::collections::HashMap;
use std::panic;
use std::{
    io::{stdout, Write},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tui::{
    backend::CrosstermBackend,
    text::{Span, Spans},
    Terminal,
};

use debug::{PrintInfo, TrafficInfo};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

enum Event<I> {
    Input(I),
    Tick,
}

pub struct State {
    messages: Mutex<Vec<PrintInfo>>,
    plugins: Mutex<ExternalFunctions>,
    configuration: Mutex<Configuration>,
    traffic_info: Mutex<Vec<TrafficInfo>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MainError {
    Other { msg: String },
}

impl From<ClientError> for MainError {
    fn from(client_error: ClientError) -> MainError {
        match client_error {
            ClientError::Other { msg } => MainError::Other { msg },
        }
    }
}

impl<S: ToString> From<S> for MainError {
    fn from(other: S) -> MainError {
        MainError::Other {
            msg: other.to_string(),
        }
    }
}

#[derive(Clap, Clone)]
#[clap(version = VERSION, author = "Florian Pfingstag")]
pub struct Opts {
    /// The directory from where to load config files
    #[clap(short, long, default_value = "./config")]
    config: PathBuf,
    /// The directory from where to load plugins
    #[clap(long, default_value = ".")]
    plugins: PathBuf,
    #[clap(short, long, default_value = "8888")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let mut plugins = ExternalFunctions::new(&opts.plugins);

    let runtime = Runtime::new().unwrap();

    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        plugins: Mutex::new(plugins),
        configuration: Mutex::new(Configuration::new(&opts.config)),
        traffic_info: Mutex::new(vec![]),
    });

    let mut app = App::new(true, state, opts.clone());

    let addr = ([127, 0, 0, 1], opts.port).into();

    let capture_state = Arc::clone(&app.state);
    let make_svc = make_service_fn(move |_| {
        let inner_capture = Arc::clone(&capture_state);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let route_capture = Arc::clone(&inner_capture);
                async move { moxy::routes(req, route_capture).await }
            }))
        }
    });

    runtime.spawn(Server::bind(&addr).serve(make_svc));

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(50);

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    panic::set_hook({
        let foo = app.state.clone();
        Box::new(move |panic_info| {
            foo.messages
                .lock()
                .unwrap()
                .push(PrintInfo::PLAIN(panic_info.to_string()))
        })
    });

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match rx.recv()? {
            Event::Input(event) => util::match_keybinds(event.code, &mut app)?,
            Event::Tick => app.on_tick()?,
        };

        if app.should_quit {
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            runtime.shutdown_background();
            terminal.show_cursor()?;
            break;
        }
    }

    Ok(())
}
