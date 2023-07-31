use std::sync::Mutex;
use gradient_tui_fork::{
    style::{Color, Modifier, Style},
    text::{Spans},
    widgets::{ListItem, List},
};

use crate::configuration::configuration::Configuration;

pub struct ConfigurationNewtype<'a>(pub &'a Mutex<Configuration>);

impl<'a> From<ConfigurationNewtype<'_>> for List<'a> {
    fn from(wrapper: ConfigurationNewtype) -> List<'a> {
        let configuration = wrapper.0.lock().unwrap().clone();
        let items: Vec<ListItem> = configuration
            .rule_collection
            .iter()
            .map(|c| {
                let mut lines: Vec<Spans> = vec![];
                if let Some(rule_name) = c.get_name().clone() {
                    lines.extend(vec![Spans::from(format!(
                        "name: {} --- path: {}",
                        rule_name, c.get_path()
                    ))]);
                } else {
                    lines.extend(vec![Spans::from(format!("path: {}", c.get_path().clone()))]);
                }
                let bg = match c.get_selected() {
                    true => Color::Reset,
                    false => Color::Reset,
                };
                let fg = match c.get_active() {
                    true => Color::Blue,
                    false => Color::DarkGray,
                };
                let modifier = match c.get_selected() {
                    true => Modifier::UNDERLINED,
                    false => Modifier::DIM,
                };
                ListItem::new(lines).style(Style::default().fg(fg).bg(bg).add_modifier(modifier))
            })
            .collect();
        List::new(items).style(Style::default())
    }
}
