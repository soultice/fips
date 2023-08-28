use gradient_tui_fork::{
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{List, ListItem},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::configuration::nconfiguration::NConfiguration;

pub struct ConfigurationNewtype<'a>(pub &'a AsyncMutex<NConfiguration>);
pub trait AsyncFrom<T> {
    type Output;
    async fn async_from(t: T) -> Self::Output;
}

impl<'a> AsyncFrom<ConfigurationNewtype<'_>> for List<'a> {
    type Output = List<'a>;
    async fn async_from(wrapper: ConfigurationNewtype<'_>) -> List<'a> {
        let configuration = wrapper.0.lock().await;
        let items: Vec<ListItem> = configuration
            .rules
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let mut lines: Vec<Spans> = vec![];
                if let crate::configuration::nconfiguration::RuleSet::Rule(r) =
                    c
                {
                    lines.extend(vec![Spans::from(format!(
                        "name: {} --- path: {}",
                        r.name, r.path
                    ))]);
                }
                // can be used to set background color of a rule
                let bg = match true {
                    true => Color::Reset,
                    false => Color::Reset,
                };
                let fg = match configuration.active_rule_indices.contains(&idx) {
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
        List::new(items).style(Style::default())
    }
}
