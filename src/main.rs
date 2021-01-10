#![allow(unused)]
extern crate json_patch;
extern crate serde_json;
extern crate serde_yaml;
extern crate bytes;

extern crate strum;
#[macro_use]
extern crate strum_macros;

mod cli;
mod util;
mod client;

use crate::cli::{ui, App};
use crate::client::AppClient;

use hyper::{Client, Body, Method, Request, Response, Server, StatusCode, http::{HeaderValue, response::Parts}, header::{HeaderName}, service::{make_service_fn, service_fn}};
use futures::{TryStreamExt, Stream}; // 0.3.7

use json_patch::merge;
use regex::RegexSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, path::PathBuf};

use tokio::runtime::Runtime;

use hyper::body::Buf;
use argh::FromArgs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    str::FromStr,
    io::{stdout, Write, Read},
    ops::Deref,
    borrow::Borrow,
    sync::{atomic::Ordering, mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    text::{Span, Spans},
    Terminal,
};
use json_dotpath::DotPaths;
use fake::{locales::*, faker::name::raw::*, Fake, Faker};
use std::alloc::{handle_alloc_error};

enum Event<I> {
    Input(I),
    Tick,
}

/// Crossterm cli
#[derive(Debug, FromArgs)]
struct Cli {
    /// time in ms between two ticks.
    #[argh(option, default = "250")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    #[argh(option, default = "true")]
    enhanced_graphics: bool,
}

struct Note {
    text: String,
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

#[derive(Debug, Display)]
enum Mode {
    PROXY,
    MOXY,
    MOCK,
}

#[derive(Serialize, Deserialize, Debug)]
struct RuleCollection {
    path: String,
    forwardUri: Option<String>,
    forwardHeaders: Option<Vec<String>>,
    backwardHeaders: Option<Vec<String>>,
    rules: Option<Vec<Rule>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    path: String,
    item: Value,
}

fn get_config() -> Result<Vec<RuleCollection>, Box<dyn Error>> {
    let f = std::fs::File::open("config.yaml")?;
    let d: Vec<RuleCollection> = serde_yaml::from_reader(f)?;
    Ok(d)
}

#[derive(Debug)]
enum HeadersConversionError {
    Generic,
}

struct RocketInfo {
    messages: Mutex<Vec<PrintInfo>>,
}

struct PrintInfo {
    method: String,
    path: String,
    mode: String,
    matching_rules: usize,
    response_code: String,
}

fn recursive_expand(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(val) => {
            match val.as_str() {
                "{{Name}}" => {
                    *val = Name(EN).fake();
                }
                _ => {}
            }
        }
        serde_json::Value::Array(val) => {
            for i in val {
                recursive_expand(i);
            }
        }
        serde_json::Value::Object(val) => {
            for (_, i) in val {
                recursive_expand(i);
            }
        }
        _ => {}
    }
}

