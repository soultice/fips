#![feature(proc_macro_hygiene, decl_macro)]

#![deny(warnings)]
use futures::TryStreamExt as _;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

#[macro_use]
extern crate rocket;
extern crate json_patch;
extern crate reqwest;
extern crate serde_json;
extern crate serde_yaml;

use json_patch::merge;
use regex::RegexSet;
use reqwest::header::{HeaderName, HeaderValue};
use rocket::{
    http::{Method, Status},
    request::{FromRequest, Outcome, Request},
    tokio::runtime::Runtime,
    State,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, path::PathBuf};

mod demo;
mod util;

use crate::demo::{ui, App};
use argh::FromArgs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::str::FromStr;
use std::{
    error::Error,
    io::{stdout, Write},
    sync::{atomic::Ordering, mpsc, Arc, Mutex},
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

/// Crossterm demo
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

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    path: String,
    append: Option<RuleSpecifics>,
    prepend: Option<RuleSpecifics>,
    insert: Option<InsertRuleSpecifics>,
    merge: Option<InsertRuleSpecifics>,
    delete: Option<RuleSpecifics>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RuleSpecifics {
    items: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InsertRuleSpecifics {
    at_index: usize,
    items: Vec<Value>,
}

fn get_config() -> Result<Vec<Rule>, Box<dyn Error>> {
    let f = std::fs::File::open("config.yaml")?;
    let d: Vec<Rule> = serde_yaml::from_reader(f)?;
    Ok(d)
}

struct RocketRequestInfo {
    headers: reqwest::header::HeaderMap,
    method: Method,
}

#[derive(Debug)]
enum HeadersConversionError {
    Generic,
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for RocketRequestInfo {
    type Error = HeadersConversionError;

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let headers = request.headers();
        let mut reqw_headers = reqwest::header::HeaderMap::new();
        for header in headers.iter() {
            reqw_headers.append(
                HeaderName::try_from(&header.name().to_owned().to_string()).unwrap(),
                HeaderValue::try_from(&header.value().to_owned().to_string()).unwrap(),
            );
        }
        Outcome::Success(RocketRequestInfo {
            headers: reqw_headers,
            method: request.method(),
        })
    }
}

struct RocketInfo {
    messages: Mutex<Vec<PrintInfo>>,
}

struct PrintInfo {
    method: String,
    path: String,
}

async fn make_request(
    method: &str,
    path: &str,
    headers: reqwest::header::HeaderMap,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp: Result<reqwest::Response, reqwest::Error> = client
        .request(reqwest::Method::from_str(method).unwrap(), path)
        .headers(headers)
        .send()
        .await;
    return resp;
}

async fn moxy<'r>(
    path: PathBuf,
    rocket_headers: RocketRequestInfo,
    rocket_info: State<'r, Arc<RocketInfo>>,
) -> Option<String> {
    let config = get_config().unwrap();
    let str_path = path.to_str().unwrap();

    let mut lock = rocket_info.messages.lock().unwrap().push(PrintInfo {
        method: rocket_headers.method.as_str().to_owned(),
        path: str_path.to_owned(),
    });

    let url_path = format!("http://localhost:3000/{}", path.to_str()?);

    let resp = make_request(rocket_headers.method.as_str(), &url_path, rocket_headers.headers).await.ok()?;
    let resp_body = resp.text().await.ok()?;

    let mut resp_json: serde_json::Value = serde_json::from_str(&resp_body).ok()?;

    let mut path_regex: Vec<String> = Vec::new();

    for rule in config.iter() {
        path_regex.push(rule.path.to_owned())
    }

    let set = RegexSet::new(&path_regex).unwrap();

    let matches: Vec<_> = set.matches(str_path).into_iter().collect();

    for idx in matches.iter() {
        if config[*idx].insert.is_some() {
            match resp_json {
                serde_json::Value::Array(ref mut typ) => {
                    for x in config[*idx].insert.as_ref()?.items.iter().rev() {
                        typ.insert(config[*idx].insert.as_ref()?.at_index, x.to_owned());
                    }
                }
                _ => (),
            }
        }
        if config[*idx].append.is_some() {
            match resp_json {
                serde_json::Value::Array(ref mut typ) => {
                    typ.extend_from_slice(&config[*idx].append.as_ref()?.items)
                }
                _ => (),
            }
        }
        if config[*idx].prepend.is_some() {
            match resp_json {
                serde_json::Value::Array(ref mut typ) => {
                    for (i, x) in config[*idx].prepend.as_ref()?.items.iter().enumerate() {
                        typ.insert(i, x.to_owned());
                    }
                }
                _ => (),
            }
        }
        if config[*idx].merge.is_some() {
            match resp_json {
                serde_json::Value::Array(ref mut typ) => {
                    for x in config[*idx].merge.as_ref()?.items.iter() {
                        merge(
                            &mut typ[config[*idx].merge.as_ref()?.at_index],
                            &x.to_owned(),
                        );
                    }
                }
                _ => (),
            }
        }
    }

    Some(serde_json::to_string(&resp_json).ok()?)
}

#[get("/<path..>")]
async fn get_mock<'r>(
    path: PathBuf,
    rocket_headers: RocketRequestInfo,
    rocket_info: State<'r, Arc<RocketInfo>>,
) -> Option<String> {
    return moxy(path, rocket_headers, rocket_info).await;
}

#[post("/<path..>")]
async fn post_mock<'r>(
    path: PathBuf,
    rocket_headers: RocketRequestInfo,
    rocket_info: State<'r, Arc<RocketInfo>>,
) -> Option<String> {
    return moxy(path, rocket_headers, rocket_info).await;
}

/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => Ok(Response::new(Body::from(
            "Try POSTing data to /echo such as: `curl localhost:3000/echo -XPOST -d 'hello world'`",
        ))),

        (&Method::POST, "/echo/reversed") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await?;

            let reversed_body = whole_body.iter().rev().cloned().collect::<Vec<u8>>();
            Ok(Response::new(Body::from(reversed_body)))
        }

        _ => {
            println!("route not found {}", req.uri().path());
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new().unwrap();

    let cfg = Arc::new(RocketInfo {
        messages: Mutex::new(Vec::new()),
    });

    rt.spawn(
        rocket::ignite()
            .mount("/", routes![get_mock])
            .mount("/", routes![post_mock])
            .manage(Arc::clone(&cfg))
            .launch(),
    );

    let addr = ([127, 0, 0, 1], 3001).into();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(echo)) });

    rt.spawn(
        let server = Server::bind(&addr).serve(service);
    );


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

    let test = String::from("");
    let mut practical_note = Note { text: test };

    loop {
        let spans: Vec<Spans> = cfg
            .messages
            .lock()
            .unwrap()
            .iter()
            .map(|x| {
                Spans::from(vec![
                    Span::from(x.method.to_owned()),
                    Span::from(" "),
                    Span::from(x.path.to_owned()),
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
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char(c) => practical_note.text += &c.to_string(),
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
