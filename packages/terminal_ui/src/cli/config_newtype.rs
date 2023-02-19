use configuration::Configuration;
use std::sync::Mutex;
use tui::{
    style::{Color, Modifier, Style},
    text::{Spans},
    widgets::{ListItem, List},
};

pub struct ConfigurationNewtype<'a>(pub &'a Mutex<Configuration>);

impl<'a> From<ConfigurationNewtype<'_>> for List<'a> {
    fn from(wrapper: ConfigurationNewtype) -> List<'a> {
        let configuration = wrapper.0.lock().unwrap().clone();
        let items: Vec<ListItem> = configuration
            .rule_collection
            .iter()
            .map(|c| {
                let mut lines: Vec<Spans> = vec![];
                if let Some(rule_name) = c.name.clone() {
                    lines.extend(vec![Spans::from(format!(
                        "name: {} --- path: {}",
                        rule_name, c.path
                    ))]);
                } else {
                    lines.extend(vec![Spans::from(format!("path: {}", c.path.clone()))]);
                }
                let bg = match c.selected {
                    true => Color::Reset,
                    false => Color::Reset,
                };
                let fg = match c.active {
                    true => Color::Blue,
                    false => Color::DarkGray,
                };
                let modifier = match c.selected {
                    true => Modifier::UNDERLINED,
                    false => Modifier::DIM,
                };
                ListItem::new(lines).style(Style::default().fg(fg).bg(bg).add_modifier(modifier))
            })
            .collect();
        List::new(items).style(Style::default())
    }
}
