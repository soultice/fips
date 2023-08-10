use gradient_tui_fork::{
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{List, ListItem},
};
use std::sync::Mutex;

use crate::configuration::nconfiguration::NConfiguration;

pub struct ConfigurationNewtype<'a>(pub &'a Mutex<NConfiguration>);

impl<'a> From<ConfigurationNewtype<'_>> for List<'a> {
    fn from(wrapper: ConfigurationNewtype) -> List<'a> {
        let configuration = wrapper.0.lock().unwrap();
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
                let bg = match true {
                    true => Color::Reset,
                    false => Color::Reset,
                };
                let fg = match true {
                    true => Color::Blue,
                    false => Color::DarkGray,
                };
                let modifier = match true {
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
