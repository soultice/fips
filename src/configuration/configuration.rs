use crate::plugin::ExternalFunctions;
use hyper::Uri;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};
use tui::style::{Color, Style};
use tui::text::Spans;
use tui::widgets::{List, ListItem};

// see https://github.com/serde-rs/serde/issues/1030
fn default_as_true() -> bool {
    true
}

#[derive(Debug, Display)]
pub enum Mode {
    PROXY,
    MOXY,
    MOCK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    fn load_from_path(&mut self, path_to_config: &PathBuf) -> io::Result<()> {
        let abs_path_to_config = std::fs::canonicalize(&path_to_config).unwrap();
        let entries: Vec<_> = fs::read_dir(abs_path_to_config)?
            .filter_map(|res| match res {
                Ok(e) if e.path().extension()? == "yaml" => Some(e.path()),
                _ => None,
            })
            .collect();
        for path in entries.iter() {
            let f = std::fs::File::open(path).unwrap();
            let d: Vec<RuleCollection> = serde_yaml::from_reader(f).ok().unwrap();
            for rule in d {
                self.rule_collection.push(rule)
            }
            self.loaded_paths.push(path.clone());
        }
        Ok(())
    }

    pub fn active_matching_rules(&mut self, uri: &Uri) -> Vec<usize> {
        let path_regex: Vec<String> = self
            .rule_collection
            .iter()
            .map(|rule| rule.path.to_owned())
            .collect();
        let set = RegexSet::new(&path_regex).unwrap();
        set.matches(&*uri.to_string())
            .into_iter()
            .filter(|i| self.rule_collection[*i].active)
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
                let mut lines = vec![Spans::from(c.path.clone())];
                let bg = match c.selected {
                    true => Color::White,
                    false => Color::Reset,
                };
                let default = ListItem::new(lines);
                match c.active {
                    true => default.style(Style::default().fg(Color::Green).bg(bg)),
                    false => default.style(Style::default().fg(Color::Red).bg(bg)),
                }
            })
            .collect();
        let list = List::new(items).style(Style::default());
        list
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleCollection {
    #[serde(skip)]
    pub selected: bool,
    #[serde(default = "default_as_true")]
    pub active: bool,
    pub path: String,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<u16>,
    #[serde(rename = "forwardUri")]
    pub forward_uri: Option<String>,
    #[serde(rename = "forwardHeaders")]
    pub forward_headers: Option<Vec<String>>,
    #[serde(rename = "backwardHeaders")]
    pub backward_headers: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule {
    pub path: String,
    pub item: Value,
}

impl RuleCollection {
    pub fn expand_rule_template(&mut self, plugins: &ExternalFunctions) -> () {
        if let Some(rules) = &mut self.rules {
            for rule in rules {
                recursive_expand(&mut rule.item, plugins);
            }
        }
    }

    pub fn mode(&self) -> Mode {
        let mode: Mode = match (&self.forward_uri, &self.rules) {
            (Some(_), Some(_)) => Mode::MOXY,
            (None, Some(_)) => Mode::MOCK,
            _ => Mode::PROXY,
        };
        mode
    }

    pub fn forward_url(&self, uri: &Uri) -> Uri {
        let mut url_path = String::from("");
        if let Some(forward_url) = &self.forward_uri.clone() {
            url_path.push_str(&forward_url);
        }
        url_path.push_str(&uri.to_string());
        Uri::from_str(&url_path).ok().unwrap()
    }
}

fn recursive_expand(value: &mut serde_json::Value, plugins: &ExternalFunctions) {
    match value {
        serde_json::Value::String(val) => match val.as_str() {
            _ => {
                if plugins.has(val) {
                    let result = plugins.call(&val, &[1.0]).expect("Invocation failed");
                    let try_serialize = serde_json::from_str(&result.clone());
                    if let Ok(i) = try_serialize {
                        *value = i;
                    } else {
                        *val = result.clone();
                    }
                }
            }
        },
        serde_json::Value::Array(val) => {
            for i in val {
                recursive_expand(i, plugins);
            }
        }
        serde_json::Value::Object(val) => {
            for (_, i) in val {
                recursive_expand(i, plugins);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn configurations_load_from_folder() -> Result<(), String> {
        let mut configuration = Configuration::new(&PathBuf::from("./test/configuration_files/"));
        let all_rules_loaded = configuration.rule_collection.len() == 4;
        let first_configuration_is_selected = configuration.selected == 0;
        if all_rules_loaded && first_configuration_is_selected {
            Ok(())
        } else {
            Err(String::from("two plus two does not equal four"))
        }
    }
}
