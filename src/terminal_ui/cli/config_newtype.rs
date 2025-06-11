use std::sync::Arc;

use gradient_tui_fork::{
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{List, ListItem},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::configuration::configuration::Config;
use crate::configuration::rule::Rule;

pub struct ConfigurationNewtype(pub Arc<AsyncMutex<Config>>);
pub trait AsyncFrom<T> {
    type Output;
    async fn async_from(t: T) -> Self::Output;
}

impl<'a> AsyncFrom<ConfigurationNewtype> for List<'a> {
    type Output = List<'a>;
    async fn async_from(wrapper: ConfigurationNewtype) -> List<'a> {
        let configuration = wrapper.0.lock().await;
        let items: Vec<ListItem> = configuration
            .rules
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let mut lines: Vec<Spans> = vec![];
                lines.extend(vec![Spans::from(format!(
                    "name: {} --- path: {}",
                    r.get_name(), r.get_path()
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
                let modifier = match configuration.fe_selected_rule == Some(idx as i32) {
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

pub struct ConfigNewtype<'a>(&'a Rule);

impl<'a> From<&'a Rule> for ConfigNewtype<'a> {
    fn from(rule: &'a Rule) -> Self {
        ConfigNewtype(rule)
    }
}

impl<'a> ConfigNewtype<'a> {
    pub fn get_name(&self) -> &str {
        &self.0.name
    }
    
    pub fn get_path(&self) -> &str {
        &self.0.path
    }
    
    pub fn as_inner(&self) -> &Rule {
        self.0
    }
}

fn format_rules(rules: &[Rule], selected_rule: Option<i32>) -> Vec<String> {
    rules.iter()
        .enumerate()
        .map(|(idx, r)| {
            let is_selected = selected_rule.map(|s| s as usize == idx).unwrap_or(false);
            format!("{}{} ({})", 
                if is_selected { "> " } else { "  " },
                r.name, 
                r.path
            )
        })
        .collect()
}

pub fn get_rule_entries(configuration: &[Rule], selected_rule: Option<i32>) -> Vec<String> {
    format_rules(configuration, selected_rule)
}
