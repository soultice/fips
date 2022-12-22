use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

use configuration;
use plugin_registry;
use terminal_ui;
mod client;
mod fips;

use terminal_ui::cli::{ui, App};

use bytes;
use configuration::{Configuration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use plugin_registry::ExternalFunctions;
use std::{
    io::{stdout, Write},
    panic,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tui::{backend::CrosstermBackend, Terminal};

use clap::Parser;
use std::net::SocketAddr;
use terminal_ui::cli::options::Opts;
use terminal_ui::cli::state::State;
use terminal_ui::debug::PrintInfo;
use terminal_ui::util;
use tokio::task::JoinHandle;

enum Event<I> {
    Input(I),
    Tick,
}

fn spawn_backend(state: &Arc<State>, addr: &SocketAddr) -> JoinHandle<hyper::Result<()>> {
    let capture_state = Arc::clone(state);
    let make_svc = make_service_fn(move |_| {
        let inner_capture = Arc::clone(&capture_state);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let route_capture = Arc::clone(&inner_capture);
                async move { fips::routes(req, route_capture).await }
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
    let configuration = Configuration::new(&opts.config).unwrap_or(Configuration::default());

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
    let rt_handle = spawn_backend(&app.state, &addr);

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

#[cfg(test)]
mod test {
    use super::*;
    use bytes::Buf;
    use hyper::{Client, Method, Request};
    use json_dotpath::DotPaths;

    #[tokio::test]
    async fn all_functions_work() -> Result<(), String> {
        let opts = Opts {
            config: PathBuf::from("./tests/configuration_files"),
            plugins: PathBuf::from("./plugins"),
            port: 8888,
            headless: false,
        };
        let plugins = ExternalFunctions::new(&opts.plugins);
        let configuration = Configuration::new(&opts.config);

        let state = Arc::new(State {
            messages: Mutex::new(Vec::new()),
            plugins: Mutex::new(plugins),
            configuration: Mutex::new(configuration),
            traffic_info: Mutex::new(vec![]),
        });

        let app = App::new(true, state, opts.clone());

        let addr = ([127, 0, 0, 1], opts.port).into();
        let runtime = Runtime::new().unwrap();
        let _guard = runtime.enter();
        let _rt_handle = spawn_backend(&app.state, &addr);

        let client = Client::new();

        let req = Request::builder()
            .method(Method::GET)
            .uri("http://localhost:8888/final/")
            .body(Body::default())
            .unwrap();

        let res = client.request(req).await.unwrap();

        let (_parts, body) = res.into_parts();
        let body = hyper::body::aggregate(body).await.unwrap().reader();
        let body: serde_json::Value = serde_json::from_reader(body).unwrap();

        runtime.shutdown_background();
        match body {
            serde_json::Value::Object(b) => {
                let mut pattern_has_been_replaced = false;
                if let Some(path_value) = b.dot_get::<serde_json::Value>("users.0.foo").unwrap() {
                    if path_value != serde_json::Value::String(String::from("{{Name}}")) {
                        pattern_has_been_replaced = true;
                    }
                }
                if pattern_has_been_replaced {
                    Ok(())
                } else {
                    Err(String::from("Response path does not match"))
                }
            }
            _ => Err(String::from("Response should be a Map Object")),
        }
    }
}
