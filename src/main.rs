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
    Body, Request, Server,
};
use plugin::ExternalFunctions;
use std::{
    io::{stdout, Write},
    panic,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::runtime::{Handle, Runtime};
use tui::{backend::CrosstermBackend, Terminal};

use debug::{PrintInfo, TrafficInfo};
use std::net::SocketAddr;
use tokio::task::JoinHandle;

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

impl State {
    pub fn add_traffic_info(&self, traffic_info: TrafficInfo) -> Result<(), MainError> {
        if let Ok(mut traffic) = self.traffic_info.lock() {
            traffic.insert(0, traffic_info);
            if traffic.len() > 20 {
                traffic.pop();
            }
        }
        Ok(())
    }

    pub fn add_message(&self, message: PrintInfo) -> Result<(), MainError> {
        if let Ok(mut messages) = self.messages.lock() {
            messages.insert(0, message);
            if messages.len() > 200 {
                messages.pop();
            }
        }
        Ok(())
    }
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
    #[clap(short, long, default_value = ".")]
    config: PathBuf,
    /// The directory from where to load plugins
    #[clap(long, default_value = ".")]
    plugins: PathBuf,
    #[clap(short, long, default_value = "8888")]
    port: u16,
    #[clap(long)]
    headless: bool,
}

fn spawn_server(state: &Arc<State>, addr: &SocketAddr) -> JoinHandle<hyper::Result<()>> {
    let capture_state = Arc::clone(state);
    let make_svc = make_service_fn(move |_| {
        let inner_capture = Arc::clone(&capture_state);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let route_capture = Arc::clone(&inner_capture);
                async move { moxy::routes(req, route_capture).await }
            }))
        }
    });

    let handle = tokio::spawn(Server::bind(addr).serve(make_svc));
    handle
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let plugins = ExternalFunctions::new(&opts.plugins);
    let configuration = Configuration::new(&opts.config);

    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        plugins: Mutex::new(plugins),
        configuration: Mutex::new(configuration),
        traffic_info: Mutex::new(vec![]),
    });

    let mut app = App::new(true, state, opts.clone());

    let addr = ([127, 0, 0, 1], opts.port).into();
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();
    let rt_handle = spawn_server(&app.state, &addr);

    if !opts.headless {
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
            let captured_state = app.state.clone();
            Box::new(move |panic_info| {
                captured_state
                    .add_message(PrintInfo::PLAIN(panic_info.to_string()))
                    .unwrap_or_default();
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
    } else {
        println!("server is running");
        rt_handle.await?.unwrap();
    }

    Ok(())
}
