use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

mod client;
mod fips;
use clap::Parser;
use configuration::configuration::Configuration;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use log::info;
use plugin_registry::ExternalFunctions;
use std::net::SocketAddr;
use std::{
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use utility::{
    log::{Loggable, LoggableType},
    options::Opts,
};
use std::panic;

#[cfg(feature = "ui")]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "ui")]
use std::{
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
#[cfg(feature = "ui")]
use terminal_ui::{
    cli::{state::State, ui, App},
    debug::{LoggableNT, PrintInfo},
    util,
};
#[cfg(feature = "ui")]
use tui::{backend::CrosstermBackend, Terminal};

#[cfg(feature = "ui")]
enum Event<I> {
    Input(I),
    Tick,
}

#[cfg(feature = "logging")]
use log::LevelFilter;
#[cfg(feature = "logging")]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

type LogFunction = Box<dyn Fn(&Loggable) + Send + Sync>;

pub struct PaintLogsCallbacks(LogFunction);

#[cfg(not(feature = "ui"))]
fn define_log_callbacks() -> PaintLogsCallbacks {
    let log = Box::new(|message: &Loggable| info!("{:?}", message.message));
    PaintLogsCallbacks(log)
}

#[cfg(feature = "ui")]
fn define_log_callbacks(state: Arc<State>) -> PaintLogsCallbacks {
    let inner_state = Arc::clone(&state);

    let log = Box::new(move |message: &Loggable| {
        match &message.message_type {
            LoggableType::IncomingRequestAtFfips(i) => {
                inner_state
                    .add_traffic_info(LoggableNT(LoggableType::IncomingRequestAtFfips(i.clone())))
                    .unwrap();
            }
            LoggableType::OutGoingResponseFromFips(i) => {
                inner_state
                    .add_traffic_info(LoggableNT(LoggableType::OutGoingResponseFromFips(
                        i.clone(),
                    )))
                    .unwrap();
            }
            LoggableType::OutgoingRequestToServer(i) => {
                inner_state
                    .add_traffic_info(LoggableNT(LoggableType::OutgoingRequestToServer(i.clone())))
                    .unwrap();
            }
            LoggableType::IncomingResponseFromServer(i) => {
                inner_state
                    .add_traffic_info(LoggableNT(LoggableType::IncomingResponseFromServer(
                        i.clone(),
                    )))
                    .unwrap();
            }
            LoggableType::Plain => {
                inner_state
                    .add_message(PrintInfo::PLAIN(message.message.clone()))
                    .unwrap();
            }
        }
        info!("{:?}", message.message)
    });
    PaintLogsCallbacks(log)
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

            async move {
                fips::routes(
                    req,
                    innermost_configuration,
                    innermost_plugins,
                    &innermost_logger,
                )
                .await
            }
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

    let (_state, _app, logging) = {
        #[cfg(feature = "ui")]
        let (state, app, logging) = {
            let state = Arc::new(State {
                messages: Mutex::new(Vec::new()),
                plugins: plugins.clone(),
                configuration: configuration.clone(),
                traffic_info: Mutex::new(vec![]),
            });

            let app = App::new(true, state.clone(), opts.clone());

            let logging = Arc::new(define_log_callbacks(app.state.clone()));
            (Some(state), Some(app), logging)
        };
        #[cfg(not(feature = "ui"))]
        let (state, app, logging) = {
            let logging = Arc::new(define_log_callbacks());
            (
                None::<std::marker::PhantomData<String>>,
                None::<std::marker::PhantomData<String>>,
                logging,
            )
        };
        (state, app, logging)
    };

    let addr = ([127, 0, 0, 1], opts.port).into();
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();

    let _rt_handle = spawn_backend(&configuration, &plugins, &addr, &logging);

    #[cfg(feature = "ui")]
    {
        enable_raw_mode()?;

        let mut unwrapped_app = _app.unwrap();

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
            let captured_state = unwrapped_app.state.clone();
            Box::new(move |panic_info| {
                captured_state
                    .add_message(PrintInfo::PLAIN(panic_info.to_string()))
                    .unwrap_or_default();
            })
        });

        loop {
            terminal.draw(|f| ui::draw(f, &mut unwrapped_app))?;

            match rx.recv()? {
                Event::Input(event) => util::match_keybinds(event.code, &mut unwrapped_app)?,
                Event::Tick => unwrapped_app.on_tick()?,
            };

            if unwrapped_app.should_quit {
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
