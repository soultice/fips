use crossterm::event::KeyCode;

use crate::terminal_ui::{cli::App, debug::PrintInfo};

pub fn match_keybinds(code: KeyCode, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('r') => {
            //app.state.configuration.lock().unwrap().reload()?;
            app.state
                .add_message(PrintInfo::PLAIN(String::from("Config files reloaded")))
                .unwrap_or_default();
        }
        KeyCode::Char('c') => {
            *app.state.messages.lock().unwrap() = Vec::new();
            *app.state.traffic_info.lock().unwrap() = Vec::new();
        }
        KeyCode::Char(_c) => {}
        KeyCode::BackTab => app.on_left(),
        KeyCode::Tab => app.on_right(),
        KeyCode::Enter => {
            if app.tabs.index == 2 {
                //TODO app.state.configuration.lock().unwrap().toggle_rule()
            }
        }
        KeyCode::Down => {
            if app.tabs.index == 2 {
                app.state.configuration.lock().unwrap().select_next()
            }
        }
        KeyCode::Up => app.state.configuration.lock().unwrap().select_previous(),
        _ => {}
    }
    Ok(())
}
