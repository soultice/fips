use clap::Clap;
use std::alloc::System;
use std::panic;

#[global_allocator]
static ALLOCATOR: System = System;

use bytes;
extern crate strum;
#[macro_use]
extern crate strum_macros;

mod cli;
mod client;
mod configuration;
mod moxy;
mod plugin;
mod util;

use cli::{ui, App};
use client::ClientError;
use configuration::Configuration;
use plugin::ExternalFunctions;

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};

use tokio::runtime::Runtime;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::path::PathBuf;
use std::{
    io::{stdout, Write},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
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

#[derive(Clap)]
#[clap(version = "1.0", author = "Florian Pfingstag")]
struct Opts {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config.yaml")]
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

    let mut plugins = ExternalFunctions::new();
    plugins.load_plugins_from_path(&opts.plugins)?;

    let rt = Runtime::new().unwrap();

    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        plugins: Mutex::new(plugins),
        configuration: Mutex::new(Configuration::new(opts.config.clone())),
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

    rt.spawn(Server::bind(&addr).serve(make_svc));

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
                PrintInfo::PLAIN(info) => Spans::from(vec![Span::from(info.clone())]),
                PrintInfo::MOXY(x) => Spans::from(vec![
                    Span::from(x.method.to_owned()),
                    Span::from(" "),
                    Span::from("Mode: "),
                    Span::from(x.mode.to_owned()),
                    Span::from("=> "),
                    Span::from(x.response_code.to_owned()),
                    Span::from(" "),
                    Span::from("Matched Rules: "),
                    Span::from(x.matching_rules.to_owned().to_string()),
                    Span::from(" "),
                    Span::from(x.path.to_owned()),
                ]),
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
            Event::Input(event) => match event.code {
                KeyCode::Esc => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    rt.shutdown_background();
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('r') => {
                    *state.configuration.lock().unwrap() = Configuration::new(opts.config.clone());
                    state
                        .messages
                        .lock()
                        .unwrap()
                        .push(PrintInfo::PLAIN(String::from("Config file reloaded")))
                }
                KeyCode::Char('c') => {
                    *state.messages.lock().unwrap() = Vec::new();
                }
                KeyCode::Char(_c) => {}
                KeyCode::BackTab => app.on_left(),
                KeyCode::Tab => app.on_right(),
                KeyCode::Up => app.on_up(),
                KeyCode::Down => app.on_down(),
                _ => {}
            },
            Event::Tick => {
                app.on_tick();
            }
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
