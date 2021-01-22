use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

#[macro_use]
extern crate strum_macros;

mod cli;
mod client;
mod configuration;
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
    Body, Request, Server,
};
use plugin::ExternalFunctions;
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

enum Event<I> {
    Input(I),
    Tick,
}

pub struct State {
    messages: Mutex<Vec<PrintInfo>>,
    plugins: Mutex<ExternalFunctions>,
    configuration: Mutex<Configuration>,
}

enum PrintInfo {
    PLAIN(String),
    MOXY(MoxyInfo),
}

struct MoxyInfo {
    method: String,
    path: String,
    mode: String,
    matching_rules: usize,
    response_code: String,
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

impl<'a> From<&MoxyInfo> for Spans<'a> {
    fn from(moxy_info: &MoxyInfo) -> Spans<'a> {
        Spans::from(vec![
            Span::from(moxy_info.method.to_owned()),
            Span::from(" "),
            Span::from("Mode: "),
            Span::from(moxy_info.mode.to_owned()),
            Span::from("=> "),
            Span::from(moxy_info.response_code.to_owned()),
            Span::from(" "),
            Span::from("Matched Rules: "),
            Span::from(moxy_info.matching_rules.to_owned().to_string()),
            Span::from(" "),
            Span::from(moxy_info.path.to_owned()),
        ])
    }
}

#[derive(Clap)]
#[clap(version = "1.0", author = "Florian Pfingstag")]
pub struct Opts {
    /// Sets a custom config file
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
    });

    let addr = ([127, 0, 0, 1], opts.port).into();

    let capture_state = Arc::clone(&state);
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
    let title = format!(
        "Moxy──live on {} 😌, using config path: {}",
        opts.port,
        opts.config.clone().to_str().unwrap()
    );
    let mut app = App::new(&title, true);

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
        let foo = state.clone();
        Box::new(move |panic_info| {
            foo.messages
                .lock()
                .unwrap()
                .push(PrintInfo::PLAIN(panic_info.to_string()))
        })
    });

    loop {
        let main_info: Vec<Spans> = state
            .messages
            .lock()
            .unwrap()
            .iter()
            .map(|x| match x {
                PrintInfo::PLAIN(info) => Spans::from(info.clone()),
                PrintInfo::MOXY(info) => Spans::from(info),
            })
            .collect();

        let loaded_plugins_info: Vec<Spans> = state
            .plugins
            .lock()
            .unwrap()
            .keys()
            .map(|e| Spans::from(Span::from(e.clone())))
            .collect();

        terminal.draw(|f| {
            ui::draw(
                f,
                &mut app,
                main_info,
                loaded_plugins_info.clone(),
                loaded_plugins_info.clone(),
            )
        })?;

        match rx.recv()? {
            Event::Input(event) => util::match_keybinds(event.code, &mut app, &state, &opts)?,
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
