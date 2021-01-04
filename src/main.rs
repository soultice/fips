#[macro_use]
extern crate json_patch;
extern crate serde_json;
extern crate serde_yaml;
extern crate bytes;

use hyper::service::{make_service_fn, service_fn};
use hyper::Client;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use futures::TryStreamExt; // 0.3.7

use json_patch::merge;
use regex::RegexSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, path::PathBuf};

use tokio::runtime::Runtime;

mod demo;
mod util;

use hyper::body::Buf;
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
use std::borrow::Borrow;
use std::ops::Deref;
use hyper::http::HeaderValue;
use std::io::Read;

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
    matching_rules: usize,
    response_code: String,
}

async fn moxy<'r>(
    body: Body,
    parts: hyper::http::request::Parts,
    info: Arc<RocketInfo>,
) -> Option<Response<Body>> {
    let config = get_config().unwrap();

    let method = parts.method;
    let uri = parts.uri;

    let url_path = format!("http://localhost:4041{}", uri.to_string());

    let client = Client::new();

    let mut client_req = hyper::Request::builder().method(method.clone()).uri(&url_path).body(body).unwrap();
    // println!("uri: {:?}, headers: {:?}", uri.to_string(), &parts.headers);
    client_req.headers_mut().insert("Authorization", parts.headers.get("Authorization").unwrap().clone());

    let client_res = client.request(client_req).await.unwrap();
    let (mut client_parts, client_body) = client_res.into_parts();
    //println!("{:?}", &client_body);

    let body = hyper::body::aggregate(client_body).await.unwrap();
    let mut buffer = String::new();
    body.reader().read_to_string(&mut buffer);
    // println!("in buffer {:?}", &buffer);
    //let mut resp_json: serde_json::Value = serde_json::from_reader(body.reader()).unwrap();
    let mut resp_json: serde_json::Value = serde_json::from_str(&buffer).unwrap();
    // println!("in resp_json {:?}", &resp_json);

    let mut path_regex: Vec<String> = Vec::new();

    for rule in config.iter() {
        path_regex.push(rule.path.to_owned())
    }

    let set = RegexSet::new(&path_regex).unwrap();

    let matches: Vec<_> = set.matches(&*uri.to_string()).into_iter().collect();

    info.messages.lock().unwrap().push(PrintInfo {
        method: method.to_string(),
        path: uri.to_string(),
        matching_rules: matches.len(),
        response_code: client_parts.status.to_string(),
    });

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

    let final_response_string = serde_json::to_string(&resp_json).unwrap();

    let mut returned_response = Response::from_parts(client_parts, Body::from(final_response_string.clone()));
    returned_response.headers_mut().insert("content-length", HeaderValue::from_str(&*final_response_string.as_bytes().len().to_string()).unwrap());
    returned_response.headers_mut().clear();
    returned_response.headers_mut().insert("Access-Control-Allow-Origin",  HeaderValue::from_static("*"));
    returned_response.headers_mut().insert("Access-Control-Allow-Headers",  HeaderValue::from_static("authorization,content-type"));
    Some(returned_response)
}

/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn echo(req: Request<Body>, info: Arc<RocketInfo>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::from(
            "",
        ))),

        (&Method::OPTIONS, _) => {
            let mut new_response = Response::new(Body::from(""));
            new_response.headers_mut().insert("Access-Control-Allow-Origin",  HeaderValue::from_static("*"));
            new_response.headers_mut().insert("Access-Control-Allow-Headers",  HeaderValue::from_static("authorization,content-type"));
            Ok(new_response)
        },

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

    let cfg = Arc::new(RocketInfo {
        messages: Mutex::new(Vec::new()),
    });

    let addr = ([127, 0, 0, 1], 8000).into();

    let foo = Arc::clone(&cfg);
    let make_svc = make_service_fn(move |_| {
        let onion1 = Arc::clone(&foo);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let onion2 = Arc::clone(&onion1);
                async move { echo(req, onion2).await }
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
