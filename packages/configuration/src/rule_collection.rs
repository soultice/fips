use super::mode::Mode;
use super::rule::Rule;
use plugin_registry::plugin::ExternalFunctions;
use hyper::Uri;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

// see https://github.com/serde-rs/serde/issues/1030
fn default_as_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleCollection {
    pub name: Option<String>,
    #[serde(rename = "matchProbability")]
    pub match_with_prob: Option<f32>,
    #[serde(rename = "matchMethods")]
    pub match_methods: Option<Vec<String>>,
    #[serde(skip)]
    pub selected: bool,
    pub sleep: Option<u64>,
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
            (Some(_), Some(_)) => Mode::PIMPS,
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
