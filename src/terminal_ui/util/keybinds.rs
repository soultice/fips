use crokey::key;

use crate:: terminal_ui::{cli::App, debug::PrintInfo};
use eyre::Result;

pub async fn match_keybinds(
    code: crokey::crossterm::event::KeyEvent,
    app: &mut App<'_>,
) -> Result<()> {
    log::debug!("Key pressed: {:?}", code);
    match code {
            key!(esc) => {
                app.should_quit = true;
            }
            key!(ctrl-c) => {
                app.should_quit = true;
            },
            key!(ctrl-d) => {
                app.should_quit = true;
            },
            key!(shift-backtab) => app.go_to_previous_tab(),
            key!(tab) => app.go_to_next_tab(),
            key!(r) => {
                app.state
                    .configuration
                    .lock()
                    .await
                    .reload(&app.opts.nconfig)?;
                app.state
                    .add_message(PrintInfo::Plain(String::from(
                        "Config files reloaded",
                    )))
                    .unwrap_or_default();
            }
            key!(c) => {
                *app.state.messages.lock().unwrap() = Vec::new();
                *app.state.traffic_info.lock().unwrap() = Vec::new();
            }
            key!(enter) => {
                if app.tabs.index == 2 {
                    app.state.configuration.lock().await.toggle_rule()
                }
            }
            key!(down) => {
                if app.tabs.index == 2 {
                    app.state.configuration.lock().await.select_next()
                }
            }
            key!(up) => {
                app.state.configuration.lock().await.select_previous()
            }
            _ => {}
    }
    Ok(())
}
