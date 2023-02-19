use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

#[cfg(feature = "logging")]
use log::LevelFilter;
use log::info;
#[cfg(feature = "logging")]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use terminal_ui::debug::ResponseInfo;
#[cfg(feature = "ui")]
use terminal_ui::{
    cli::{options::Opts, state::State, ui, App},
    debug::{RequestInfo, TrafficInfo, PrintInfo, FipsInfo},
    util,
};

#[cfg(feature = "ui")]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(feature = "ui")]
use tui::{backend::CrosstermBackend, Terminal};

use configuration;
use plugin_registry;
mod client;
mod fips;

use bytes;
use configuration::Configuration;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Server, Response
};
use plugin_registry::ExternalFunctions;
use std::{
    io::stdout,
    panic,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

use clap::Parser;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

enum Event<I> {
    Input(I),
    Tick,
}

type PrintRequest<'a> = Box<dyn Fn(&Request<Body>) -> () + Send + Sync + 'a>;
type PrintResponse<'a> = Box<dyn Fn(&Response<Body>) -> () + Send + Sync + 'a>;
type PrintPlainInfo<'a> = Box<dyn Fn(String) -> () + Send + Sync + 'a>;
type PrintInfoType<'a> = Box<dyn Fn(&FipsInfo) -> () + Send + Sync + 'a>;

pub struct PaintLogsCallbacks<'a> {
    log_incoming_request_to_fips: PrintRequest<'a>,
    log_outgoing_request_to_server: PrintRequest<'a>,
    log_incoming_response_from_server: PrintResponse<'a>,
    log_outgoing_response_to_client: PrintResponse<'a>,
    log_fips_info: PrintInfoType<'a>,
    log_plain: PrintPlainInfo<'a>
}

// spawns the hyper server on a separate thread
fn spawn_backend(state: &Arc<State>, addr: &SocketAddr) -> JoinHandle<hyper::Result<()>> {
    let capture_state = Arc::clone(state);

    let make_svc = make_service_fn(move |_| {
        let inner_state = Arc::clone(&capture_state);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let innermost_state = Arc::clone(&inner_state);

                // clone the state multiple times because the logging callbacks move the state
                // hence we need to have a copy for each callback
                let innermost_state_1= Arc::clone(&inner_state);
                let innermost_state_2= Arc::clone(&inner_state);
                let innermost_state_4 = Arc::clone(&inner_state);
                let innermost_state_3 = Arc::clone(&inner_state);
                let innermost_state_5 = Arc::clone(&inner_state);
                let innermost_state_6 = Arc::clone(&inner_state);

                let logging = PaintLogsCallbacks {
                    log_incoming_request_to_fips: Box::new(move |message: &Request<Body>| {
                        innermost_state_1
                            .add_traffic_info(TrafficInfo::IncomingRequest(RequestInfo::from(
                                message,
                            )))
                            .unwrap_or_default();
                    }),
                    log_outgoing_response_to_client: Box::new(move |message: &Response<Body>| {
                        innermost_state_2
                            .add_traffic_info(TrafficInfo::IncomingResponse(ResponseInfo::from(
                                message,
                            )))
                            .unwrap_or_default();
                    }),
                    log_incoming_response_from_server: Box::new(move |message: &Response<Body>| {
                        innermost_state_3
                            .add_traffic_info(TrafficInfo::OutgoingResponse(ResponseInfo::from(
                                message,
                            )))
                            .unwrap_or_default();
                    }),
                    log_outgoing_request_to_server: Box::new(move |message: &Request<Body>| {
                        innermost_state_4
                            .add_traffic_info(TrafficInfo::OutgoingRequest(RequestInfo::from(
                                message,
                            )))
                            .unwrap_or_default();
                    }),
                    log_fips_info: Box::new(move |message: &FipsInfo| {
                        innermost_state_5.add_message(PrintInfo::FIPS(message.clone())).unwrap_or_default();
                    }),
                    log_plain: Box::new(move |message: String| {
                        innermost_state_6.add_message(PrintInfo::PLAIN(String::from(message))).unwrap_or_default();
                    })
                };

                async move { fips::routes(req, innermost_state, &logging).await }
            }))
        }
    });

    let handle = tokio::spawn(Server::bind(addr).serve(make_svc));
    handle
}

#[cfg(feature = "logging")]
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
        .build("log/fips.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "logging")]
    {
        init_logging()?;
        log::info!("Starting FIPS");
        panic::set_hook({
            Box::new(|e| {
                log::error!("Panic: {}", e);
            })
        });
    }

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
    let _rt_handle = spawn_backend(&app.state, &addr);

    #[cfg(feature = "ui")]
    {
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
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("server is running");
        _rt_handle.await?.unwrap();
    }

    Ok(())
}
