use crate::cli::App;
use crate::configuration::Configuration;
use crate::{Opts, PrintInfo, State};
use crossterm::event::DisableMouseCapture;
use crossterm::event::KeyCode;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::Stdout;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tui::backend::CrosstermBackend;
use tui::Terminal;

pub fn match_keybinds(
    code: KeyCode,
    app: &mut App,
    state: &Arc<State>,
    opts: &Opts,
) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Esc => {
            app.should_quit = true;
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
        KeyCode::BackTab => app.on_left(),
        KeyCode::Tab => app.on_right(),
        KeyCode::Up => app.on_up(),
        KeyCode::Down => app.on_down(),
        _ => {}
    }
    Ok(())
}
