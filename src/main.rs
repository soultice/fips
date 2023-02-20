use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

use http::Error;
use log::info;
#[cfg(feature = "logging")]
use log::LevelFilter;
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
    debug::{FipsInfo, PrintInfo, RequestInfo, TrafficInfo},
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
    Body, Request, Response, Server,
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
use std::future::Future;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

enum Event<I> {
    Input(I),
    Tick,
}

type PrintRequest = Box<dyn Fn(&Request<Body>) -> () + Send + Sync>;
type PrintResponse = Box<dyn Fn(&Response<Body>) -> () + Send + Sync>;
type PrintPlainInfo = Box<dyn Fn(String) -> () + Send + Sync>;
type PrintInfoType = Box<dyn Fn(&FipsInfo) -> () + Send + Sync>;

pub struct PaintLogsCallbacks {
    log_incoming_request_to_fips: PrintRequest,
    log_outgoing_request_to_server: PrintRequest,
    log_incoming_response_from_server: PrintResponse,
    log_outgoing_response_to_client: PrintResponse,
    log_fips_info: PrintInfoType,
    log_plain: PrintPlainInfo,
}

fn define_log_callbacks(state: Arc<State>) -> PaintLogsCallbacks {
    let innermost_state_1 = Arc::clone(&state);
    let innermost_state_2 = Arc::clone(&state);
    let innermost_state_4 = Arc::clone(&state);
    let innermost_state_3 = Arc::clone(&state);
    let innermost_state_5 = Arc::clone(&state);
    let innermost_state_6 = Arc::clone(&state);
    let innermost_state_7 = Arc::clone(&state);

    PaintLogsCallbacks {
        log_incoming_request_to_fips: Box::new(move |message: &Request<Body>| {
            innermost_state_1
                .add_traffic_info(TrafficInfo::IncomingRequest(RequestInfo::from(message)))
                .unwrap_or_default();
        }),
        log_outgoing_response_to_client: Box::new(move |message: &Response<Body>| {
            innermost_state_2
                .add_traffic_info(TrafficInfo::IncomingResponse(ResponseInfo::from(message)))
                .unwrap_or_default();
        }),
        log_incoming_response_from_server: Box::new(move |message: &Response<Body>| {
            innermost_state_3
                .add_traffic_info(TrafficInfo::OutgoingResponse(ResponseInfo::from(message)))
                .unwrap_or_default();
        }),
        log_outgoing_request_to_server: Box::new(move |message: &Request<Body>| {
            innermost_state_4
                .add_traffic_info(TrafficInfo::OutgoingRequest(RequestInfo::from(message)))
                .unwrap_or_default();
        }),
        log_fips_info: Box::new(move |message: &FipsInfo| {
            innermost_state_5
                .add_message(PrintInfo::FIPS(message.clone()))
                .unwrap_or_default();
        }),
        log_plain: Box::new(move |message: String| {
            innermost_state_6
                .add_message(PrintInfo::PLAIN(String::from(message)))
                .unwrap_or_default();
        }),
    }
}

// spawns the hyper server on a separate thread
fn spawn_backend(
    configuration: &Arc<Mutex<Configuration>>,
    plugins: &Arc<Mutex<ExternalFunctions>>,
    addr: &SocketAddr,
    logger: &Arc<PaintLogsCallbacks>,
) -> JoinHandle<hyper::Result<()>> {
    let capture_plugins = plugins.clone();
    let capture_configuration = configuration.clone();
    let capture_logger = logger.clone();

    let make_svc = make_service_fn(move |_| {
        let inner_plugins = capture_plugins.clone();
        let inner_configuration = capture_configuration.clone();
        let inner_logger = capture_logger.clone();

        let responder = Box::new(move |req: Request<Body>| {
            let innermost_plugins = inner_plugins.clone();
            let innermost_configuration = inner_configuration.clone();
            let innermost_logger = inner_logger.clone();

            async move { fips::routes(req, innermost_configuration, innermost_plugins, &innermost_logger).await }
        });
        let service = service_fn(responder);

        async move { Ok::<_, hyper::Error>(service) }
    });

    tokio::spawn(Server::bind(addr).serve(make_svc))
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

    let plugins = Arc::new(Mutex::new(ExternalFunctions::new(&opts.plugins)));
    let configuration = Arc::new(Mutex::new(
        Configuration::new(&opts.config).unwrap_or(Configuration::default()),
    ));

    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        plugins: plugins.clone(),
        configuration: configuration.clone(),
        traffic_info: Mutex::new(vec![]),
    });

    let mut app = App::new(true, state, opts.clone());

    let logging = &Arc::new(define_log_callbacks(app.state.clone()));
    let addr = ([127, 0, 0, 1], opts.port).into();
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();
    let _rt_handle = spawn_backend(&configuration, &plugins, &addr, logging);

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
