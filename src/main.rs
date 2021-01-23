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

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Debug)]
pub struct ResponseInfo {
    status: String,
    version: String,
    headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct RequestInfo {
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
}

impl<'a> From<&ResponseInfo> for Spans<'a> {
    fn from(response_info: &ResponseInfo) -> Spans<'a> {
        let mut info_vec = vec![
            Span::from(response_info.status.clone()),
            Span::from(response_info.version.clone()),
        ];
        for (k, v) in &response_info.headers {
            info_vec.push(Span::from(k.clone()));
            info_vec.push(Span::from(v.clone()));
        }
        Spans::from(info_vec)
    }
}

impl<'a> From<&RequestInfo> for Spans<'a> {
    fn from(request_info: &RequestInfo) -> Spans<'a> {
        let mut info_vec = vec![
            Span::from(request_info.method.clone()),
            Span::from(request_info.uri.clone()),
            Span::from(request_info.version.clone()),
        ];
        for (k, v) in &request_info.headers {
            info_vec.push(Span::from(k.clone()));
            info_vec.push(Span::from("\n"));
            info_vec.push(Span::from(v.clone()));
        }
        Spans::from(info_vec)
    }
}

impl From<&Request<Body>> for RequestInfo {
    fn from(request: &Request<Body>) -> RequestInfo {
        let method = String::from(request.method().clone().as_str());
        let uri = String::from(request.uri().clone().to_string());
        let version = String::from(format!("{:?}", request.version().clone()));
        let mut headers = HashMap::new();
        for (k, v) in request.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        RequestInfo {
            method,
            uri,
            version,
            headers,
        }
    }
}

impl From<&Response<Body>> for ResponseInfo {
    fn from(response: &Response<Body>) -> ResponseInfo {
        let status = String::from(response.status().clone().as_str());
        let version = String::from(format!("{:?}", response.version().clone()));
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        ResponseInfo {
            status,
            version,
            headers,
        }
    }
}

pub enum TrafficInfo {
    INCOMING_REQUEST(RequestInfo),
    OUTGOING_REQUEST(RequestInfo),
    INCOMING_RESPONSE(ResponseInfo),
    OUTGOING_RESPONSE(ResponseInfo),
}

pub struct State {
    messages: Mutex<Vec<PrintInfo>>,
    plugins: Mutex<ExternalFunctions>,
    configuration: Mutex<Configuration>,
    traffic_info: Mutex<Vec<TrafficInfo>>,
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
