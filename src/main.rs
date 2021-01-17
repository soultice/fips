use clap::Clap;
use std::alloc::System;
use std::env;
use std::fmt::Display;

#[global_allocator]
static ALLOCATOR: System = System;

use bytes;
use serde_json;

extern crate strum;
#[macro_use]
extern crate strum_macros;

mod cli;
mod client;
mod configuration;
mod plugin;
mod util;

use cli::{ui, App};
use client::{AppClient, ClientError};
use configuration::{Configuration, Mode};
use plugin::ExternalFunctions;

use hyper::{
    header::HeaderName,
    http::HeaderValue,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};

use tokio::runtime::Runtime;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hyper::body::Buf;
use json_dotpath::DotPaths;
use std::path::PathBuf;
use std::{
    fs, io,
    io::{stdout, Read, Write},
    str::FromStr,
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

struct State {
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

async fn moxy<'r>(
    body: Body,
    parts: hyper::http::request::Parts,
    state: Arc<State>,
) -> Result<Response<Body>, MainError> {
    let capture = Arc::clone(&state);
    //let mut cfg: Configuration = Configuration::new(PathBuf::from("./config.yaml"));
    let method = &parts.method;
    let uri = &parts.uri;
    //let matches = cfg.matching_rules(&uri);
    let matches = state.configuration.lock().unwrap().matching_rules(&uri);

    let (mut returned_response, mode) = match matches.len() {
        0 => {
            let mut response = Response::new(Body::from("no matching rule found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            (response, Mode::PROXY)
        }
        _ => {
            //let mut first_matched_rule = cfg.clone_collection(matches[0]);
            let mut first_matched_rule = state
                .configuration
                .lock()
                .unwrap()
                .clone_collection(matches[0]);
            //let mut test = cfg.clone_collection(matches[0]);
            //println!("{:?}", test);
            let mode: Mode = first_matched_rule.mode();

            let mut returned_response = match mode {
                Mode::PROXY | Mode::MOXY => {
                    let uri = &first_matched_rule.forward_url(&uri);

                    let body_str = hyper::body::aggregate(body).await?;
                    let mut buffer = String::new();
                    body_str.reader().read_to_string(&mut buffer)?;

                    let mut client = AppClient {
                        uri,
                        method,
                        headers: first_matched_rule.forward_headers.clone(),
                        body: buffer,
                        parts: &parts,
                    };

                    let (client_parts, mut resp_json) = client.response().await?;

                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());

                    if let Some(rules) = &first_matched_rule.rules {
                        for rule in rules {
                            resp_json.dot_set(&rule.path, rule.item.clone())?;
                        }
                    }

                    let final_response_string = serde_json::to_string(&resp_json)?;
                    let returned_response = Response::from_parts(
                        client_parts,
                        Body::from(final_response_string.clone()),
                    );
                    returned_response
                }
                _ => {
                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());
                    let body = Body::from(serde_json::to_string(
                        &first_matched_rule.rules.as_ref().unwrap()[0].item,
                    )?);
                    let returned_response = Response::new(body);
                    returned_response
                }
            };

            if let Some(backward_headers) = &first_matched_rule.backward_headers {
                let mut header_buffer: Vec<(HeaderName, HeaderValue)> = Vec::new();
                for header_name in backward_headers {
                    let header = HeaderName::from_str("foo")?; //HeaderName::from_str(&header_name)?;
                    let header_value = returned_response
                        .headers()
                        .get(header_name)
                        .unwrap()
                        .clone();
                    header_buffer.push((header, header_value));
                }
                returned_response.headers_mut().clear();
                for header_tup in header_buffer {
                    returned_response
                        .headers_mut()
                        .insert(header_tup.0, header_tup.1);
                }
            }

            if let Some(response_status) = &first_matched_rule.response_status {
                *returned_response.status_mut() = StatusCode::from_u16(*response_status)?
            }
            (returned_response, mode)
        }
    };

    state
        .messages
        .lock()
        .unwrap()
        .push(PrintInfo::MOXY(MoxyInfo {
            method: method.to_string(),
            path: uri.to_string(),
            mode: mode.to_string(),
            matching_rules: matches.len(),
            response_code: returned_response.status().to_string(),
        }));

    returned_response
        .headers_mut()
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    returned_response.headers_mut().insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );

    Ok(returned_response)
}

async fn routes(req: Request<Body>, state: Arc<State>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::from(""))),

        (&Method::OPTIONS, _) => {
            let mut new_response = Response::new(Body::from(""));
            new_response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            new_response.headers_mut().insert(
                "Access-Control-Allow-Headers",
                HeaderValue::from_static("*"),
            );
            new_response.headers_mut().insert(
                "Access-Control-Allow-Methods",
                HeaderValue::from_static("*"),
            );
            Ok(new_response)
        }

        _ => {
            let (parts, body) = req.into_parts();
            let resp: Response<Body> = moxy(body, parts, state).await.unwrap();
            Ok(resp)
        }
    }
}

#[derive(Clap)]
#[clap(version = "1.0", author = "Florian Pfingstag")]
struct Opts {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "./config.yaml")]
    config: PathBuf,
    #[clap(short, long, default_value = "8888")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let mut functions = ExternalFunctions::new();

    let mut entries: Vec<_> = fs::read_dir("./plugins")?
        .filter_map(|res| {
            let path = match env::consts::OS {
                "windows" => match res {
                    Ok(e) if e.path().extension()? == "dll" => Some(e.path()),
                    _ => None,
                },
                _ => match res {
                    Ok(e) if e.path().extension()? == "so" => Some(e.path()),
                    _ => None,
                },
            };
            path
        })
        .collect();

    unsafe {
        //entries.iter()
        for path in entries.iter() {
            functions.load(&path).expect("Function loading failed");
        }
    }

    let rt = Runtime::new().unwrap();

    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        plugins: Mutex::new(functions),
        configuration: Mutex::new(Configuration::new(opts.config.clone())),
    });

    let addr = ([127, 0, 0, 1], opts.port).into();

    let capture_state = Arc::clone(&state);
    let make_svc = make_service_fn(move |_| {
        let inner_capture = Arc::clone(&capture_state);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let route_capture = Arc::clone(&inner_capture);
                async move { routes(req, route_capture).await }
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

        let loaded_plugins_info: Vec<Spans> = entries
            .iter()
            .map(|e| Spans::from(Span::from(e.to_str().unwrap())))
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
                KeyCode::Left => app.on_left(),
                KeyCode::Up => app.on_up(),
                KeyCode::Right => app.on_right(),
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
