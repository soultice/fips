use crate::cli::App;
use crate::configuration::Configuration;
use crate::{Opts, PrintInfo, State};
use crossterm::event::KeyCode;
use std::sync::Arc;

pub fn match_keybinds(
    code: KeyCode,
    app: &mut App,
    opts: &Opts,
) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('r') => {
            *app.state.configuration.lock().unwrap() = Configuration::new(&opts.config);
            app.state
                .messages
                .lock()
                .unwrap()
                .push(PrintInfo::PLAIN(String::from("Config file reloaded")))
        }
        KeyCode::Char('c') => {
            *app.state.messages.lock().unwrap() = Vec::new();
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
