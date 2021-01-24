use crate::cli::App;
use crate::PrintInfo;
use crossterm::event::KeyCode;

pub fn match_keybinds(code: KeyCode, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('r') => {
            app.state.configuration.lock().unwrap().reload()?;
            app.state
                .messages
                .lock()
                .unwrap()
                .insert(0, PrintInfo::PLAIN(String::from("Config files reloaded")))
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
