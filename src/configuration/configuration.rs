use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::plugin::ExternalFunctions;
use fake::{faker::name::raw::*, locales::*, Fake};
use hyper::Uri;
use regex::RegexSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};

#[derive(Debug, Display)]
pub enum Mode {
    PROXY,
    MOXY,
    MOCK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub rule_collection: Vec<RuleCollection>,
    loaded_paths: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleCollection {
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

impl Configuration {
    pub fn new(path_to_config: &PathBuf) -> Configuration {
        let mut rules = Configuration {
            rule_collection: Vec::new(),
            loaded_paths: Vec::new(),
        };
        rules.load_from_path(path_to_config).unwrap();
        rules
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

    pub fn matching_rules(&mut self, uri: &Uri) -> Vec<usize> {
        let path_regex: Vec<String> = self
            .rule_collection
            .iter()
            .map(|rule| rule.path.to_owned())
            .collect();
        let set = RegexSet::new(&path_regex).unwrap();
        set.matches(&*uri.to_string()).into_iter().collect()
    }

    pub fn clone_collection(&mut self, idx: usize) -> RuleCollection {
        self.rule_collection.get_mut(idx).unwrap().clone()
    }
}

fn recursive_expand(value: &mut serde_json::Value, plugins: &ExternalFunctions) {
    match value {
        serde_json::Value::String(val) => match val.as_str() {
            "{{Name}}" => {
                *val = Name(EN).fake();
            }
            _ => {
                if plugins.has(val) {
                    let result = plugins.call(&val, &[1.0]).expect("Invocation failed");
                    //*val = result.clone();
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