async fn moxy<'r>(
    body: Body,
    parts: hyper::http::request::Parts,
    info: Arc<RocketInfo>,
) -> Option<Response<Body>> {
    let mut config = get_config().unwrap();

    let method = &parts.method;
    let uri = &parts.uri;

    let mut path_regex: Vec<String> = Vec::new();
    for rule in &config {
        path_regex.push(rule.path.to_owned())
    }
    let set = RegexSet::new(&path_regex).unwrap();
    let matches: Vec<_> = set.matches(&*uri.to_string()).into_iter().collect();

    let (mut returned_response, mode) = match matches.len() {
        0 => {
            (Response::new(Body::from("no matching rule found")), Mode::PROXY)
        }
        _ => {
            let firstMatchedRule = matches[0];
            let mode: Mode = match (&config[firstMatchedRule].forwardUri, &config[firstMatchedRule].rules) {
                (Some(_), Some(_)) => Mode::MOXY,
                (None, Some(_)) => Mode::MOCK,
                _ => Mode::PROXY,
            };
            let mut returned_response = match mode {
                Mode::PROXY | Mode::MOXY => {
                    let mut url_path = String::from("");
                    if let Some(forward_url) = config[firstMatchedRule].forwardUri.clone() {
                        url_path.push_str(&forward_url);
                    }
                    url_path.push_str(&uri.to_string());

                    let body_str = hyper::body::aggregate(body).await.unwrap();
                    let mut buffer = String::new();
                    body_str.reader().read_to_string(&mut buffer);

                    let mut client = AppClient{
                        uri: &url_path,
                        method,
                        headers: config[firstMatchedRule].forwardHeaders.clone(),
                        body: buffer,
                        parts: &parts
                    };

                    let (mut client_parts, mut resp_json) = client.response().await?;

                    if let Some(rules) = &mut config[firstMatchedRule].rules {
                        for rule in rules {
                            recursive_expand(&mut rule.item);
                            resp_json.dot_set(&rule.path, rule.item.clone());
                        }
                    }
                    let final_response_string = serde_json::to_string(&resp_json).unwrap();
                    let mut returned_response = Response::from_parts(client_parts, Body::from(final_response_string.clone()));
                    returned_response
                }
                _ => {
                    let body = match &mut config[firstMatchedRule].rules {
                        Some(rules) => {
                            recursive_expand(&mut rules[0].item);
                            Body::from(serde_json::to_string(&rules[0].item).unwrap())
                        }
                        _ => Body::from("")
                    };
                    let mut returned_response = Response::new(body);
                    returned_response
                }
            };

            if let Some(backward_headers) = &config[firstMatchedRule].backwardHeaders {
                let mut header_buffer: Vec<(HeaderName, HeaderValue)> = Vec::new();
                for header_name in backward_headers {
                    let header = HeaderName::from_str(&header_name).ok()?;
                    let header_value = returned_response.headers().get(header_name).unwrap().clone();
                    header_buffer.push((header, header_value));
                }
                returned_response.headers_mut().clear();
                for header_tup in header_buffer {
                    returned_response.headers_mut().insert(header_tup.0, header_tup.1);
                }
            }
            (returned_response, mode)
        }
    };

    info.messages.lock().unwrap().push(PrintInfo {
        method: method.to_string(),
        path: uri.to_string(),
        mode: mode.to_string(),
        matching_rules: matches.len(),
        response_code: returned_response.status().to_string(),
    });


    returned_response.headers_mut().insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    returned_response.headers_mut().insert("Access-Control-Allow-Headers", HeaderValue::from_static("*"));

    Some(returned_response)
}

async fn routes(req: Request<Body>, info: Arc<RocketInfo>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::from(
            "",
        ))),

        (&Method::OPTIONS, _) => {
            let mut new_response = Response::new(Body::from(""));
            new_response.headers_mut().insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            new_response.headers_mut().insert("Access-Control-Allow-Headers", HeaderValue::from_static("*"));
            new_response.headers_mut().insert("Access-Control-Allow-Methods", HeaderValue::from_static("*"));
            Ok(new_response)
        }

        _ => {
            //let mut not_found = Response::default();
            let (parts, body) = req.into_parts();
            let resp = moxy(body, parts, info).await.unwrap();
            //<println!("outgoing {:?}", &resp);
            //*not_found.status_mut() = StatusCode::NOT_FOUND;
            //Ok(not_found)
            // println!("before return {:?}", &resp);
            Ok(resp)
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new().unwrap();

    let print_info = Arc::new(RocketInfo {
        messages: Mutex::new(Vec::new()),
    });

    let addr = ([127, 0, 0, 1], 8000).into();

    let capture_print_info = Arc::clone(&print_info);
    let make_svc = make_service_fn(move |_| {
        let inner_capture = Arc::clone(&capture_print_info);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let route_capture = Arc::clone(&inner_capture);
                async move { routes(req, route_capture).await }
            }))
        }
    });

    rt.spawn(Server::bind(&addr).serve(make_svc));

    let cli: Cli = argh::from_env();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new("Moxy", cli.enhanced_graphics);

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(cli.tick_rate);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
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
        let spans: Vec<Spans> = print_info
            .messages
            .lock()
            .unwrap()
            .iter()
            .map(|x| {
                Spans::from(vec![
                    Span::from(x.method.to_owned()),
                    Span::from(" "),
                    Span::from("Mode for this path: "),
                    Span::from(x.mode.to_owned()),
                    Span::from(" "),
                    Span::from(x.path.to_owned()),
                    Span::from(" "),
                    Span::from("Matched Rules: "),
                    Span::from(" "),
                    Span::from(x.matching_rules.to_owned().to_string()),
                    Span::from(" "),
                    Span::from("Response Code: => "),
                    Span::from(x.response_code.to_owned()),
                ])
            })
            .collect();

        terminal.draw(|f| ui::draw(f, &mut app, spans))?;
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
                KeyCode::Char(c) => {}
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
