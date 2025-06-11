use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use gradient_tui_fork::{
    backend::CrosstermBackend, text::Spans, widgets::List, Terminal,
};
use std::{
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

enum Event<I> {
    Input(I),
    Tick,
}

use crate::{
    configuration::configuration::Config,
    terminal_ui::{
        cli::{
            config_newtype::{AsyncFrom, ConfigurationNewtype},
            ui, App,
        },
        debug::{LoggableNT, PrintInfo},
        state::State,
        util,
    },
    utility::{
        log::{Loggable, LoggableType},
        options::CliOptions,
    },
    PaintLogsCallbacks,
};
use eyre::Result;
use crate::configuration::rule::Rule;
use std::panic;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Error, Debug)]
pub enum FrontendError {
    #[error("Failed to start frontend due to io error")]
    GenericFrontend(#[from] std::io::Error),
    #[error("Failed to start frontend due to channel error")]
    Input(#[from] std::sync::mpsc::RecvError),
    #[error("unexpected none option")]
    NoneOption,
}

pub async fn spawn_frontend(
    _app: Option<App<'_>>,
    runtime: Runtime,
) -> Result<()> {
    enable_raw_mode()?;

    let mut unwrapped_app = _app.ok_or_else(|| eyre::eyre!("No app available"))?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(50);

    thread::spawn(move || -> Result<()> {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read()? {
                    tx.send(Event::Input(key))?;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick)?;
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    panic::set_hook({
        let captured_state = unwrapped_app.state.clone();
        Box::new(move |panic_info| {
            log::error!("Panic: {}", panic_info);
            captured_state
                .add_message(PrintInfo::Plain(panic_info.to_string()))
                .unwrap();
        })
    });

    loop {
        let config = unwrapped_app.state.configuration.clone();
        let config_guard = config.lock().await;
        let plugin_names = collect_plugin_info(&config_guard.rules).await;
        let plugin_spans: Vec<Spans> = plugin_names.into_iter()
            .map(Spans::from)
            .collect();

        let wrapper = ConfigurationNewtype(config.clone());
        let list = List::async_from(wrapper).await;
        
        let app_ref = &mut unwrapped_app;
        terminal
            .draw(|f| ui::draw(f, app_ref, plugin_spans, list))?;

        match rx.recv()? {
            Event::Input(event) => {
                util::match_keybinds(event, app_ref).await?;
            }
            Event::Tick => app_ref.on_tick()?,
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
    Ok(())
}

async fn collect_plugin_info(rules: &[Rule]) -> Vec<String> {
    let mut result = Vec::new();
    for rule in rules {
        if let Some(with) = &rule.with {
            result.extend(with.plugins.iter().map(|p| p.name.clone()));
        }
    }
    result
}

pub fn define_log_callbacks(state: Arc<State>) -> PaintLogsCallbacks {
    let inner_state = Arc::clone(&state);

    let log = Box::new(move |message: &Loggable| match &message.message_type {
        LoggableType::IncomingRequestAtFips(i) => {
            inner_state
                .add_traffic_info(LoggableNT(
                    LoggableType::IncomingRequestAtFips(i.clone()),
                ))
                .unwrap();
        }
        LoggableType::OutgoingResponseAtFips(i) => {
            inner_state
                .add_traffic_info(LoggableNT(
                    LoggableType::OutgoingResponseAtFips(i.clone()),
                ))
                .unwrap();
        }
        LoggableType::OutgoingRequestToServer(i) => {
            inner_state
                .add_traffic_info(LoggableNT(
                    LoggableType::OutgoingRequestToServer(i.clone()),
                ))
                .unwrap();
        }
        LoggableType::IncomingResponseFromServer(i) => {
            inner_state
                .add_traffic_info(LoggableNT(
                    LoggableType::IncomingResponseFromServer(i.clone()),
                ))
                .unwrap();
        }
        LoggableType::IncomingResponseAtFips(i) => {
            inner_state
                .add_traffic_info(LoggableNT(
                    LoggableType::IncomingResponseAtFips(i.clone()),
                ))
                .unwrap();
        }
        LoggableType::Plain => {
            inner_state
                .add_message(PrintInfo::Plain(message.message.clone()))
                .unwrap();
        }
    });
    PaintLogsCallbacks(log)
}

pub async fn setup(
    configuration: Arc<AsyncMutex<Config>>,
    options: CliOptions,
) -> (
    Option<Arc<State>>,
    Option<App<'static>>,
    Arc<PaintLogsCallbacks>,
) {
    let state = Arc::new(State {
        messages: Mutex::new(Vec::new()),
        configuration,
        traffic_info: Mutex::new(vec![]),
    });

    let app = App::new(true, state.clone(), options);

    let logging = Arc::new(define_log_callbacks(app.state.clone()));
    (Some(state), Some(app), logging)
}
