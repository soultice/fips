use serde::{Deserialize, Serialize};
use serde_json::Value;

use fake::{faker::name::raw::*, locales::*, Fake};
use hyper::Uri;
use regex::RegexSet;
use std::str::FromStr;

#[derive(Debug, Display)]
pub enum Mode {
    PROXY,
    MOXY,
    MOCK,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Rule {
    pub path: String,
    pub item: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleCollection {
    pub path: String,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<u16>,
    #[serde(rename = "forwardUri")]
    pub forward_uri: Option<String>,
    #[serde(rename = "forwardUri")]
    pub forward_headers: Option<Vec<String>>,
    #[serde(rename = "forwardUri")]
    pub backward_headers: Option<Vec<String>>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub rule_collection: Vec<RuleCollection>,
}

impl RuleCollection {
    pub fn expand_rule_template(&mut self) -> () {
        if let Some(rules) = &mut self.rules {
            for rule in rules {
                recursive_expand(&mut rule.item);
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
    pub fn new() -> Configuration {
        let mut current_path = std::env::current_dir().unwrap();
        current_path.push("config.yaml");
        let f = std::fs::File::open(current_path).unwrap();
        let d: Vec<RuleCollection> = serde_yaml::from_reader(f).ok().unwrap();
        Configuration { rule_collection: d }
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

    pub fn get_rule_collection_mut(&mut self, idx: usize) -> Option<&mut RuleCollection> {
        self.rule_collection.get_mut(idx)
    }
}

fn recursive_expand(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(val) => match val.as_str() {
            "{{Name}}" => {
                *val = Name(EN).fake();
            }
            _ => {}
        },
        serde_json::Value::Array(val) => {
            for i in val {
                recursive_expand(i);
            }
        }
        serde_json::Value::Object(val) => {
            for (_, i) in val {
                recursive_expand(i);
            }
        }
        _ => {}
    }
}
