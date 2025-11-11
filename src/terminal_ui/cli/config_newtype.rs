

use std::sync::Arc;

use gradient_tui_fork::{
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{List, ListItem},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::configuration::configuration::Config;

pub struct ConfigurationNewtype(pub Arc<AsyncMutex<Config>>);
pub trait AsyncFrom<T> {
    type Output;
    async fn async_from(t: T) -> Self::Output;
}

impl<'a> AsyncFrom<ConfigurationNewtype> for List<'a> {
    type Output = List<'a>;
    async fn async_from(wrapper: ConfigurationNewtype) -> List<'a> {
        let configuration = wrapper.0.lock().await;
        let mut items: Vec<ListItem> = configuration
            .rules
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let mut lines: Vec<Spans> = vec![];
                lines.extend(vec![Spans::from(format!(
                    "name: {} --- path: {}",
                    r.into_inner().name, r.into_inner().path
                ))]);
                // can be used to set background color of a rule
                let bg = match true {
                    true => Color::Reset,
                    false => Color::Reset,
                };
                let fg = match configuration.active_rule_indices.contains(&idx)
                {
                    true => Color::Blue,
                    false => Color::DarkGray,
                };
                let modifier = match configuration.fe_selected_rule == idx {
                    true => Modifier::UNDERLINED,
                    false => Modifier::DIM,
                };
                ListItem::new(lines).style(
                    Style::default().fg(fg).bg(bg).add_modifier(modifier),
                )
            })
            .collect();

        #[cfg(feature = "logging")]
        for (path, err) in &configuration.parse_errors {
            let mut lines: Vec<Spans> = vec![];
            lines.push(Spans::from(format!("ERROR parsing file")));
            lines.push(Spans::from(format!("path: {}", path)));
            // truncate error if very long
            let display_err = if err.len() > 120 { format!("{}...", &err[..117]) } else { err.clone() };
            lines.push(Spans::from(format!("reason: {}", display_err)));
            items.push(ListItem::new(lines).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
        }
        List::new(items).style(Style::default())
    }
}
