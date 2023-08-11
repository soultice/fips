use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use gradient_tui_fork::{backend::CrosstermBackend, Terminal};

enum Event<I> {
    Input(I),
    Tick,
}

use crate::{PaintLogsCallbacks, utility::{log::{LoggableType, Loggable}, options::CliOptions}, configuration::{nconfiguration::NConfiguration}, terminal_ui::{debug::{PrintInfo, LoggableNT}, cli::{ui, App}, util, state::State}};
use log::info;
use std::panic;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::runtime::Runtime;

#[derive(Error, Debug)]
pub enum FrontendError {
    #[error("Failed to start frontend due to io error")]
    GenericFrontend(#[from] std::io::Error),
    #[error("Failed to start frontend")]
    BoxedFrontend(#[from] Box<dyn std::error::Error>),
    #[error("Failed to start frontend due to channel error")]
    Input(#[from] std::sync::mpsc::RecvError),
}

pub fn spawn_frontend(_app: Option<App>, runtime: Runtime) -> Result<(), FrontendError> {
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
            Event::Input(event) => util::match_keybinds(event, &mut unwrapped_app)?,
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
    Ok(())
}

pub fn define_log_callbacks(state: Arc<State>) -> PaintLogsCallbacks {
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

pub fn setup(
    configuration: Arc<Mutex<NConfiguration>>,
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

    let app = App::new(true, state.clone(), options );

    let logging = Arc::new(define_log_callbacks(app.state.clone()));
    (Some(state), Some(app), logging)
}
