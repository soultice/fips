use super::rule_collection::RuleCollection;
use hyper::{Method};
use rand::Rng;
use regex::RegexSet;
use serde::{Deserialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::{error, fs, io};
use tui::style::{Color, Modifier, Style};
use tui::text::Spans;
use tui::widgets::{List, ListItem};

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Deserialize, Debug, Clone)]
pub struct Configuration {
    pub selected: usize,
    pub rule_collection: Vec<RuleCollection>,
    loaded_paths: Vec<PathBuf>,
}

impl Configuration {
    pub fn new(path_to_config: &PathBuf) -> Configuration {
        let mut rules = Configuration {
            selected: 0,
            rule_collection: Vec::new(),
            loaded_paths: Vec::new(),
        };
        rules.load_from_path(path_to_config).unwrap();
        rules.rule_collection[0].selected = true;
        rules
    }

    pub fn toggle_rule(&mut self) {
        self.rule_collection[self.selected].active = !self.rule_collection[self.selected].active
    }

    pub fn select_prev(&mut self) {
        self.rule_collection[self.selected].selected = false;
        match self.selected {
            0 => self.selected = self.rule_collection.len() - 1,
            _ => self.selected -= 1,
        }
        self.rule_collection[self.selected].selected = true;
    }

    pub fn select_next(&mut self) {
        self.rule_collection[self.selected].selected = false;
        self.selected = (self.selected + 1) % self.rule_collection.len();
        self.rule_collection[self.selected].selected = true;
    }

    pub fn paths(&self) -> Vec<String> {
        self.loaded_paths
            .iter()
            .map(|e| String::from(e.to_str().unwrap()))
            .collect()
    }

    pub fn reload(&mut self) -> io::Result<()> {
        self.rule_collection = Vec::new();
        for path in self.loaded_paths.iter() {
            let f = std::fs::File::open(path).unwrap();
            let d: Vec<RuleCollection> = serde_yaml::from_reader(f).ok().unwrap();
            for rule in d {
                self.rule_collection.push(rule)
            }
        }
        Ok(())
    }

    fn load_from_path(&mut self, path_to_config: &PathBuf) -> Result<()> {
        let abs_path_to_config = std::fs::canonicalize(&path_to_config).unwrap();
        let mut entries: Vec<_> = fs::read_dir(abs_path_to_config)?
            .filter_map(|res| match res {
                Ok(e) if e.path().extension()? == "yaml" => Some(e.path()),
                _ => None,
            })
            .collect();
        entries.sort();
        for path in entries.iter() {
            let f = std::fs::File::open(path).unwrap();
            let d: Vec<RuleCollection> = serde_yaml::from_reader(f)?;
            for rule in d {
                self.rule_collection.push(rule)
            }
            self.loaded_paths.push(path.clone());
        }
        Ok(())
    }

    pub fn active_matching_rules(&mut self, uri: &str, method: &Method) -> Vec<usize> {
        let mut rng = rand::thread_rng();
        let path_regex: Vec<String> = self
            .rule_collection
            .iter()
            .map(|rule| rule.path.to_owned())
            .collect();
        let set = RegexSet::new(&path_regex).unwrap();
        set.matches(uri)
            .into_iter()
            .filter(|i| {
                self.rule_collection[*i].active
                    && rng.gen_range(0.0, 1.0) < self.clone_rule(*i).match_with_prob.unwrap_or(1.0)
                    && self
                        .clone_rule(*i)
                        .match_methods
                        .unwrap_or(vec![
                            "GET".to_owned(),
                            "OPTIONS".to_owned(),
                            "POST".to_owned(),
                            "PUT".to_owned(),
                            "DELETE".to_owned(),
                            "HEAD".to_owned(),
                            "TRACE".to_owned(),
                            "CONNECT".to_owned(),
                            "PATCH".to_owned(),
                        ])
                        .iter()
                        .map(|s| Method::from_str(s).unwrap())
                        .collect::<Vec<Method>>()
                        .contains(method)
            })
            .collect()
    }

    pub fn clone_rule(&mut self, idx: usize) -> RuleCollection {
        self.rule_collection.get_mut(idx).unwrap().clone()
    }
}

impl<'a> From<&Configuration> for List<'a> {
    fn from(configuration: &Configuration) -> List<'a> {
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
                    true => Color::Green,
                    false => Color::Red,
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
